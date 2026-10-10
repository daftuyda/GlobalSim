//! Observation-backed conserve-power interval accounting.
//!
//! The one-second gain clock starts at the first AI update after gate delay.
//! Due credits describe the elapsed interval and use its previous mode. Current
//! transitions then change cooldown and apply entry reductions. This realizes
//! the successful state-conditioned ledger without sampling future mode state.
//! Evidence: clock-origin prospective plan (seed 161803398) and tick-order-v2
//! ledger. No autonomous parity claim; exact server float/timer code is unknown.

pub(crate) struct ConservePowerState {
    pub(crate) gauge: f64,
    next_gain_at: Option<f64>,
    /// Hypothesis: gains compare a float32 elapsed accumulator, reset to zero
    /// at clock or cooldown start and advanced by each later tick delta, with
    /// float32 thresholds (1 s or 2.5 s, then +1 s per credit). `next_gain_at`
    /// remains trace-only.
    gain_elapsed: f32,
    gain_threshold: Option<f32>,
    gain_clock_running: bool,
    previous_rate: f64,
    was_blocked: bool,
    was_rushing: bool,
    was_spot: bool,
    initial_power: Option<f64>,
    unsupported: bool,
    unsupported_reason: Option<&'static str>,
    cutoff_observed: bool,
    trace: Option<Vec<crate::json::Value>>,
}

impl Default for ConservePowerState {
    fn default() -> Self {
        Self { gauge: 0., next_gain_at: None, gain_elapsed: 0., gain_threshold: None, gain_clock_running: false, previous_rate: 0.,
               was_blocked: false, was_rushing: false, was_spot: false,
               initial_power: None, unsupported: false, unsupported_reason: None,
               cutoff_observed: false, trace: None }
    }
}

impl ConservePowerState {
    pub(crate) fn enable_trace(&mut self) {
        if self.trace.is_none() { self.trace = Some(Vec::new()); }
    }

