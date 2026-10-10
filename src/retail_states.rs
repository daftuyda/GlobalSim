//! Recovered rushing/downhill state machines for the native float32 runtime.
//! State owns no RNG: the caller supplies the race's shared stream in pass order.
use crate::json::Value;
use crate::retail_physics::bounded_stat;
use crate::retail_random::RandomSource;

fn number(value: &Value, key: &str) -> Result<f32, String> {
    let result = value.get_f64(key)? as f32;
    if !result.is_finite() {
        return Err(format!("Nonfinite retail state parameter {key}"));
    }
    Ok(result)
}

#[derive(Clone, Copy)]
pub struct RushingParameters {
    check_time: f32,
    maximum_time: f32,
    end_percent: f32,
    senko_nige: f32,
    sashi_senko: f32,
    sashi_nige: f32,
    oikomi_sashi: f32,
    oikomi_senko: f32,
    oikomi_nige: f32,
}

impl RushingParameters {
    pub fn from_value(value: &Value) -> Result<Self, String> {
        Ok(Self {
            check_time: number(value, "TemptationEndCheckTime")?,
            maximum_time: number(value, "TemptationForceEndTime")?,
            end_percent: number(value, "TemptationEndPer")?,
            senko_nige: number(value, "SenkoToNigePer")?,
            sashi_senko: number(value, "SashiToSenkoPer")?,
            sashi_nige: number(value, "SashiToNigePer")?,
            oikomi_sashi: number(value, "OikomiToSashiPer")?,
            oikomi_senko: number(value, "OikomiToSenkoPer")?,
            oikomi_nige: number(value, "OikomiToNigePer")?,
        })
    }
}

pub struct RushingState {
    parameters: RushingParameters,
    style: usize,
    start_distance: f32,
    pub active: bool,
    pub used: bool,
    pub mode: i32,
    pub check_remaining: f32,
    pub maximum_remaining: f32,
    pub forced_remaining: Option<f32>,
}

impl RushingState {
    pub fn new(parameters: RushingParameters, style: usize, start_distance: f32) -> Self {
        assert!((1..=5).contains(&style));
        Self {
            parameters,
            style,
            start_distance,
            active: false,
            used: false,
            mode: 0,
            check_remaining: 0.,
            maximum_remaining: 0.,
            forced_remaining: None,
        }
    }

