//! Captured course curves and float32 world-distance correction.
//! Input is course animation data, never observed runner trajectories.
//! Used only by the opt-in runtime; unsupported courses fail explicitly.
use crate::json::Value;
use std::path::Path;

type Quaternion = [f32; 4];

fn finite(value: &Value) -> Result<f32, String> {
    let number = value.as_f64().ok_or("Expected course geometry number")? as f32;
    if number.is_finite() {
        Ok(number)
    } else {
        Err("Nonfinite course geometry value".into())
    }
}

fn number(value: &Value, key: &str) -> Result<f32, String> {
    finite(
        value
            .get(key)
            .ok_or_else(|| format!("Missing course geometry {key}"))?,
    )
}

fn quaternion(value: &Value) -> Result<Quaternion, String> {
    let rows = value.as_array().ok_or("Expected course quaternion")?;
    if rows.len() != 4 {
        return Err("Course quaternion needs four components".into());
    }
    let mut result = [0.; 4];
    for (out, v) in result.iter_mut().zip(rows) {
        *out = finite(v)?;
    }
    if dot(result, result) <= 0. {
        return Err("Invalid zero course quaternion".into());
    }
    Ok(result)
}

/// RaceManager._InitCourseDistance2World (0x35e9b20): the 3D distance between
/// CalcWorldTransform(0 m, lane 0) and CalcWorldTransform(1 m, lane 0).
/// v12: evaluated with the rounding points the official backend shows in the
/// per-tick world-distance correction (v11): float sample and float lerped
/// coordinates (as position()), float differences, squares and sum in
/// double, sqrt rounded to float. The client's all-float value (what the
/// captures/extraction store) differs by one ulp on 10611 (+1), 10808 (-1)
/// and 10906 (-1) among archived courses; on each of those the per-course
/// optimum over dense one-tick official samples is exactly this value.
fn backend_distance_to_world(length: f32, x: &[f32], y: &[f32], z: &[f32]) -> f32 {
    let at = |d: f32| {
        let sample = ((d as f64 / length as f64) * 1000.) as f32;
        let i = (sample as usize).min(1000);
        let j = (i + 1).min(1000);
        let t = sample - i as f32;
        let l = |a: &[f32]| (a[i] as f64 + (a[j] - a[i]) as f64 * t as f64) as f32;
        [l(x), l(y), l(z)]
    };
    let (a, b) = (at(0.), at(1.));
    let (dx, dy, dz) = ((a[0] - b[0]) as f64, (a[1] - b[1]) as f64, (a[2] - b[2]) as f64);
    ((dy * dy + dx * dx) + dz * dz).sqrt() as f32
}
#[cfg(test)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn dot(a: Quaternion, b: Quaternion) -> f32 {
    ((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]) + a[3] * b[3]
}
#[cfg(test)]
fn quaternion_lerp(a: Quaternion, mut b: Quaternion, t: f32) -> Quaternion {
    if dot(a, b) < 0. {
        for v in &mut b {
            *v = -*v;
        }
    }
    let mut q = [0.; 4];
    for i in 0..4 {
        q[i] = lerp(a[i], b[i], t);
    }
    let norm = (dot(q, q) as f64).sqrt() as f32;
    for v in &mut q {
        *v /= norm;
    }
    q
}
fn quaternion_multiply(a: Quaternion, b: Quaternion) -> Quaternion {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        ((aw * bx + ax * bw) + ay * bz) - az * by,
        ((aw * by + ay * bw) + az * bx) - ax * bz,
        ((aw * bz + az * bw) + ax * by) - ay * bx,
        ((aw * bw - ax * bx) - ay * by) - az * bz,
    ]
}

// Source samples are immutable. Compute their fixed subtraction and short-arc
// operands once; interpolation keeps the original float/double rounding order.
#[derive(Clone, Debug)]
struct AnimationSegment {
    x: f64,
    x_delta: f64,
    z: f64,
    z_delta: f64,
    rotation: Quaternion,
    rotation_delta: Quaternion,
}