    fn status(&self) -> &'static str {
        self.unsupported_reason.unwrap_or(if self.initial_power.is_none() {
            "notInitialized"
        } else { "supported" })
    }

    fn observation(&self, now: f64, position: f64, kind: &str) -> crate::json::Value {
        use crate::json::{self, Value};
        json::obj(vec![
            ("kind", Value::String(kind.to_string())),
            ("t", json::num(now)), ("distance", json::num(position)),
            ("gauge", json::num(self.gauge)),
            ("nextGainAt", self.next_gain_at.map(json::num).unwrap_or(Value::Null)),
            ("previousRate", json::num(self.previous_rate)),
            ("status", Value::String(self.status().to_string())),
        ])
    }

    fn record(&mut self, now: f64, position: f64, kind: &str) {
        if self.trace.is_some() {
            let row = self.observation(now, position, kind);
            self.trace.as_mut().unwrap().push(row);
        }
    }

    pub(crate) fn record_release(&mut self, now: f64, position: f64,
                                duration: f64, legacy_gauge: f64, power: f64) {
        if self.trace.is_some() {
            let mut row = self.observation(now, position, "release");
            if let crate::json::Value::Object(map) = &mut row {
                map.insert("intendedDuration".into(), crate::json::num(duration));
                map.insert("legacyGauge".into(), crate::json::num(legacy_gauge));
                map.insert("powerAtRelease".into(), crate::json::num(power));
                map.insert("usedLegacyFallback".into(), crate::json::Value::Bool(!self.supported()));
            }
            self.trace.as_mut().unwrap().push(row);
        }
    }

    pub(crate) fn diagnostics(&self, horse_index: usize, legacy_gauge: f64) -> crate::json::Value {
        use crate::json::{self, Value};
        json::obj(vec![
            ("horseIndex", json::num(horse_index as f64)),
            ("finalStatus", Value::String(self.status().to_string())),
            ("initialPower", self.initial_power.map(json::num).unwrap_or(Value::Null)),
            ("finalGauge", json::num(self.gauge)),
            ("finalLegacyGauge", json::num(legacy_gauge)),
            ("cutoffObserved", Value::Bool(self.cutoff_observed)),
            ("timeline", json::arr(self.trace.clone().unwrap_or_default())),
        ])
    }

    pub(crate) fn supported(&self) -> bool {
        self.initial_power.is_some() && !self.unsupported
    }

    pub(crate) fn update(&mut self, now: f64, dt: f32, position: f64, distance: f64,
                        power: f64, strategy: usize, blocked: bool,
                        pace_down: bool, rushing: bool, spot: bool) {
        if self.unsupported { return; }
        if self.gain_clock_running { self.gain_elapsed += dt; }
        let accumulating = position < (distance / 24.).floor() * 10.;
        if accumulating {
            if power <= 1200. || !(1..=5).contains(&strategy)
                || self.initial_power.is_some_and(|initial| initial != power) {
                self.unsupported_reason = Some(if power <= 1200. {
                    if self.initial_power.is_none() { "initialPowerAtOrBelowThreshold" }
                    else { "powerDroppedToOrBelowThreshold" }
                } else if !(1..=5).contains(&strategy) { "unsupportedStrategy" }
                else { "powerChangedDuringAccumulation" });
                self.unsupported = true;
                self.record(now, position, "unsupported");
                return;
            }
            if self.initial_power.is_none() {
                self.initial_power = Some(power);
                self.next_gain_at = Some(now + 1.);
                self.gain_elapsed = 0.;
                self.gain_threshold = Some(1.);
                self.gain_clock_running = true;
                self.record(now, position, "clockStarted");
            }
        }
        // Credit the completed interval on the first cutoff tick before closing
        // accumulation. Later ticks must not keep adding expired-clock credits.
        if !self.cutoff_observed {
            while self.gain_threshold.is_some_and(|threshold| self.gain_elapsed >= threshold) {
                self.gauge += self.previous_rate;
                self.gain_threshold = self.gain_threshold.map(|threshold| threshold + 1.);
                self.next_gain_at = self.next_gain_at.map(|time| time + 1.);
                self.record(now, position, "gainCredit");
            }
        }
        if rushing && !self.was_rushing {
            self.gauge *= 0.8;
            self.record(now, position, "rushingEntryReduction");
        }
        if spot && !self.was_spot {
            self.gauge *= 0.95;
            self.record(now, position, "spotEntryReduction");
        }
        self.was_rushing = rushing;
        self.was_spot = spot;
        if !accumulating {
            if !self.cutoff_observed {
                self.cutoff_observed = true;
                self.record(now, position, "accumulationCutoff");
            }
            return;
        }
        self.previous_rate = (if pace_down { 6.7 } else { 4.2 })
            * match strategy { 2 => 0.75, 4 => 0.7, _ => 1. };
        if blocked || rushing || spot {
            let was_blocked = self.was_blocked;
            self.was_blocked = true;
            self.next_gain_at = None;
            self.gain_threshold = None;
            if !was_blocked { self.record(now, position, "gainBlocked"); }
        } else if self.was_blocked {
            // Transition time stays current even though the due credit above
            // used the prior state. Shifting this timestamp loses valid gains.
            self.next_gain_at = Some(now + 1.5 + 1.);
            self.gain_elapsed = 0.;
            self.gain_threshold = Some(1.5 + 1.);
            self.was_blocked = false;
            self.record(now, position, "cooldownStarted");
        }
    }

    pub(crate) fn duration(&self, distance: f64, distance_type: i32) -> f64 {
        let coefficient = match distance_type {
            1 => 0.45, 2 => 1., 3 => 0.875, 4 => 0.8, _ => return 0.,
        };
        // Existing effect timers handle duration quantization once.
        (self.gauge * distance.sqrt() * coefficient / 1450.) as f32 as f64
    }
}
