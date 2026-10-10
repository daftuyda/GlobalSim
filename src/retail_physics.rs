//! Float32 physics reconstructed from the loaded Global client.
//!
//! Mirrors retail_physics.py's operation order. This is a component of the
//! native runtime port, not a switch to the unfinished full runtime. Keep the
//! normal solver separate until shared RNG, field passes and geometry agree.
use crate::json::Value;

/// RacePhaseCalculator.Init (3605620), using the rounded section length.
pub fn phase_boundaries(distance: f32) -> [f32; 4] {
    let section = distance / 24.;
    [section * 4., section * 16., section * 20., distance]
}

#[cfg(test)]
mod phase_boundary_tests {
    use super::phase_boundaries;
    #[test]
    fn exported_thresholds_and_adjacent_float() {
        for (distance, expected) in [(1000.,833.3333740234375_f64),
            (1600.,1333.333251953125), (2000.,1666.666748046875),
            (3200.,2666.66650390625)] {
            let boundaries = phase_boundaries(distance);
            assert_eq!(boundaries[2] as f64, expected);
            let before = f32::from_bits(boundaries[2].to_bits()-1);
            for (position, phase) in [(before,2), (boundaries[2],3), (distance,4)] {
                assert_eq!(boundaries.iter().filter(|&&b|position>=b).count(), phase);
            }
        }
    }
}

/// The base-speed table uses current distance, independently of cached AI phase.
/// End/Last coefficients are equal in the captured client parameter set.
pub fn target_speed_indices(position: f32, distance: f32) -> (usize, usize) {
    // Exploratory server expression: store section length as f32, then truncate
    // the wider quotient directly to integer without an intermediate f32 store.
    let section = ((position as f64 / (distance / 24.) as f64) as i32).clamp(0, 23) as usize;
    (section, if section < 4 { 0 } else if section < 16 { 1 } else { 2 })
}

/// UpdateAI's float countdown; exact zero gives a whole movement tick.
/// `dt` is zero for startup or a positive tick. None suppresses AI and movement.
/// Official server observations retain a double-precision crossing ratio,
/// unlike the loaded client's float division. Keep the countdown in float.
pub fn advance_start_delay(mut remaining: f32, dt: f32) -> (f32, Option<f64>) {
    // UpdateAI's zero-time guard precedes the countdown, even with no delay.
    if dt == 0. {
        return (remaining, None);
    }
    if remaining > 0. {
        remaining -= dt;
        if remaining > 0. {
            return (remaining, None);
        }
    }
    (0., Some(if remaining < 0. { -(remaining as f64) / dt as f64 } else { 1. }))
}

/// CalcFinishTime: only adjacent simulation ticks may be interpolated.
pub fn finish_time(frame_time: f32, dt: f32, previous: f32, distance: f32, course: f32) -> f32 {
    let span = distance - previous;
    debug_assert!(span > 0.);
    let fraction = ((distance - course) / span).clamp(0., 1.);
    (frame_time as f64 - fraction as f64 * dt as f64) as f32
}

#[derive(Clone)]
pub struct Parameters {
    course: [f32; 3],
    phase_speed: [[f32; 4]; 5],
    phase_accel: [[f32; 4]; 5],
    end_speed_sqrt: f32,
    add_speed: f32,
    minimum: [f32; 3],
    blocking: [f32; 3],
    spurt: [f32; 6],
    slope: [f32; 3],
    target_min: f32,
    accel: [f32; 4],
    hp_initial: f32,
    hp_style: [f32; 5],
    hp_rates: [f32; 8],
    hp_drain: [f32; 5],
}

fn group<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("Missing retail physics group {key}"))
}

fn number(value: &Value, key: &str) -> Result<f32, String> {
    let result = value.get_f64(key)? as f32;
    if !result.is_finite() {
        return Err(format!("Nonfinite retail physics parameter {key}"));
    }
    Ok(result)
}

fn numbers<const N: usize>(value: &Value, keys: [&str; N]) -> Result<[f32; N], String> {
    let mut result = [0.; N];
    for (i, key) in keys.iter().enumerate() {
        result[i] = number(value, key)?;
    }
    Ok(result)
}