impl AnimationSegment {
    fn interpolate(&self, t: f32, endpoint: bool) -> (f32, f32, Quaternion) {
        let x = (self.x + self.x_delta * t as f64) as f32;
        let z = (self.z + self.z_delta * t as f64) as f32;
        let mut q = self.rotation;
        if !endpoint {
            for i in 0..4 {
                q[i] = self.rotation[i] + self.rotation_delta[i] * t;
            }
            let norm = (dot(q, q) as f64).sqrt() as f32;
            for v in &mut q { *v /= norm; }
        }
        (x, z, q)
    }
}

fn animation_segments(xs: &[f32], zs: &[f32], rotation: &[Quaternion]) -> Vec<AnimationSegment> {
    (0..xs.len()).map(|i| {
        let j = (i + 1).min(xs.len() - 1);
        let a = rotation[i];
        let mut b = rotation[j];
        if dot(a, b) < 0. {
            for v in &mut b { *v = -*v; }
        }
        AnimationSegment {
            x: xs[i] as f64,
            x_delta: xs[j] as f64 - xs[i] as f64,
            z: zs[i] as f64,
            z_delta: zs[j] as f64 - zs[i] as f64,
            rotation: a,
            rotation_delta: std::array::from_fn(|k| b[k] - a[k]),
        }
    }).collect()
}

#[derive(Clone, Debug)]
struct OverrunAnimation {
    distance: f32,
    segments: Vec<AnimationSegment>,
}

impl OverrunAnimation {
    fn from_value(value: &Value) -> Result<Self, String> {
        let distance = number(value, "distance")?;
        let x = value.get_array("x")?.iter().map(finite).collect::<Result<Vec<_>, _>>()?;
        let z = value.get_array("z")?.iter().map(finite).collect::<Result<Vec<_>, _>>()?;
        let rotation = value.get_array("rotation")?.iter().map(quaternion).collect::<Result<Vec<_>, _>>()?;
        if distance <= 0. || x.len() != 1001 || z.len() != 1001 || rotation.len() != 1001 {
            return Err("Invalid overrun animation".into());
        }
        Ok(Self { distance, segments: animation_segments(&x, &z, &rotation) })
    }
}

#[derive(Clone, Debug)]
pub struct CourseGeometry {
    pub course_id: i32,
    pub distance: f32,
    distance_to_world: f32,
    segments: Vec<AnimationSegment>,
    overrun: Option<OverrunAnimation>,
    rotation_fix: Quaternion,
    initial_lane_max: f32,
    lane_max_changes: Vec<(f32, f32)>,
    corner_events: Vec<(f32, f32, i32, bool)>,
    pub final_corner_half: Option<(f32, f32)>,
    pub first_move_lane_point: f32,
    pub empty_in_move: bool,
}

impl CourseGeometry {
    /// The native worker is relocatable; captured geometry travels with it.
    /// Unknown courses still fail rather than substituting another track.
    pub fn bundled(course_id:i32,distance:f32)->Result<Self,String> {
        let data=include!("retail_geometry_assets.rs");
        Self::from_value(&crate::json::parse(data)?,course_id,distance)
    }

    pub fn load(directory: &Path, course_id: i32, distance: f32) -> Result<Self, String> {
        let text = std::fs::read_to_string(directory.join(format!("{course_id}.json")))
            .map_err(|e| format!("Captured course geometry {course_id} unavailable: {e}"))?;
        Self::from_value(&crate::json::parse(&text)?, course_id, distance)
    }

