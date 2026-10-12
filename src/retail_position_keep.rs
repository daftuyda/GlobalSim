//! Client-order pace control. Field pacemaker selection is deliberately supplied
//! by the caller; section handovers live in RaceField.
use crate::json::Value;
use crate::retail_physics::bounded_stat;
use crate::retail_random::RandomSource;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    SpeedUp,
    Overtake,
    PaceUp,
    PaceDown,
    Extra,
}

#[derive(Clone)]
pub struct Parameters {
    pub pacemaker_forward_range: f32,
    pub pacemaker_not_forward_count: usize,
    start_section: f32,
    end_section: f32,
    cooldown: f32,
    interval: f32,
    continue_sections: [f32; 2],
    base_distance: f32,
    distance_coefficient: f32,
    gaps: [[f32; 2]; 3],
    middle_gap_coefficient: f32,
    speed_end: [f32; 3],
    overtake_end: [f32; 2],
    wisdom: [[f32; 2]; 3],
    coefficients: [[f32; 2]; 5],
}

impl Parameters {
    pub fn start_section(&self) -> f32 { self.start_section }
    pub fn end_section(&self) -> f32 { self.end_section }
    pub fn from_value(value: &Value) -> Result<Self, String> {
        let n = |suffix: &str| -> Result<f32, String> {
            let key = format!("PositionKeep{suffix}");
            let number = value.get_f64(&key)? as f32;
            if number.is_finite() {
                Ok(number)
            } else {
                Err(format!("Invalid pace parameter {key}"))
            }
        };
        let mut gaps = [[0.; 2]; 3];
        for (i, name) in ["Senko", "Sashi", "Oikomi"].iter().enumerate() {
            gaps[i] = [
                n(&format!("PaseUpDownDistanceDiffMin{name}"))?,
                n(&format!("PaseUpDownDistanceDiffMax{name}"))?,
            ];
        }
        let extra = value.get_array("PositionKeepContinueSectionExArray")?;
        if extra.len() != 1 {
            return Err("Expected one Runaway pace duration".into());
        }
        let extra = extra[0].as_f64().ok_or("Invalid Runaway pace duration")? as f32;
        if !extra.is_finite() || extra <= 0. {
            return Err("Invalid Runaway pace duration".into());
        }
        let result = Self {
            pacemaker_forward_range: value.get_f64("PaseMakerChangeForwardRunningStyleRange")? as f32,
            pacemaker_not_forward_count: value.get_f64("PaseMakerChangeTopHorseNotMostForwardStyleCount")? as usize,
            start_section: n("StartSection")?,
            end_section: n("EndSection")?,
            cooldown: n("CoolDownSec")?,
            interval: n("CheckIntervalSec")?,
            continue_sections: [n("ContinueSection")?, extra],
            base_distance: n("PaseBaseDistance")?,
            distance_coefficient: n("PaseBaseDistanceCoef")?,
            gaps,
            middle_gap_coefficient: n("PaseUpDownDistanceDiffMaxCoef")?,
            speed_end: [
                n("SpeedUpEndDistanceDiff")?,
                n("SpeedUpEndDistanceDiffOne")?,
                n("SpeedUpEndDistanceDiffOonige")?,
            ],
            overtake_end: [
                n("OvertakeEndDistanceDiff")?,
                n("OvertakeEndDistanceDiffOonige")?,
            ],
            wisdom: [
                [n("SpeedUpStartPerVal1")?, n("SpeedUpStartPerVal1Oonige")?],
                [n("OvertakeStartPerVal1")?, n("OvertakeStartPerVal1Oonige")?],
                [n("PaseUpStartPerVal1")?; 2],
            ],
            coefficients: [
                [
                    n("SpeedUpBaseTargetSpeedMultiply")?,
                    n("SpeedUpBaseTargetSpeedMultiplyOonige")?,
                ],
                [
                    n("OvertakeBaseTargetSpeedMultiply")?,
                    n("OvertakeBaseTargetSpeedMultiplyOonige")?,
                ],
                [n("PaseUpBaseTargetSpeedMultiply")?; 2],
                [
                    n("PaseDownBaseTargetSpeedMultiply")?,
                    n("PaseDownBaseTargetSpeedMultiplyPhaseMiddle")?,
                ],
                [n("PaseUpExBaseTargetSpeedMultiply")?; 2],
            ],
        };
        if result.interval < 0. || result.cooldown < 0. || result.continue_sections[0] <= 0. {
            return Err("Invalid pace control clock/duration".into());
        }
        Ok(result)
    }

