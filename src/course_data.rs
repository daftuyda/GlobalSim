//! Port of course_data.py.

use std::collections::BTreeMap;
use std::sync::OnceLock;

pub const SURFACE_TURF: i32 = 1;
pub const SURFACE_DIRT: i32 = 2;

#[derive(Clone, Copy, Debug)]
pub struct Corner {
    pub start: f64,
    pub length: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Straight {
    pub start: f64,
    pub end: f64,
    pub front_type: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Slope {
    pub start: f64,
    pub length: f64,
    pub slope: f64,
}

#[derive(Clone, Debug)]
pub struct CourseData {
    pub race_track_id: i32,
    pub distance: f64,
    pub distance_type: i32,
    pub surface: i32,
    pub turn: i32,
    pub course_set_status: Vec<i32>,
    pub corners: Vec<Corner>,
    pub straights: Vec<Straight>,
    pub slopes: Vec<Slope>,
    pub max_lane: f64,
    pub first_move_lane_point: f64,
    pub empty_in_move: bool,
}

pub fn is_sorted_by_start_f(starts: impl Iterator<Item = f64>) -> bool {
    let mut last = -1.0;
    for start in starts {
        if start <= last {
            return false;
        }
        last = start;
    }
    true
}

pub fn phase_start(distance: f64, phase: i32) -> f64 {
    match phase {
        0 => 0.0,
        1 => distance * 1.0 / 6.0,
        2 => distance * 2.0 / 3.0,
        3 => distance * 5.0 / 6.0,
        _ => panic!("invalid phase {phase}"),
    }
}

pub fn phase_end(distance: f64, phase: i32) -> f64 {
    match phase {
        0 => distance * 1.0 / 6.0,
        1 => distance * 2.0 / 3.0,
        2 => distance * 5.0 / 6.0,
        3 => distance,
        _ => panic!("invalid phase {phase}"),
    }
}

/// `speed`/`stamina`/`power`/`guts`/`wisdom` accessed positionally exactly
/// like the Python port's `[0, stats.speed, stats.stamina, ...]` array
/// (index 0 unused, ThresholdStat is 1-indexed in course_set_status).
pub fn course_speed_modifier(course: &CourseData, speed: f64, stamina: f64, power: f64, guts: f64, wisdom: f64) -> f64 {
    // InitCourseSetStatus: average applicable master terms into the float
    // accumulator starting at1. Zero status IDs do not enter the divisor.
    let values=[0.,speed,stamina,power,guts,wisdom];
    let count=course.course_set_status.iter().filter(|&&stat|stat!=0).count();
    if count==0 { return 1.; }
    let mut coefficient=1.0_f32;
    for &stat in &course.course_set_status {
        if stat==0 {continue;}
        let value=values[stat as usize] as f32;
        let term=if value<=300. {0.05_f32} else if value<=600. {0.1_f32}
            else if value<=900. {0.15_f32} else if value<=2000. {0.2_f32} else {0.};
        coefficient+=term/count as f32;
    }
    coefficient as f64
}

// -- data loading -------------------------------------------------------
// Populated by main.rs at startup from data/course_data.json (parsed with
// serde_json), via `init_courses`. Kept separate from this module's pure
// logic so the latter needed no JSON dependency while the linker was
// unavailable during development.

static COURSES: OnceLock<BTreeMap<i32, CourseData>> = OnceLock::new();

pub fn init_courses(courses: BTreeMap<i32, CourseData>) {
    COURSES.set(courses).ok();
}

pub fn get_course(course_id: i32) -> &'static CourseData {
    COURSES
        .get()
        .expect("init_courses must run before get_course")
        .get(&course_id)
        .unwrap_or_else(|| panic!("unknown course id {course_id}"))
}