    pub fn from_value(data: &Value, course_id: i32, distance: f32) -> Result<Self, String> {
        let animation = data.get("animation").ok_or("Missing course animation")?;
        let identity = data
            .get("course_identity")
            .ok_or("Missing verified course identity")?;
        // Check exact numeric identity before a narrowing cast. Equal lengths
        // alone do not identify a course, and fractional IDs are not valid.
        let source = data.get_str("course_id_verified_by")?;
        if course_id <= 0
            || !distance.is_finite()
            || distance <= 0.
            || data.get_f64("course_id")? != course_id as f64
            || identity.get_f64("id")? != course_id as f64
            || identity.get_f64("distance")? != distance as f64
            || animation.get_f64("distance")? != distance as f64
            || !matches!(source, "RaceManager.RaceInfo.RaceCourseSet.Id" | "race_course_set.id + ResourcePath.GetLanePath")
        {
            return Err(
                "Captured course geometry lacks matching verified client course identity".into(),
            );
        }
        if source == "race_course_set.id + ResourcePath.GetLanePath" {
            let assets = data.get("source_assets").ok_or("Missing installed asset provenance")?;
            let track = identity.get_i64("race_track_id")?;
            let ground = identity.get_i64("ground")?;
            let io = identity.get_i64("inout")?;
            let expected = format!("race/course/{track}/pos/an_pos_race{track}_00_{:04}_{:02}_{}_0", distance as i32, ground-1, io-1);
            let event = format!("race/courseeventparam/{course_id}/pfb_prm_race{course_id}");
            if assets.get_str("animation")? != expected || assets.get_str("events")? != event {
                return Err("Installed course asset path does not match master identity".into());
            }
            for key in ["animation_sha256", "events_sha256", "master_sha256"] {
                let digest = assets.get_str(key)?;
                if digest.len()!=64 || !digest.bytes().all(|c|c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
                    return Err("Missing installed course asset provenance".into());
                }
            }
            if data.get("overrun").is_some() {
                let expected = format!("race/course/{track}/pos/an_pos_race{track}_01_0000_{:02}_0_0", ground-1);
                let digest = assets.get_str("overrun_sha256")?;
                if assets.get_str("overrun")? != expected || digest.len()!=64
                    || !digest.bytes().all(|c|c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
                    return Err("Invalid installed overrun provenance".into());
                }
            }
            let expected_fix = if matches!(identity.get_i64("turn")?,1|3) {
                [0.,1.,0.,-4.371138828673793e-08]
            } else {[0.,0.,0.,1.]};
            if quaternion(data.get("rotation_fix").ok_or("Missing installed rotation")?)? != expected_fix {
                return Err("Installed course rotation does not match master identity".into());
            }
        }
        let x = animation
            .get_array("x")?
            .iter()
            .map(finite)
            .collect::<Result<Vec<_>, _>>()?;
        let z = animation
            .get_array("z")?
            .iter()
            .map(finite)
            .collect::<Result<Vec<_>, _>>()?;
        let rotation = animation
            .get_array("rotation")?
            .iter()
            .map(quaternion)
            .collect::<Result<Vec<_>, _>>()?;
        if x.len() != 1001 || z.len() != x.len() || rotation.len() != x.len() {
            return Err("Expected 1001 aligned course animation samples".into());
        }
        // The captured/extracted ratio is still required and validated, but the
        // runtime value is re-derived from the same animation with the official
        // backend's rounding points (see backend_distance_to_world). v12.
        if number(data, "distance_to_world")? <= 0. {
            return Err("Invalid distance-to-world ratio".into());
        }
        let y = animation
            .get_array("y")?
            .iter()
            .map(finite)
            .collect::<Result<Vec<_>, _>>()?;
        if y.len() != x.len() {
            return Err("Expected 1001 aligned course animation samples".into());
        }
        let distance_to_world = backend_distance_to_world(distance, &x, &y, &z);
        if !(distance_to_world > 0.) || !distance_to_world.is_finite() {
            return Err("Invalid distance-to-world ratio".into());
        }
        let rotation_fix = match data.get("rotation_fix") {
            Some(value) => quaternion(value)?,
            None => [0., 0., 0., 1.],
        };
        // v69: the integer master width becomes a lane maximum as the float
        // product width * 0.0001f, the game's basis-point idiom (doc 13.186).
        let width = identity.get_f64("lane_max")?;
        if !width.is_finite() || width <= 0. || width > 100000. || width.fract() != 0. {
            return Err("Invalid initial course lane width".into());
        }
        let mut lane_max_changes = Vec::new();
        let mut corner_events = Vec::new();
        let mut final_corner_half = None;
        let mut first_corner: Option<f32> = None;
        let mut first_override: Option<(f32, Option<bool>)> = None;
        if let Some(events) = data.get("course_event_diagnostics") {
            for event in events.get_array("rows")? {
                let kind = event.get_f64("kind")?;
                let boundary = number(event, "distance")?;
                let values = event.get_array("values_i32")?;
                if kind == 0. && !values.is_empty() {
                    let id = values[0].as_i64().ok_or("Invalid corner ID")? as i32;
                    let length = if values.len() > 1 { finite(&values[1])? } else { 200. };
                    let end = boundary + length;
                    if values.len() >= 2 && id >= 4 && distance - boundary <= 1000. {
                        final_corner_half = Some((boundary + (end - boundary) * 0.5, end));
                    }
                    corner_events.push((boundary, end, id, id >= 4 && distance - end <= 1000.));
                }
                if kind == 0. && first_corner.map_or(true, |previous|boundary < previous) {
                    first_corner = Some(boundary);
                }
                if kind == 7. && first_override.map_or(true, |(previous,_)|boundary < previous) {
                    first_override = Some((boundary, if values.len()==1 {Some(values[0].as_i64()==Some(1))} else {None}));
                }
                if kind != 3. {
                    continue;
                }
                if boundary < 0. || boundary > distance || values.len() != 1 {
                    return Err("Invalid captured lane-width event".into());
                }
                let limit = values[0]
                    .as_f64()
                    .ok_or("Invalid captured lane-width value")?;
                if !limit.is_finite() || limit <= 0. || limit > 100000. || limit.fract() != 0. {
                    return Err("Invalid captured lane-width value".into());
                }
                lane_max_changes.push((boundary, limit as f32 * 0.0001_f32));
            }
        }
        // Stable here: event ordering is not the runner comparison sort.
        corner_events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        lane_max_changes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let (first_move_lane_point, empty_in_move) = match first_override {
            Some((point,Some(inward))) => (point,point>0. && inward),
            _ => {let point=first_corner.unwrap_or(0.).max(0.); (point,point>0.)},
        };
        Ok(Self {
            course_id,
            distance,
            distance_to_world,
            segments: animation_segments(&x, &z, &rotation),
            overrun: data.get("overrun").map(OverrunAnimation::from_value).transpose()?,
            rotation_fix,
            initial_lane_max: width as f32 * 0.0001_f32,
            lane_max_changes,
            corner_events,
            final_corner_half,
            first_move_lane_point,
            empty_in_move,
        })
    }