    pub fn thresholds(&self, style: usize, distance: f32) -> (f32, f32) {
        if !(2..=4).contains(&style) {
            return (0., 0.);
        }
        let factor = (1. + (distance - self.base_distance) * self.distance_coefficient).max(1.);
        let [low, high] = self.gaps[style - 2];
        (if style == 2 { low } else { low * factor }, high * factor)
    }
}

/// Values sampled immediately before this runner's pace update, with ranks
/// from the preceding manager rank pass and live positions for distance gaps.
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub distance: f32,
    pub position: f32,
    pub wisdom: f32,
    pub strategy: usize,
    pub rushing: bool,
    pub rushing_mode: i32,
    pub phase: i32,
    pub rank: usize,
    pub pacemaker_rank: usize,
    pub pacemaker_style: usize,
    pub pacemaker_gap: f32,
    pub pacemaker_is_self: bool,
    pub later_style_ahead: bool,
    pub ordinary_nige_count: usize,
    pub behind_gap: Option<f32>,
    pub has_speed_skill: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub rushing: bool,
    pub style: usize,
    pub mode: Option<Mode>,
    pub cooldown: f32,
    pub interval: f32,
    pub coefficient: f32,
    pub effect_start: f32,
    pub effect_end: f32,
    pub exit_gap: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            rushing: false,
            style: 0,
            mode: None,
            cooldown: 0.,
            interval: 0.,
            coefficient: 1.,
            effect_start: 0.,
            effect_end: 0.,
            exit_gap: 0.,
        }
    }
}

fn base_style(style: usize) -> usize {
    if style == 5 {
        1
    } else {
        style
    }
}

impl State {
    fn clear(&mut self) {
        self.mode = None;
        self.coefficient = 1.;
        self.cooldown = 1.;
    }
    fn choose(&self, c: Context, low: f32, high: f32) -> Option<Mode> {
        if self.style == 1 {
            return Some(if c.rushing {
                Mode::SpeedUp
            } else if c.later_style_ahead {
                Mode::Extra
            } else if c.rank == 1 {
                Mode::SpeedUp
            } else {
                Mode::Overtake
            });
        }
        if c.pacemaker_rank < c.rank && base_style(c.pacemaker_style) > base_style(c.strategy) {
            return Some(Mode::Extra);
        }
        if c.pacemaker_gap < low {
            return if c.pacemaker_is_self {
                None
            } else {
                Some(Mode::PaceDown)
            };
        }
        if c.pacemaker_gap > high {
            Some(Mode::PaceUp)
        } else {
            None
        }
    }