fn phases(value: &Value, regular: &str, runaway: &str) -> Result<[[f32; 4]; 5], String> {
    let regular_rows = value.get_array(regular)?;
    let runaway_rows = value.get_array(runaway)?;
    if regular_rows.len() != 4 || runaway_rows.len() != 1 {
        return Err("Retail physics needs four ordinary styles and one Runaway row".into());
    }
    let mut result = [[0.; 4]; 5];
    for (i, row) in regular_rows.iter().chain(runaway_rows).enumerate() {
        result[i] = numbers(row, ["Start", "Middle", "End", "Last"])?;
    }
    Ok(result)
}

impl Parameters {
    pub fn from_value(value: &Value) -> Result<Self, String> {
        let base = group(value, "BasetTargetSpeed")?;
        let speed = group(value, "Speed")?;
        let block = group(value, "Block")?;
        let slope = group(value, "Slope")?;
        let hp = group(value, "Hp")?;
        let result = Self {
            course: numbers(
                value,
                [
                    "raceBaseSpeedBaseSpeed",
                    "raceBaseSpeedBaseDistance",
                    "raceBaseSpeedDistanceRate",
                ],
            )?,
            phase_speed: phases(
                base,
                "PhaseBaseTargetSpeedPerArray",
                "PhaseBaseTargetSpeedPerExArray",
            )?,
            phase_accel: phases(speed, "PhaseAccelCoefArray", "PhaseAccelCoefExArray")?,
            end_speed_sqrt: number(base, "PhaseEndBaseTargetSpeedCoef")?,
            add_speed: number(value, "addSpeedParamCoef")?,
            minimum: numbers(
                speed,
                ["MinSpeedRate", "MinSpeedGutsCoefSqrt", "MinSpeedGutsCoef"],
            )?,
            blocking: numbers(
                block,
                [
                    "FrontBlockDistanceGap",
                    "FrontBlockMaxSpeedRateMin",
                    "FrontBlockMaxSpeedRateMax",
                ],
            )?,
            spurt: numbers(
                value,
                [
                    "lastSpurtBaseTargetSpeedAddCoef",
                    "LastSpurtTargetSpeedCoefSqrt",
                    "LastSpurtTargetSpeedGutsCoef1",
                    "LastSpurtTargetSpeedGutsPow",
                    "LastSpurtTargetSpeedGutsCoef2",
                    "baseTargetSpeedCoef",
                ],
            )?,
            slope: numbers(
                slope,
                [
                    "UpSlopeAddSpeedVal1",
                    "DownSlopeAddSpeedVal1",
                    "DownSlopeAddSpeedVal2",
                ],
            )?,
            target_min: number(speed, "TargetSpeedMin")?,
            accel: numbers(
                value,
                [
                    "AccelPowCoefSqrt",
                    "accelPowCoef",
                    "accelPowCoefUpSlope",
                    "StartAccelAdd",
                ],
            )?,
            hp_initial: number(hp, "HpInitialVal1")?,
            hp_style: numbers(
                hp,
                [
                    "HpMaxCoefNige",
                    "HpMaxCoefSenko",
                    "HpMaxCoefSashi",
                    "HpMaxCoefOikomi",
                    "HpMaxCoefOoNige",
                ],
            )?,
            hp_rates: numbers(
                hp,
                [
                    "HpDecRateBaseNormal",
                    "HpDecRateBasePositionKeepPaseDown",
                    "HpDecRateBaseTemptation",
                    "HpDecRateBaseCompeteTopNige",
                    "HpDecRateBaseCompeteTopOonige",
                    "HpDecRateBaseTemptationAndCompeteTopNige",
                    "HpDecRateBaseTemptationAndCompeteTopOonige",
                    "HpDecRateMultiplyDownSlopeAccelMode",
                ],
            )?,
            hp_drain: numbers(
                hp,
                [
                    "SpeedGapParam1",
                    "SpeedGapParam1Pow",
                    "HpDecBase",
                    "HpGutsCoefSqrt",
                    "HpGutsCoef",
                ],
            )?,
        };
        if result.course[2] <= 0.
            || result.blocking[0] <= 0.
            || result.slope[2] <= 0.
            || result.hp_drain[1] <= 0.
            || result.hp_drain[3] <= 0.
        {
            return Err("Retail physics denominator must be positive".into());
        }
        Ok(result)
    }

