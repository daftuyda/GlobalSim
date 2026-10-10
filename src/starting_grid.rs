//! Global client gate layouts: HorseRaceInfo constructor 0x34175fb..0x3417648.
use std::sync::OnceLock;
use crate::json::{self, Value};

pub fn starting_lane(gate: usize, track_id: i32, horse_count: usize) -> f64 {
    lane(gate,track_id,horse_count,false)
}

pub fn retail_starting_lane(gate: usize, track_id: i32, horse_count: usize) -> f64 {
    lane(gate,track_id,horse_count,true)
}

fn lane(gate: usize, track_id: i32, horse_count: usize, retail:bool) -> f64 {
    static GRIDS: OnceLock<Value> = OnceLock::new();
    let grids = GRIDS.get_or_init(|| json::parse(include_str!("../data/starting_grids.json"))
        .expect("bundled starting grid metadata"));
    let (kind, half) = grids.get(&track_id.to_string())
        .map(|g| (g.get_i64("initialLaneType").expect("grid type"),
                  g.get_bool("enableHalfGate").expect("half gate flag") && horse_count <= 9)).unwrap_or((1, false));
    let extra = if (kind == 1 && gate >= 9) || (kind == 4 && !half && gate >= 8) { 0.6 }
        else if kind == 3 && gate >= 14 { 1.86 } else { 0.0 };
    if retail {
        let lane=gate as f32*(1./18.);
        (lane+if extra==0.6 {0.033333335_f32} else if extra==1.86 {0.10333333_f32} else {0.}) as f64
    } else {(gate as f64 + extra) / 18.0}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn track_specific_dividers_and_half_gate() {
        assert_eq!(starting_lane(9,10005,18),9.6/18.0);
        assert_eq!(starting_lane(9,10101,18),9.0/18.0);
        assert_eq!(starting_lane(13,10201,18),13.0/18.0);
        assert_eq!(starting_lane(14,10201,18),15.86/18.0);
        assert_eq!(starting_lane(7,10105,10),7.0/18.0);
        assert_eq!(starting_lane(8,10105,10),8.6/18.0);
        assert_eq!(starting_lane(8,10105,9),8.0/18.0);
    }
}