    pub fn update(&mut self, c: Context, dt: f32, p: &Parameters, rng: &mut impl RandomSource) {
        if c.rushing != self.rushing {
            self.rushing = c.rushing;
            self.clear();
            self.cooldown = 0.;
            self.interval = 0.;
        }
        self.style = if self.rushing {
            match c.rushing_mode {
                1 => 3,
                2 => 2,
                _ => 1,
            }
        } else {
            base_style(c.strategy)
        };
        let (low, mut high) = p.thresholds(self.style, c.distance);
        let runaway = usize::from(c.strategy == 5);
        let speed_end = if runaway == 1 {
            p.speed_end[2]
        } else if c.pacemaker_is_self && base_style(c.strategy) != 1 {
            p.speed_end[0]
        } else {
            p.speed_end[usize::from(c.ordinary_nige_count == 1)]
        };
        // v6 (prospectively confirmed): while rushing, having nobody behind does not
        // satisfy the speed-up distance-behind condition (start or end).
        let speed_gap_met = c.behind_gap.map_or(!c.rushing, |gap| gap >= speed_end);
        if let Some(mode) = self.mode {
            let end = c.position >= self.effect_end
                || match mode {
                    Mode::Extra => !c.later_style_ahead,
                    Mode::SpeedUp => c.behind_gap.is_some() && speed_gap_met,
                    Mode::Overtake => {
                        c.rank == 1
                            && c.behind_gap
                                .map_or(true, |gap| gap >= p.overtake_end[runaway])
                    }
                    Mode::PaceUp => c.pacemaker_gap <= self.exit_gap,
                    Mode::PaceDown => {
                        c.pacemaker_is_self || c.has_speed_skill || c.pacemaker_gap >= self.exit_gap
                    }
                };
            if end {
                self.clear();
                self.cooldown = p.cooldown;
            }
            return;
        }
        let section = (c.distance / 24.).floor();
        if !(section * (p.start_section - 1.) <= c.position && c.position < section * p.end_section)
        {
            return;
        }
        // Deliberately sequential: the second clock cannot tick until the
        // first has crossed zero. Negative crossing residue is retained.
        for timer in [&mut self.cooldown, &mut self.interval] {
            if *timer > 0. {
                *timer -= dt;
                if *timer > 0. {
                    return;
                }
            }
        }
        self.interval = p.interval;
        let mode = match self.choose(c, low, high) {
            Some(m) => m,
            None => return,
        };
        if (mode == Mode::PaceDown && c.has_speed_skill) || (mode == Mode::SpeedUp && speed_gap_met)
        {
            return;
        }
        let index = match mode {
            Mode::SpeedUp => 0,
            Mode::Overtake => 1,
            Mode::PaceUp => 2,
            Mode::PaceDown => 3,
            Mode::Extra => 4,
        };
        if index < 3 && !c.rushing {
            let roll = rng.real(0., 100.);
            let logarithm = ((bounded_stat(c.wisdom) * 0.1) as f64).log10() as f32;
            if roll >= logarithm * p.wisdom[index][runaway] {
                return;
            }
        }
        self.mode = Some(mode);
        self.effect_start = c.position;
        self.effect_end = c.position + section * p.continue_sections[runaway];
        if mode == Mode::PaceUp || mode == Mode::PaceDown {
            if mode == Mode::PaceDown && c.phase == 1 {
                high = low + (high - low) * p.middle_gap_coefficient.clamp(0., 1.);
            }
            self.exit_gap = rng.real(low, high);
        }
        let alternate = if mode == Mode::PaceDown {
            usize::from(c.phase == 1)
        } else {
            runaway
        };
        self.coefficient = p.coefficients[index][alternate];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retail_random::RetailRandom;
    #[test]
    fn sequential_field_contexts_match_python_timers_draws_and_modes() {
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_pacing_port.json"))
        .unwrap();
        let p = Parameters::from_value(fixture.get("parameters").unwrap()).unwrap();
        let dt = fixture.get_f64("dt").unwrap() as f32;
        for trace in fixture.get_array("traces").unwrap() {
            let mut rng = RetailRandom::new(trace.get_i64("seed").unwrap() as i32);
            let mut state = State::default();
            for (tick, row) in trace.get_array("rows").unwrap().iter().enumerate() {
                let v = row.get("context").unwrap();
                let n = |key| v.get_f64(key).unwrap() as f32;
                let b = |key| v.get_bool(key).unwrap();
                if tick % 17 == 0 {
                    rng.real(-1., 1.);
                }
                let c = Context {
                    distance: n("distance"),
                    position: n("position"),
                    wisdom: n("wisdom"),
                    strategy: n("strategy") as usize,
                    rushing: b("rushing"),
                    rushing_mode: n("rushing_mode") as i32,
                    phase: n("phase") as i32,
                    rank: n("rank") as usize,
                    pacemaker_rank: n("pacemaker_rank") as usize,
                    pacemaker_style: n("pacemaker_style") as usize,
                    pacemaker_gap: n("pacemaker_gap"),
                    pacemaker_is_self: b("pacemaker_is_self"),
                    later_style_ahead: b("later_style_ahead"),
                    ordinary_nige_count: n("ordinary_nige_count") as usize,
                    behind_gap: v.get("behind_gap").unwrap().as_f64().map(|x| x as f32),
                    has_speed_skill: b("has_speed_skill"),
                };
                state.update(c, dt, &p, &mut rng);
                let floats = [
                    state.cooldown,
                    state.interval,
                    state.coefficient,
                    state.effect_start,
                    state.effect_end,
                    state.exit_gap,
                ];
                for (i, (a, e)) in floats
                    .iter()
                    .zip(row.get_array("bits").unwrap())
                    .enumerate()
                {
                    assert_eq!(
                        a.to_bits(),
                        e.as_i64().unwrap() as u32,
                        "seed {} tick {tick} field {i}",
                        trace.get_i64("seed").unwrap()
                    );
                }
                let mode = match state.mode {
                    None => "",
                    Some(Mode::SpeedUp) => "speed_up",
                    Some(Mode::Overtake) => "overtake",
                    Some(Mode::PaceUp) => "pace_up",
                    Some(Mode::PaceDown) => "pace_down",
                    Some(Mode::Extra) => "pace_up_ex",
                };
                assert_eq!(mode, row.get_str("mode").unwrap(), "tick {tick}");
                assert_eq!(state.rushing, row.get_bool("rushing").unwrap());
                assert_eq!(state.style, row.get_i64("style").unwrap() as usize);
                assert_eq!(
                    rng.draw_count,
                    row.get_i64("draws").unwrap() as usize,
                    "tick {tick}"
                );
            }
        }
    }
}