    pub fn course_speed(&self, distance: f32) -> f32 {
        self.course[0] - (distance - self.course[1]) / self.course[2]
    }

    pub fn base_target_speed(
        &self,
        speed: f32,
        strategy: usize,
        phase: usize,
        aptitude: f32,
        distance: f32,
    ) -> f32 {
        // v19: the official backend evaluates this expression x87-style: float
        // operands (course speed is a rounded float return value, 0.01 a float
        // literal), double intermediates, one rounding on the store. Dense
        // one-tick official samples: start-dash/at-target first drift removed
        // (see analysis/). The all-float form drifts 1 ulp of speed.
        let mut base = (self.phase_speed[strategy - 1][phase] as f64 * 0.01f32 as f64)
            * self.course_speed(distance) as f64;
        if phase >= 2 {
            // Research hypothesis: float sqrt argument/return and helper return;
            // preserve wider intermediate products inside the helper.
            let end_add = ((root(bounded_stat(speed) * self.end_speed_sqrt) as f64
                * aptitude as f64) * self.add_speed as f64) as f32;
            base += end_add as f64;
        }
        base as f32
    }

    pub fn start_dash_speed(&self, distance: f32) -> f32 {
        self.course_speed(distance) * self.minimum[0]
    }

    pub fn minimum_speed(&self, guts: f32, distance: f32) -> f32 {
        // Research candidate: float argument and return at sqrt helper;
        // retain wider following multiplication and sum until the field store.
        (self.start_dash_speed(distance) as f64
            + root(bounded_stat(guts) * self.minimum[1]) as f64 * self.minimum[2] as f64)
            as f32
    }

    pub fn blocking_speed_cap(&self, minimum: f32, blocker_speed: f32, gap: f32) -> f32 {
        let ratio = (gap / self.blocking[0]).clamp(0., 1.);
        // v20: the rate local is evaluated x87-style (double, rounded once).
        let rate = (self.blocking[1] as f64
            + ratio as f64 * (self.blocking[2] as f64 - self.blocking[1] as f64)) as f32;
        minimum.max(blocker_speed * rate)
    }

    pub fn maximum_spurt_speed(
        &self,
        speed: f32,
        guts: f32,
        strategy: usize,
        aptitude: f32,
        distance: f32,
    ) -> f32 {
        // v20: x87-style statements. The base and guts locals are evaluated
        // at double and rounded once on the store; the speed addition is an
        // unrounded double temporary of the final expression, which is
        // rounded once (dense one-tick official samples, see analysis/).
        let base = (self.base_target_speed(speed, strategy, 2, aptitude, distance) as f64
            + self.course_speed(distance) as f64 * self.spurt[0] as f64) as f32;
        let addition = (root(bounded_stat(speed) * self.spurt[1]) as f64
            * aptitude as f64) * self.add_speed as f64;
        // Float pow result, followed by a wider coefficient product.
        let guts_add = ((bounded_stat(guts) * self.spurt[2]).powf(self.spurt[3]) as f64)
            * self.spurt[4] as f64;
        ((base as f64 * self.spurt[5] as f64 + addition) + guts_add as f64) as f32
    }

    pub fn target_speed(&self, input: TargetSpeed, modifiers: &[f32]) -> f32 {
        self.target_speed_with_late_terms(input, modifiers, 0., 0.)
    }

