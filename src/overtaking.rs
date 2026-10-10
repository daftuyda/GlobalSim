//! CheckOvertakeTarget, RVA 0x340ffb0. Uses a shared pre-movement snapshot.
pub fn is_target(gap:f64, speed_gap:f64, owner_target:f64, other_target:f64,
                 other_speed:f64, directly_blocking:bool, other_blocked:bool)->bool {
    let gap=gap as f32;let speed_gap=speed_gap as f32;
    if gap < -1.0 {return false;}
    if directly_blocking {return (other_speed as f32)<owner_target as f32;}
    if !(1.0..=20.0).contains(&gap) || speed_gap<=0.0 || gap/speed_gap>15.0 {return false;}
    ((if other_blocked {other_speed} else {other_target}) as f32)<owner_target as f32
}