    pub fn corner_state_at(&self, distance: f32, ready: bool) -> (i32, bool) {
        if ready {
            for &(start, end, number, final_corner) in self.corner_events.iter().rev() {
                if distance >= start {
                    // v76: the server leaves a corner at its end distance (distance >= end), unlike
                    // the client's _UpdateCornerEnd (end >= distance keeps it). Official races
                    // expansion-01-0274 and pacemaker-dirt-01-0018 cast corner==0 skills on the
                    // tick a runner stands exactly on the end (1099.0 m on 10611).
                    return (if distance < end { number } else { 0 }, final_corner);
                }
            }
        }
        (0, false)
    }

    pub fn lane_max_at(&self, distance: f32) -> f32 {
        let mut value = self.initial_lane_max;
        for &(boundary, limit) in &self.lane_max_changes {
            if distance < boundary {
                break;
            }
            value = limit;
        }
        value
    }

    pub fn position(&self, distance: f32, lane: f32) -> Result<[f32; 2], String> {
        if !distance.is_finite() || distance < 0. || !lane.is_finite() {
            return Err("Course geometry requires finite nonnegative distance and finite lane".into());
        }
        let (distance, length, segments) = if distance > self.distance {
            let a = self.overrun.as_ref().ok_or("Course geometry lacks an overrun animation")?;
            ((distance-self.distance) % a.distance, a.distance, &a.segments)
        } else { (distance, self.distance, &self.segments) };
        let sample = ((distance as f64 / length as f64) * 1000.) as f32;
        let i = (sample as usize).min(1000);
        let t = sample - i as f32;
        // The last sample retains its raw rotation, without normalization.
        // Coordinate interpolation still runs, including its signed-zero rules.
        let (x, z, q) = segments[i].interpolate(t, i == 1000);
        let [qx, qy, qz, qw] = quaternion_multiply(q, self.rotation_fix);
        // v12: Quaternion * Vector3(lane * 11.25, 0, 0) with the backend's
        // rounding points: the float locals (num2..num11 = q * 2q products)
        // are rounded, the component expression is evaluated once at double
        // and rounded on the Vector3 store. Dense one-tick official samples
        // with the correction active: 13,175 -> 13,217 of 13,249 (clean,
        // pre-finish); all-float (v11) and all-double forms are worse.
        let num2 = qy * 2.;
        let num3 = qz * 2.;
        let (num5, num6, num8, num11) = (qy * num2, qz * num3, qx * num3, qw * num2);
        let offset = (lane * 11.25) as f64;
        let dx = ((1. - (num5 as f64 + num6 as f64)) * offset) as f32;
        let dz = ((num8 as f64 - num11 as f64) * offset) as f32;
        Ok([x + dx, z + dz])
    }