    pub fn target_speed_with_late_terms(&self, input: TargetSpeed, modifiers: &[f32], spot_struggle: f32, bonus: f32) -> f32 {
        let mut speed = (input.base + input.section) * input.pace + input.force_in;
        speed = apply_additive_modifiers(speed, modifiers.iter().copied());
        if input.moving_lane_skill {
            speed += root(bounded_stat(input.power) * 0.0002);
        }
        // v69: the slope correction is evaluated at double precision and rounded
        // once to a float local before the float addition (doc 13.186).
        let correction = (if input.slope > 0. {
            -(input.slope as f64 * (self.slope[0] as f64 / bounded_stat(input.power) as f64))
        } else if input.slope < 0. && input.downhill {
            self.slope[1] as f64 + input.slope.abs() as f64 / self.slope[2] as f64
        } else {
            0.
        }) as f32;
        speed += correction;
        // v69: the Spot Struggle modifier is summed after the skill modifiers and
        // the lane-move skill term, not in activation order (doc 13.186).
        // v77: and after the slope correction, next to the other server-only
        // (CompeteTop/CompeteFight) terms; 12 Front-runner drift races of the
        // 7 Oct audit diverge on sloped spot-struggle ticks otherwise.
        speed += spot_struggle;
        speed += bonus;
        if !input.hp_remaining {
            speed = input.minimum;
        }
        speed.min(30.).max(self.target_min)
    }

    pub fn initial_hp(&self, stamina: f32, strategy: usize, distance: f32) -> f32 {
        // v20: x87-style (double intermediates, one rounding): official HP
        // quanta knife-edges at the start (11 fixed, 0 broken).
        ((bounded_stat(stamina) as f64 * self.hp_initial as f64) * self.hp_style[strategy - 1] as f64
            + distance as f64) as f32
    }

    pub fn hp_state_rate(&self, state: HpState) -> f32 {
        let index = if state.competing {
            (if state.rushing { 5 } else { 3 }) + usize::from(state.runaway)
        } else if state.rushing {
            2
        } else if state.pace_down {
            1
        } else {
            0
        };
        let rate = self.hp_rates[index];
        if state.downhill {
            rate * self.hp_rates[7]
        } else {
            rate
        }
    }

    pub fn hp_per_second(
        &self,
        speed: f32,
        base_speed: f32,
        ground: f32,
        guts: f32,
        phase: usize,
        rate: f32,
    ) -> f32 {
        let gap = (speed - base_speed) + self.hp_drain[0];
        let mut value = (((((gap * gap) / self.hp_drain[1]) as f64) * self.hp_drain[2] as f64 * rate as f64) * ground as f64) as f32;
        if phase >= 2 {
            // Isolated expression-store hypothesis: preserve the sqrt return,
            // round the quotient plus one together before multiplying consumption.
            let factor = (1.0_f64 + self.hp_drain[4] as f64
                / root(bounded_stat(guts) * self.hp_drain[3]) as f64) as f32;
            value *= factor;
        }
        value
    }

    pub fn acceleration(&self, input: Acceleration, modifiers: &[f32]) -> f32 {
        // v20: the float sqrt is a rounded call result; the product chain is
        // evaluated at double and rounded once. Modifiers and the start-dash
        // addition stay float statements (x87 there is worse).
        let mut value = ((((root(bounded_stat(input.power) * self.accel[0]) as f64
            * self.accel[if input.uphill { 2 } else { 1 }] as f64)
            * input.ground as f64)
            * input.distance as f64)
            * self.phase_accel[input.strategy - 1][input.phase] as f64) as f32;
        value = apply_additive_modifiers(value, modifiers.iter().copied());
        if input.start_dash {
            value += self.accel[3];
        }
        value.max(0.)
    }
}

pub fn bounded_stat(value: f32) -> f32 {
    value.clamp(1., 2000.)
}
fn root(value: f32) -> f32 {
    (value as f64).sqrt() as f32
}
pub fn apply_additive_modifiers(base: f32, modifiers: impl IntoIterator<Item = f32>) -> f32 {
    modifiers
        .into_iter()
        .fold(base, |value, modifier| value + modifier)
}

#[derive(Clone, Copy)]
pub struct TargetSpeed {
    pub base: f32,
    pub section: f32,
    pub pace: f32,
    pub force_in: f32,
    pub power: f32,
    pub moving_lane_skill: bool,
    pub slope: f32,
    pub downhill: bool,
    pub hp_remaining: bool,
    pub minimum: f32,
}

#[derive(Clone, Copy)]
pub struct Acceleration {
    pub power: f32,
    pub strategy: usize,
    pub phase: usize,
    pub ground: f32,
    pub distance: f32,
    pub uphill: bool,
    pub start_dash: bool,
}

