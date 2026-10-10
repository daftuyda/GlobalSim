//! Global rushed pacing modes, independently of immutable race-entry strategy.
use crate::horse_types::Strategy;

pub fn select_mode(strategy: Strategy, roll: i32) -> u8 {
    match strategy {
        Strategy::Nige | Strategy::Oonige => 4,
        Strategy::Senkou => 3,
        Strategy::Sasi => if roll < 25 { 2 } else { 3 },
        Strategy::Oikomi => if roll < 10 { 1 } else if roll < 30 { 2 } else { 3 },
    }
}

pub fn pacing_style(original: Strategy, mode: u8) -> Strategy {
    match mode { 1 => Strategy::Sasi, 2 => Strategy::Senkou,
        3 | 4 => Strategy::Nige, _ => original }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracted_mode_lottery_boundaries_and_original_strategy() {
        for roll in 0..100 {
            assert_eq!(select_mode(Strategy::Nige, roll), 4);
            assert_eq!(select_mode(Strategy::Oonige, roll), 4);
            assert_eq!(select_mode(Strategy::Senkou, roll), 3);
            assert_eq!(select_mode(Strategy::Sasi, roll), if roll < 25 {2} else {3});
            assert_eq!(select_mode(Strategy::Oikomi, roll), if roll < 10 {1} else if roll < 30 {2} else {3});
        }
        assert_eq!(pacing_style(Strategy::Oikomi, 2), Strategy::Senkou);
        assert_eq!(pacing_style(Strategy::Oikomi, 0), Strategy::Oikomi);
    }
}