    pub fn advance(
        &self,
        distance: f32,
        lane: f32,
        mut movement: f32,
        previous_world: [f32; 2],
    ) -> Result<(f32, [f32; 2]), String> {
        if !movement.is_finite() || movement < 0. || previous_world.iter().any(|x| !x.is_finite()) {
            return Err("Invalid course geometry movement".into());
        }
        let current = self.position(distance, lane)?;
        let dx = current[0] - previous_world[0];
        let dz = current[1] - previous_world[1];
        // v11: the client's float32 X/Z differences and allowed movement are
        // kept, but the squares/sum, and the ratio and its product, are
        // evaluated at double precision and rounded to float once (norm after
        // sqrt, corrected movement after the multiply). Selected on 19,047
        // dense single-tick official samples where the correction is active
        // (engine-exact input): 18,487 -> 18,580 exact; see analysis/.
        let world_distance = ((dx as f64 * dx as f64 + dz as f64 * dz as f64).sqrt()) as f32;
        let allowed = movement * self.distance_to_world;
        if world_distance > allowed {
            movement = (movement as f64 * (allowed as f64 / world_distance as f64)) as f32;
        }
        Ok((distance + movement, current))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn directory() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("data/course_geometry")
    }
    fn data() -> Value {
        crate::json::parse(&std::fs::read_to_string(directory().join("10504.json")).unwrap())
            .unwrap()
    }
    fn object(value: &mut Value) -> &mut std::collections::BTreeMap<String, Value> {
        if let Value::Object(v) = value {
            v
        } else {
            panic!("Expected object")
        }
    }
    #[test]
    fn captured_gates_and_per_runner_width_changes() {
        let tokyo = CourseGeometry::load(&directory(), 10604, 2000.).unwrap();
        assert_eq!(
            tokyo.position(0., 0.).unwrap(),
            [-462.8399963378906_f32, 126.4000015258789_f32]
        );
        assert_eq!(
            tokyo.position(0., 6. / 18.).unwrap(),
            [-466.5899963378906_f32, 126.39814758300781_f32]
        );
        let naka = CourseGeometry::load(&directory(), 10504, 2000.).unwrap();
        for (position, width) in [
            (0., 1.2),
            (874.999, 1.2),
            (875., 0.7),
            (1449.999, 0.7),
            (1450., 1.),
            (874., 1.2),
        ] {
            assert_eq!(naka.lane_max_at(position), width);
        }
        assert!(tokyo.position(-1., 0.).is_err());
        assert!(tokyo.position(2001., 0.).is_ok());
        let overrun_distance=tokyo.overrun.as_ref().unwrap().distance;
        assert_eq!(tokyo.position(2025., 0.).unwrap(),tokyo.position(2025.+overrun_distance, 0.).unwrap());
        let mut without_overrun=tokyo.clone();
        without_overrun.overrun=None;
        assert!(without_overrun.position(2001.,0.).is_err());
        assert!(tokyo.position(f32::NAN, 0.).is_err());
        assert!(tokyo.position(0., f32::INFINITY).is_err());
        assert!(tokyo.advance(0., 0., -1., [0., 0.]).is_err());
    }
    #[test]
    fn invalid_identity_samples_and_widths_are_rejected() {
        assert!(CourseGeometry::from_value(&data(), 10604, 2000.).is_err());
        assert!(CourseGeometry::from_value(&data(), 10504, 2400.).is_err());
        for field in ["distance_to_world", "rotation_fix", "course_id_verified_by"] {
            let mut d = data();
            object(&mut d).insert(field.into(), Value::Null);
            assert!(CourseGeometry::from_value(&d, 10504, 2000.).is_err());
        }
        let mut d = data();
        object(&mut d).insert("course_id".into(), Value::Number(10504.5));
        assert!(CourseGeometry::from_value(&d, 10504, 2000.).is_err());
        let mut d = data();
        let animation = object(&mut d).get_mut("animation").unwrap();
        object(animation).insert("x".into(), Value::Array(vec![]));
        assert!(CourseGeometry::from_value(&d, 10504, 2000.).is_err());
        let mut d = data();
        let identity = object(&mut d).get_mut("course_identity").unwrap();
        object(identity).insert("lane_max".into(), Value::Number(-1.));
        assert!(CourseGeometry::from_value(&d, 10504, 2000.).is_err());
    }
    #[test]
    fn corner_ids_and_final_flag_follow_explicit_events() {
        let geometry = CourseGeometry::bundled(10613, 2400.).unwrap();
        for (pos, expected) in [(499., (0,false)), (500.,(1,false)),
            (750.,(2,false)), (999.99,(2,false)), (1000.,(0,false)), (1000.01,(0,false)),
            (1450.,(3,false)), (1675.,(4,true)), (1899.99,(4,true)), (1900.,(0,true)),
            (1900.01,(0,true)), (2348.,(90,true)), (2400.,(90,true))] {
            assert_eq!(geometry.corner_state_at(pos,true), expected);
            assert_eq!(geometry.corner_state_at(pos,false), (0,false));
        }
    }
    #[test]
    fn final_half_uses_last_eligible_event_start() {
        assert_eq!(CourseGeometry::bundled(10613, 2400.).unwrap().final_corner_half, Some((2598.,2848.)));
        for (start, values, expected) in [(999, "[4,200]", None),
            (1000, "[4,200]", Some((1100.,1200.))),
            (1500, "[4]", None), (1500, "[3,200]", None)] {
            let mut d = data();
            let events = format!("{{\"rows\":[{{\"kind\":0,\"distance\":{start},\"values_i32\":{values}}}]}}");
            object(&mut d).insert("course_event_diagnostics".into(), crate::json::parse(&events).unwrap());
            assert_eq!(CourseGeometry::from_value(&d,10504,2000.).unwrap().final_corner_half, expected);
        }
    }
    #[test]
    fn compiled_segments_preserve_source_interpolation_bits() {
        let catalog = crate::json::parse(include_str!("../data/course_geometry/catalog.json")).unwrap();
        for entry in catalog.get_array("courses").unwrap() {
            let id = entry.get_i64("id").unwrap();
            let data = crate::json::parse(&std::fs::read_to_string(directory().join(format!("{id}.json"))).unwrap()).unwrap();
            for name in ["animation", "overrun"] {
                let Some(source) = data.get(name) else { continue };
                let xs: Vec<_> = source.get_array("x").unwrap().iter().map(|v| finite(v).unwrap()).collect();
                let zs: Vec<_> = source.get_array("z").unwrap().iter().map(|v| finite(v).unwrap()).collect();
                let qs: Vec<_> = source.get_array("rotation").unwrap().iter().map(|v| quaternion(v).unwrap()).collect();
                let segments = animation_segments(&xs, &zs, &qs);
                for (i, segment) in segments.iter().enumerate() {
                    let j = (i + 1).min(1000);
                    for t in [-0., 0., f32::from_bits(1), 0.5, f32::from_bits(1.0_f32.to_bits() - 1), 1.] {
                        // Evaluate the source representation directly, including
                        // exact endpoints and the adjacent representable values.
                        let x = (xs[i] as f64 + (xs[j] as f64 - xs[i] as f64) * t as f64) as f32;
                        let z = (zs[i] as f64 + (zs[j] as f64 - zs[i] as f64) * t as f64) as f32;
                        let q = if i < 1000 { quaternion_lerp(qs[i], qs[j], t) } else { qs[i] };
                        let (actual_x, actual_z, actual_q) = segment.interpolate(t, i == 1000);
                        assert_eq!([actual_x, actual_z].map(f32::to_bits), [x, z].map(f32::to_bits), "{id} {name} {i} {t}");
                        assert_eq!(actual_q.map(f32::to_bits), q.map(f32::to_bits), "{id} {name} {i} {t}");
                    }
                }
            }
        }
    }

