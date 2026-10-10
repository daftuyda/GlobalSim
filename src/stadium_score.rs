use std::collections::BTreeMap;

// Minimum cached CurOrder among exactly equal RunningStyle values.
// Inputs are BASE styles: Runaway is Front Runner (1), not extended style 5.
// Do not substitute rushing modes.
pub fn style_leaders(styles: &[i32], ranks: &[usize]) -> Vec<bool> {
    let mut leaders: BTreeMap<i32, usize> = BTreeMap::new();
    for (i, &style) in styles.iter().enumerate() {
        if leaders.get(&style).map_or(true, |&old| ranks[i] < ranks[old]) {
            leaders.insert(style, i);
        }
    }
    let mut result = vec![false; styles.len()];
    for &i in leaders.values() { result[i] = true; }
    result
}

pub struct PositioningScore {
    previous_phase: i32,
    pub phases: Vec<i32>,
}

impl PositioningScore {
    pub fn new() -> Self { Self { previous_phase: 0, phases: Vec::new() } }

    // UpdateScorePositionKeep (0x35ac7a0), before the field rank refresh.
    pub fn update(&mut self, phase: i32, is_style_leader: Option<bool>) {
        if phase == self.previous_phase || is_style_leader.is_none() { return; }
        if is_style_leader == Some(true) { self.phases.push(phase); }
        self.previous_phase = phase;
    }
}

// Read-only HorseScoreCalculator.UpdateScoreTemptation (Global RVA 0x35ac6b0).
// Observe after the runner's update, before replay frames are thinned.
pub struct RushingScore {
    remaining: f32,
    pub count: u32,
}

impl RushingScore {
    pub fn new() -> Self { Self { remaining: 1.0, count: 0 } }

    pub fn update(&mut self, rushing: bool, dt: f64) {
        if rushing {
            self.remaining -= dt as f32;
            if self.remaining <= 0.0 {
                self.remaining = 1.0;
                self.count += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leaders_use_exact_styles_and_cached_ranks() {
        use crate::horse_types::Strategy;
        let styles: Vec<_> = [Strategy::Nige, Strategy::Oonige, Strategy::Senkou,
            Strategy::Senkou, Strategy::Nige].iter().map(|s| s.base_running_style()).collect();
        assert_eq!(style_leaders(&styles, &[4, 3, 2, 1, 5]),
                   vec![false, true, false, true, false]);
    }
    #[test]
    fn positioning_is_checked_only_at_phase_change() {
        let mut score = PositioningScore::new();
        score.update(0, Some(true));
        score.update(1, Some(false));
        score.update(1, Some(true));
        score.update(2, None);
        assert!(score.phases.is_empty());
        score.update(2, Some(true));
        score.update(2, Some(true));
        assert_eq!(score.phases, vec![2]);
    }
    #[test]
    fn countdown_pauses_and_discards_overshoot() {
        let mut score = RushingScore::new();
        for _ in 0..15 { score.update(true, 0.0666); }
        assert_eq!(score.count, 0);
        score.update(false, 10.0);
        score.update(true, 0.0666);
        assert_eq!(score.count, 1);
        for _ in 0..15 { score.update(true, 0.0666); }
        assert_eq!(score.count, 1);
        score.update(true, 0.0666);
        assert_eq!(score.count, 2);
    }
}
