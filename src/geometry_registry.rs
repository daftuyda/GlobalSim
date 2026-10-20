//! Runtime course geometry for `external-geometry` builds.
//! Those builds leave the captured course assets out of the binary (they are
//! most of its size); the host supplies a course's geometry before preparing a
//! race on it, e.g. a WebAssembly page that fetches only the course in use.
use crate::retail_geometry::CourseGeometry;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

fn table() -> &'static Mutex<BTreeMap<i32, String>> {
    static TABLE: OnceLock<Mutex<BTreeMap<i32, String>>> = OnceLock::new();
    TABLE.get_or_init(Default::default)
}

/// Register the captured geometry for `course_id`: the exact contents of
/// `data/course_geometry/<course_id>.json`. Identity is still verified when a
/// race uses it, so mismatched data fails rather than substituting a track.
pub fn register_course_geometry(course_id: i32, json: String) {
    table().lock().unwrap_or_else(|e| e.into_inner()).insert(course_id, json);
}

/// The registered geometry for a course, parsed and verified like the bundled assets.
pub(crate) fn course_geometry(course_id: i32, distance: f32) -> Result<CourseGeometry, String> {
    let table = table().lock().unwrap_or_else(|e| e.into_inner());
    let json = table.get(&course_id).ok_or_else(|| {
        format!("Captured course geometry {course_id} unavailable: not registered in this external-geometry build")
    })?;
    CourseGeometry::from_value(&crate::json::parse(json)?, course_id, distance)
}