    #[test]
    fn short_quaternion_arc() {
        assert_eq!(
            quaternion_lerp([0., 0., 0., 1.], [0., 0., 0., -1.], 0.5),
            [0., 0., 0., 1.]
        );
        let q = quaternion_lerp([0., 0., 0., 1.], [0., 1., 0., 0.], 0.5);
        assert_eq!(q[1], q[3]);
        assert!((dot(q, q) - 1.).abs() < 1e-6);
    }
    #[test]
    fn python_port_vectors_match_bits_for_all_captured_courses() {
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_geometry_port.json"))
        .unwrap();
        for course in fixture.get_array("courses").unwrap() {
            let geometry = CourseGeometry::load(
                &directory(),
                course.get_i64("course_id").unwrap() as i32,
                course.get_f64("distance").unwrap() as f32,
            )
            .unwrap();
            assert_eq!(geometry.first_move_lane_point, number(course,"first_move_lane_point").unwrap());
            assert_eq!(geometry.empty_in_move, course.get("empty_in_move").unwrap().as_bool().unwrap());
            for row in course.get_array("rows").unwrap() {
                let position = number(row, "position").unwrap();
                let lane = number(row, "lane").unwrap();
                let movement = number(row, "movement").unwrap();
                let p = row.get_array("previous_world").unwrap();
                let previous = [finite(&p[0]).unwrap(), finite(&p[1]).unwrap()];
                let (next, world) = geometry
                    .advance(position, lane, movement, previous)
                    .unwrap();
                let actual = [next, world[0], world[1], geometry.lane_max_at(position)];
                for (i, (a, e)) in actual
                    .iter()
                    .zip(row.get_array("expected_bits").unwrap())
                    .enumerate()
                {
                    assert_eq!(
                        a.to_bits(),
                        e.as_i64().unwrap() as u32,
                        "course {} position {position} lane {lane} channel {i}",
                        geometry.course_id
                    );
                }
            }
        }
    }
}