    fn choose_mode(&self, rng: &mut impl RandomSource) -> i32 {
        if self.style == 1 || self.style == 5 {
            return 4;
        }
        let roll = rng.integer(0, 100) as f32;
        let p = self.parameters;
        match self.style {
            2 => {
                if roll < p.senko_nige {
                    3
                } else {
                    0
                }
            }
            3 => {
                if roll < p.sashi_senko {
                    2
                } else if roll < p.sashi_senko + p.sashi_nige {
                    3
                } else {
                    0
                }
            }
            4 => {
                if roll < p.oikomi_sashi {
                    1
                } else if roll < p.oikomi_sashi + p.oikomi_senko {
                    2
                } else if roll < p.oikomi_sashi + p.oikomi_senko + p.oikomi_nige {
                    3
                } else {
                    0
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn set_forced_end(&mut self, seconds: f32) {
        if self.active {
            self.forced_remaining = Some(seconds.max(0.));
        }
    }

    /// +1 starts, -1 ends. The starting tick never decrements either clock.
    pub fn step(&mut self, distance: f32, dt: f32, rng: &mut impl RandomSource) -> i32 {
        if !self.active {
            if self.used || distance < self.start_distance {
                return 0;
            }
            self.active = true;
            self.used = true;
            self.mode = self.choose_mode(rng);
            self.check_remaining = self.parameters.check_time;
            self.maximum_remaining = self.parameters.maximum_time;
            return 1;
        }
        let ended;
        if let Some(remaining) = self.forced_remaining {
            let remaining = (remaining - dt).max(0.);
            self.forced_remaining = Some(remaining);
            ended = remaining == 0.;
        } else {
            self.maximum_remaining = (self.maximum_remaining - dt).max(0.);
            let mut natural_end = self.maximum_remaining == 0.;
            if !natural_end {
                self.check_remaining = (self.check_remaining - dt).max(0.);
                if self.check_remaining == 0. {
                    self.check_remaining = self.parameters.check_time;
                    natural_end = (rng.integer(0, 100) as f32) < self.parameters.end_percent;
                }
            }
            ended = natural_end;
        }
        if ended {
            self.active = false;
            self.mode = 0;
            -1
        } else {
            0
        }
    }
}

#[derive(Clone, Copy)]
pub struct DownhillParameters {
    end_percent: f32,
    wisdom_rate: f32,
}
impl DownhillParameters {
    pub fn from_value(value: &Value) -> Result<Self, String> {
        Ok(Self {
            end_percent: number(value, "DownSlopeAddSpeedEndPer")?,
            wisdom_rate: number(value, "DownSlopeAddSpeedStartWizRate")?,
        })
    }
}

#[derive(Default)]
pub struct DownhillState {
    pub active: bool,
    pub remaining: f32,
}
impl DownhillState {
    pub fn update(
        &mut self,
        downhill: bool,
        wisdom: f32,
        dt: f32,
        rng: &mut impl RandomSource,
        parameters: DownhillParameters,
    ) -> i32 {
        let previous = self.active;
        if !downhill {
            if self.active {
                self.active = false;
                self.remaining = 1.;
            }
            return -i32::from(previous);
        }
        if self.remaining > 0. {
            self.remaining -= dt;
            return 0; // Zero crossing is tested on the following tick.
        }
        self.remaining = 1.;
        let roll = rng.real(0., 100.);
        if self.active {
            if roll < parameters.end_percent {
                self.active = false;
            }
        } else {
            self.active = roll < (bounded_stat(wisdom) * parameters.wisdom_rate).min(100.);
        }
        i32::from(self.active) - i32::from(previous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    struct Rolls {
        values: VecDeque<f32>,
        draws: usize,
    }
    impl Rolls {
        fn new(values: &[f32]) -> Self {
            Self {
                values: values.iter().copied().collect(),
                draws: 0,
            }
        }
        fn take(&mut self) -> f32 {
            self.draws += 1;
            self.values.pop_front().expect("unexpected random draw")
        }
    }
    impl RandomSource for Rolls {
        fn integer(&mut self, lower: i32, upper: i32) -> i32 {
            assert_eq!((lower, upper), (0, 100));
            self.take() as i32
        }
        fn real(&mut self, lower: f32, upper: f32) -> f32 {
            assert_eq!((lower, upper), (0., 100.));
            self.take()
        }
    }
    fn parameters() -> (RushingParameters, DownhillParameters) {
        let raw = crate::json::parse(include_str!("../data/global_parameters.json"))
        .unwrap();
        let p = raw.get("parameters").unwrap();
        (
            RushingParameters::from_value(p.get("Temptation").unwrap()).unwrap(),
            DownhillParameters::from_value(p.get("Slope").unwrap()).unwrap(),
        )
    }

    #[test]
    fn rushing_boundaries_consumption_and_single_use() {
        for (style, roll, mode) in [
            (1, 99, 4),
            (5, 99, 4),
            (2, 99, 3),
            (3, 24, 2),
            (3, 25, 3),
            (4, 9, 1),
            (4, 10, 2),
            (4, 29, 2),
            (4, 30, 3),
        ] {
            let mut rng = Rolls::new(&[roll as f32, 54.]);
            let mut state = RushingState::new(parameters().0, style, 100.);
            assert_eq!(state.step(99., 3., &mut rng), 0);
            assert_eq!(rng.draws, 0);
            assert_eq!(state.step(100., 3., &mut rng), 1);
            assert_eq!(state.mode, mode);
            assert_eq!(state.check_remaining, 3.);
            assert_eq!(rng.draws, usize::from(style != 1 && style != 5));
        }
        let mut rng = Rolls::new(&[55., 99., 55.]);
        let mut state = RushingState::new(parameters().0, 1, 100.);
        state.step(100., 0., &mut rng);
        for _ in 0..3 {
            assert_eq!(state.step(101., 3., &mut rng), 0);
        }
        assert_eq!(state.step(101., 3., &mut rng), -1);
        assert_eq!(state.step(101., 3., &mut rng), 0);
        assert_eq!(rng.draws, 3);
    }

    #[test]
    fn rushing_float_tick_and_forced_timer_replacement() {
        let mut rng = Rolls::new(&[54.]);
        let mut state = RushingState::new(parameters().0, 1, 100.);
        state.step(100., 0., &mut rng);
        for _ in 0..45 {
            assert_eq!(state.step(101., 0.0666, &mut rng), 0);
        }
        assert_eq!(rng.draws, 0);
        assert_eq!(state.step(101., 0.0666, &mut rng), -1);
        let mut state = RushingState::new(parameters().0, 1, 100.);
        state.set_forced_end(1.); // An inactive state ignores the effect.
        assert!(state.forced_remaining.is_none());
        state.step(100., 0., &mut rng);
        state.set_forced_end(5.);
        state.step(101., 1., &mut rng);
        state.set_forced_end(2.);
        assert_eq!(state.step(101., 1., &mut rng), 0);
        assert_eq!(state.step(101., 1., &mut rng), -1);
        assert_eq!(rng.draws, 1);
    }

    #[test]
    fn downhill_zero_crossing_and_level_terrain_freeze() {
        let p = parameters().1;
        let mut state = DownhillState::default();
        let mut rng = Rolls::new(&[0., 20., 19.999]);
        assert_eq!(state.update(true, 1000., 0.0666, &mut rng, p), 1);
        state.remaining = 0.01;
        assert_eq!(state.update(true, 1000., 0.0666, &mut rng, p), 0);
        assert!(state.remaining < 0.);
        assert_eq!(rng.draws, 1);
        assert_eq!(state.update(true, 1000., 0.0666, &mut rng, p), 0); // Strict end threshold.
        state.remaining = 0.;
        assert_eq!(state.update(true, 1000., 0.0666, &mut rng, p), -1);
        let remaining = state.remaining;
        for _ in 0..20 {
            state.update(false, 1000., 0.0666, &mut rng, p);
        }
        assert_eq!(state.remaining, remaining);
        assert_eq!(rng.draws, 3);
    }

    #[test]
    fn shared_stream_matches_python_state_transitions_and_float_timers() {
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_physics_port.json"))
        .unwrap();
        let (rush_params, down_params) = parameters();
        for trace in fixture.get_array("state_traces").unwrap() {
            let seed = trace.get_i64("seed").unwrap() as i32;
            let style = trace.get_i64("style").unwrap() as usize;
            let mut rng = crate::retail_random::RetailRandom::new(seed);
            let mut rushing = RushingState::new(rush_params, style, 100.);
            let mut downhill = DownhillState::default();
            let mut actual = Vec::new();
            for tick in 0..300 {
                if tick % 17 == 0 {
                    rng.real(-1., 1.);
                }
                if style % 2 == 0 && (tick == 50 || tick == 65) {
                    rushing.set_forced_end(if tick == 50 { 5. } else { 2. });
                }
                let rush = rushing.step(tick as f32 * 2.5, 0.0666, &mut rng);
                let down =
                    downhill.update((tick / 23) % 3 != 0, 900., 0.0666, &mut rng, down_params);
                if tick % 23 == 0 || rush != 0 || down != 0 || tick == 299 {
                    actual.push(vec![
                        tick as i64,
                        rush as i64,
                        down as i64,
                        rushing.mode as i64,
                        i64::from(rushing.active),
                        i64::from(rushing.used),
                        rushing.check_remaining.to_bits() as i64,
                        rushing.maximum_remaining.to_bits() as i64,
                        rushing
                            .forced_remaining
                            .map(|v| v.to_bits() as i64)
                            .unwrap_or(-1),
                        i64::from(downhill.active),
                        downhill.remaining.to_bits() as i64,
                        rng.draw_count as i64,
                    ]);
                }
            }
            let expected: Vec<Vec<i64>> = trace
                .get_array("observations")
                .unwrap()
                .iter()
                .map(|row| {
                    row.as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_i64().unwrap())
                        .collect()
                })
                .collect();
            assert_eq!(actual, expected, "seed {seed}, style {style}");
        }
    }
}