#[derive(Clone, Copy, Default)]
pub struct HpState {
    pub rushing: bool,
    pub competing: bool,
    pub runaway: bool,
    pub pace_down: bool,
    pub downhill: bool,
}

#[derive(Default)]
pub struct AdditiveReceiver {
    entries: Vec<(u64, f32)>,
}

impl AdditiveReceiver {
    pub fn values(&self) -> impl Iterator<Item = f32> + '_ {
        self.entries.iter().map(|entry| entry.1)
    }
    pub fn values_except_tokens<'a>(&'a self, tokens: &'a [u64]) -> impl Iterator<Item = f32> + 'a {
        self.entries.iter().filter(move |entry| !tokens.contains(&entry.0)).map(|entry| entry.1)
    }
    pub fn values_except(&self, token: u64) -> impl Iterator<Item = f32> + '_ {
        self.entries.iter().filter(move |entry| entry.0 != token).map(|entry| entry.1)
    }
    pub fn value(&self, token: u64) -> Option<f32> {
        self.entries.iter().find(|entry| entry.0 == token).map(|entry| entry.1)
    }
    pub fn register(&mut self, token: u64, value: f32) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.0 == token) {
            entry.1 = value;
        } else {
            self.entries.push((token, value));
        }
    }
    pub fn remove(&mut self, token: u64) {
        self.entries.retain(|entry| entry.0 != token);
    }
    pub fn apply(&self, base: f32) -> f32 {
        apply_additive_modifiers(base, self.entries.iter().map(|entry| entry.1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_time_suppresses_ai_without_changing_countdown() {
        for remaining in [0., 0.04, -0.02] {
            assert_eq!(advance_start_delay(remaining, 0.), (remaining, None));
        }
    }

    #[test]
    fn gate_countdown_exact_zero_and_repeated_subtraction() {
        let dt = 0.0666_f32;
        assert_eq!(advance_start_delay(dt, dt), (0., Some(1.)));
        assert_eq!(advance_start_delay(0., dt), (0., Some(1.)));
        let below = advance_start_delay(f32::from_bits(dt.to_bits()-1), dt).1.unwrap();
        assert!(below > 0. && below < 0.000001);
        assert!(advance_start_delay(f32::from_bits(dt.to_bits()+1), dt).1.is_none());
        let mut remaining = 0.17_f32;
        for _ in 0..2 {
            let (next, fraction) = advance_start_delay(remaining, dt);
            assert!(fraction.is_none());
            remaining = next;
        }
        let (remaining, fraction) = advance_start_delay(remaining, dt);
        assert_eq!(remaining, 0.);
        assert_eq!(fraction.unwrap().to_bits(), 0x3fdca2fae22e92c9);
        assert_eq!(advance_start_delay(remaining, dt), (0., Some(1.)));
    }

    fn parameters() -> Parameters {
        let data = crate::json::parse(include_str!("../data/global_parameters.json"))
        .unwrap();
        Parameters::from_value(data.get("parameters").unwrap()).unwrap()
    }

    #[test]
    fn dash_threshold_uses_captured_float32_rate() {
        let p = parameters();
        assert_eq!(p.start_dash_speed(1800.).to_bits(), 0x41895c2a);
        assert_eq!(p.start_dash_speed(2000.).to_bits(), 0x41880000);
        assert_ne!(p.start_dash_speed(1800.), (0.85_f64 * 20.2) as f32);
    }

    #[test]
    fn python_port_vectors_match_every_float32_bit() {
        let p = parameters();
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_physics_port.json"))
        .unwrap();
        for (index, vector) in fixture.get_array("vectors").unwrap().iter().enumerate() {
            let input = vector.get("input").unwrap();
            let n = |key| number(input, key).unwrap();
            let strategy = n("strategy") as usize;
            let phase = n("phase") as usize;
            let flags = input.get("flags").unwrap();
            let flag = |key| flags.get_bool(key).unwrap();
            let state = HpState {
                rushing: flag("rushing"),
                competing: flag("competing"),
                runaway: flag("runaway"),
                pace_down: flag("pace_down"),
                downhill: flag("downhill"),
            };
            let t = input.get("target").unwrap();
            let tn = |key| number(t, key).unwrap();
            let modifiers: Vec<f32> = input
                .get_array("modifiers")
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap() as f32)
                .collect();
            let target = TargetSpeed {
                base: tn("base"),
                section: tn("section"),
                pace: tn("pace_coef"),
                force_in: tn("force_in"),
                power: n("power"),
                moving_lane_skill: t.get_bool("moving_lane_skill").unwrap(),
                slope: tn("slope"),
                downhill: state.downhill,
                hp_remaining: t.get_bool("hp_remaining").unwrap(),
                minimum: tn("minimum"),
            };
            let course = p.course_speed(n("distance"));
            let base =
                p.base_target_speed(n("speed"), strategy, phase, n("aptitude"), n("distance"));
            let minimum = p.minimum_speed(n("guts"), n("distance"));
            let rate = p.hp_state_rate(state);
            let acceleration = Acceleration {
                power: n("power"),
                strategy,
                phase,
                ground: n("ground"),
                distance: n("aptitude"),
                uphill: target.slope > 0.,
                start_dash: input.get_bool("start_dash").unwrap(),
            };
            let actual = [
                ("course_speed", course),
                ("base_target_speed", base),
                ("minimum_speed", minimum),
                (
                    "maximum_spurt_speed",
                    p.maximum_spurt_speed(
                        n("speed"),
                        n("guts"),
                        strategy,
                        n("aptitude"),
                        n("distance"),
                    ),
                ),
                ("target_speed", p.target_speed(target, &modifiers)),
                (
                    "initial_hp",
                    p.initial_hp(n("stamina"), strategy, n("distance")),
                ),
                ("hp_state_rate", rate),
                (
                    "hp_per_second",
                    p.hp_per_second(base, course, n("ground"), n("guts"), phase, rate),
                ),
                ("acceleration", p.acceleration(acceleration, &modifiers)),
                (
                    "blocking_speed_cap",
                    p.blocking_speed_cap(minimum, base, n("gap")),
                ),
            ];
            for (key, value) in actual {
                let bits = vector.get("expected_bits").unwrap().get_i64(key).unwrap() as u32;
                assert_eq!(
                    value.to_bits(),
                    bits,
                    "vector {index}, {key}: {value} versus {}",
                    f32::from_bits(bits)
                );
            }
        }
    }

    #[test]
    fn receiver_keeps_order_and_recomputes_after_expiry() {
        let mut receiver = AdditiveReceiver::default();
        receiver.register(1, 0.2);
        receiver.register(2, 0.2);
        receiver.register(3, 0.2);
        receiver.remove(1);
        assert_eq!(
            receiver.apply(20.).to_bits(),
            20.400001525878906f32.to_bits()
        );
        receiver.register(2, 0.3); // Updating the same token preserves its position.
        receiver.register(4, -0.3);
        let expected = ((20.0f32 + 0.3) + 0.2) - 0.3;
        assert_eq!(receiver.apply(20.).to_bits(), expected.to_bits());
        receiver.remove(3);
        receiver.remove(2);
        receiver.remove(4);
        assert_eq!(receiver.apply(20.), 20.);
        assert_eq!(apply_additive_modifiers(1., [16777216., -16777216.]), 0.);
        assert_eq!(apply_additive_modifiers(1., [-16777216., 16777216.]), 1.);
    }

    #[test]
    fn missing_parameters_are_rejected_instead_of_defaulted() {
        assert!(Parameters::from_value(&crate::json::parse("{}").unwrap()).is_err());
    }

    #[test]
    fn finish_interpolation_matches_official_adjacent_crossings() {
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_finish_crossings.json")).unwrap();
        let mut checked = 0;
        for row in fixture.get_array("rows").unwrap() {
            if row.get("adjacent_ticks").and_then(|v|v.as_bool()) != Some(true) {
                continue;
            }
            let n = |key|row.get_f64(key).unwrap() as f32;
            assert_eq!(finish_time(n("frame_time"), n("dt"), n("previous_distance"),
                n("distance"), n("course_distance")).to_bits(), n("expected").to_bits());
            checked += 1;
        }
        assert_eq!(checked, 88);
    }
}
