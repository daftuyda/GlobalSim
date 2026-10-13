//! CheckOvertakeTarget, RVA 0x340ffb0. Uses a shared pre-movement snapshot.
pub fn is_target(gap:f64, speed_gap:f64, owner_target:f64, other_target:f64,
                 other_speed:f64, directly_blocking:bool, other_blocked:bool)->bool {
    is_target_with_block_check(gap, speed_gap, owner_target, other_target,
        other_speed, directly_blocking, || other_blocked)
}

// Blocking only matters after the gap, relative-speed and direct-blocker checks.
// Let callers defer the field scan until this decision actually needs it.
pub(crate) fn is_target_with_block_check(
    gap: f64, speed_gap: f64, owner_target: f64, other_target: f64,
    other_speed: f64, directly_blocking: bool, other_blocked: impl FnOnce() -> bool,
) -> bool {
    let gap=gap as f32;let speed_gap=speed_gap as f32;
    if gap < -1.0 {return false;}
    if directly_blocking {return (other_speed as f32)<owner_target as f32;}
    if !(1.0..=20.0).contains(&gap) || speed_gap<=0.0 || gap/speed_gap>15.0 {return false;}
    ((if other_blocked() {other_speed} else {other_target}) as f32)<owner_target as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irrelevant_blocker_queries_are_skipped() {
        for (gap, speed_gap, direct) in [(-2., 1., false), (21., 1., false),
            (10., 0., false), (20., 1., false), (1., 1., true)] {
            let actual = is_target_with_block_check(gap, speed_gap, 22., 24., 20., direct,
                || panic!("irrelevant blocker query"));
            assert_eq!(actual, is_target(gap, speed_gap, 22., 24., 20., direct, false));
        }
    }

    #[test]
    fn relevant_blocker_state_selects_the_same_speed() {
        for blocked in [false, true] {
            let calls = std::cell::Cell::new(0);
            let result = is_target_with_block_check(10., 1., 22., 24., 20., false, || {
                calls.set(calls.get() + 1);
                blocked
            });
            assert_eq!(calls.get(), 1);
            assert_eq!(result, blocked);
        }
    }
}
