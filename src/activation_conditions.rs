//! Port of activation_conditions.py -- the skill trigger-condition table.
//!
//! Two-phase system per condition: a static `filter_*` function restricts a
//! region list to where the condition could possibly hold (computed once at
//! skill-build time), paired with an optional dynamic per-tick predicate
//! over a live `RaceSolver` (checked once `pos` enters the statically
//! filtered trigger region). Unlike the Python port, every filter function
//! here returns `(Vec<Region>, OptCond)` uniformly -- `None` plays the role
//! Python's `k_true` sentinel played (see `condition_parser`'s `Op::apply`
//! for where the two sides of an `&`/`@` get recombined).

use std::collections::BTreeMap;

use crate::activation_sample_policy::SamplePolicy;
use crate::course_data::CourseData;
use crate::horse_types::{strategy_matches, HorseParameters, Strategy};
use crate::race_field::RaceField;
use crate::race_parameters::RaceParametersWithSkillId;
use crate::race_solver::RaceSolver;
use crate::region::{rmap_multi, rmap_one, union, Region};

pub type CondResult<T> = Result<T, String>;
/// `field` is `None` during the pre-race "gate skill" activation round
/// (before the roster's `RaceField` exists) and `Some` afterward -- passed
/// explicitly rather than stored on `RaceSolver` so no self-referential
/// pointer/unsafe is needed anywhere in this port; every call site that has
/// a `RaceField` already has it in scope to pass down.
pub type DynCond = Box<dyn Fn(&RaceSolver, Option<&RaceField>) -> bool>;
pub type OptCond = Option<DynCond>;
pub type FilterFn = Box<dyn Fn(&[Region], f64, &CourseData, &HorseParameters, &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> + Send + Sync>;

// ---------------------------------------------------------------------------
// Condition / Operator framework

pub struct Condition {
    pub sample_policy: SamplePolicy,
    pub filter_eq: FilterFn,
    pub filter_neq: FilterFn,
    pub filter_lt: FilterFn,
    pub filter_lte: FilterFn,
    pub filter_gt: FilterFn,
    pub filter_gte: FilterFn,
}

fn not_supported() -> FilterFn {
    Box::new(|_r, _a, _c, _h, _e| Err("unsupported comparison".to_string()))
}

fn noop_filter() -> FilterFn {
    Box::new(|regions, _a, _c, _h, _e| Ok((regions.to_vec(), None)))
}

impl Condition {
    fn new(sample_policy: SamplePolicy) -> Self {
        Condition {
            sample_policy,
            filter_eq: not_supported(),
            filter_neq: not_supported(),
            filter_lt: not_supported(),
            filter_lte: not_supported(),
            filter_gt: not_supported(),
            filter_gte: not_supported(),
        }
    }

    fn eq(mut self, f: FilterFn) -> Self {
        self.filter_eq = f;
        self
    }
    fn neq(mut self, f: FilterFn) -> Self {
        self.filter_neq = f;
        self
    }
    fn lt(mut self, f: FilterFn) -> Self {
        self.filter_lt = f;
        self
    }
    fn lte(mut self, f: FilterFn) -> Self {
        self.filter_lte = f;
        self
    }
    fn gt(mut self, f: FilterFn) -> Self {
        self.filter_gt = f;
        self
    }
    fn gte(mut self, f: FilterFn) -> Self {
        self.filter_gte = f;
        self
    }
}

fn immediate() -> Condition {
    Condition::new(SamplePolicy::Immediate)
}

/// The opt-in runtime reads the preceding AI pass's cached enum (0..3).
/// Static distance regions cannot reproduce its boundary-frame timing.
pub(crate) fn retail_phase_condition() -> &'static Condition {
    static CONDITION: std::sync::OnceLock<Condition> = std::sync::OnceLock::new();
    CONDITION.get_or_init(|| {
        fn filter<C: Fn(f64, f64) -> bool + Copy + Send + Sync + 'static>(compare: C) -> FilterFn {
            Box::new(move |regions,arg,_,_,_| Ok((regions.to_vec(),Some(Box::new(move |s,_| {
                compare(s.retail.as_ref().map_or(0.,|r|r.phase as f64),arg)
            })))))
        }
        immediate().eq(filter(|a,b|a==b)).neq(filter(|a,b|a!=b))
            .lt(filter(|a,b|a<b)).lte(filter(|a,b|a<=b))
            .gt(filter(|a,b|a>b)).gte(filter(|a,b|a>=b))
    })
}
pub(crate) fn retail_distance_rate_condition() -> &'static Condition {
    static CONDITION: std::sync::OnceLock<Condition> = std::sync::OnceLock::new();
    CONDITION.get_or_init(|| {
        fn filter<C: Fn(f64, f64) -> bool + Copy + Send + Sync + 'static>(compare: C) -> FilterFn {
            Box::new(move |regions, arg, course, _, _| {
                let threshold = ((arg as f32 / 100.0 * course.distance as f32) + 0.5) as i32;
                Ok((regions.to_vec(), Some(Box::new(move |s, _| {
                    compare(s.pos as i32 as f64, threshold as f64)
                }))))
            })
        }
        immediate().eq(filter(|a,b|a==b)).neq(filter(|a,b|a!=b))
            .lt(filter(|a,b|a<b)).lte(filter(|a,b|a<=b))
            .gt(filter(|a,b|a>b)).gte(filter(|a,b|a>=b))
    })
}
pub(crate) fn retail_remaining_distance_condition() -> &'static Condition {
    static CONDITION: std::sync::OnceLock<Condition> = std::sync::OnceLock::new();
    CONDITION.get_or_init(|| {
        fn filter<C: Fn(f64, f64) -> bool + Copy + Send + Sync + 'static>(compare: C) -> FilterFn {
            Box::new(move |regions, arg, course, _, _| {
                let distance = course.distance;
                Ok((regions.to_vec(), Some(Box::new(move |s, _| {
                    compare(distance - s.pos.floor(), arg)
                }))))
            })
        }
        immediate().eq(filter(|a,b|a==b)).neq(filter(|a,b|a!=b))
            .lt(filter(|a,b|a<b)).lte(filter(|a,b|a<=b))
            .gt(filter(|a,b|a>b)).gte(filter(|a,b|a>=b))
    })
}
pub(crate) fn retail_final_corner_half_condition() -> &'static Condition {
    static CONDITION: std::sync::OnceLock<Condition> = std::sync::OnceLock::new();
    CONDITION.get_or_init(|| {
        fn filter<C: Fn(f64, f64) -> bool + Copy + Send + Sync + 'static>(compare: C) -> FilterFn {
            Box::new(move |regions, arg, _, _, _| {
                Ok((regions.to_vec(), Some(Box::new(move |s, _| {
                    s.final_corner_half_value().map_or(false, |value| compare(value as i32 as f64, arg))
                }))))
            })
        }
        immediate().eq(filter(|a,b|a==b)).neq(filter(|a,b|a!=b))
            .lt(filter(|a,b|a<b)).lte(filter(|a,b|a<=b))
            .gt(filter(|a,b|a>b)).gte(filter(|a,b|a>=b))
    })
}
pub(crate) fn retail_corner_condition(final_flag: bool) -> &'static Condition {
    static CORNER: std::sync::OnceLock<Condition> = std::sync::OnceLock::new();
    static FINAL: std::sync::OnceLock<Condition> = std::sync::OnceLock::new();
    let slot = if final_flag { &FINAL } else { &CORNER };
    slot.get_or_init(|| {
        fn filter<C: Fn(f64, f64) -> bool + Copy + Send + Sync + 'static>(compare: C, final_flag: bool) -> FilterFn {
            Box::new(move |regions, arg, _, _, _| {
                Ok((regions.to_vec(), Some(Box::new(move |s, _| compare(s.corner_value(final_flag) as f64, arg)))))
            })
        }
        immediate().eq(filter(|a,b|a==b,final_flag)).neq(filter(|a,b|a!=b,final_flag))
            .lt(filter(|a,b|a<b,final_flag)).lte(filter(|a,b|a<=b,final_flag))
            .gt(filter(|a,b|a>b,final_flag)).gte(filter(|a,b|a>=b,final_flag))
    })
}
pub(crate) fn retail_slope_condition() -> &'static Condition {
    static CONDITION: std::sync::OnceLock<Condition> = std::sync::OnceLock::new();
    CONDITION.get_or_init(|| {
        fn filter<C: Fn(f64, f64) -> bool + Copy + Send + Sync + 'static>(compare: C) -> FilterFn {
            Box::new(move |regions, arg, _, _, _| {
                Ok((regions.to_vec(), Some(Box::new(move |s, _| compare(s.slope_type() as f64, arg)))))
            })
        }
        immediate().eq(filter(|a,b|a==b)).neq(filter(|a,b|a!=b))
            .lt(filter(|a,b|a<b)).lte(filter(|a,b|a<=b))
            .gt(filter(|a,b|a>b)).gte(filter(|a,b|a>=b))
    })
}
pub(crate) fn retail_furlong_condition() -> &'static Condition {
    static CONDITION: std::sync::OnceLock<Condition> = std::sync::OnceLock::new();
    CONDITION.get_or_init(|| {
        fn filter<C: Fn(f64, f64) -> bool + Copy + Send + Sync + 'static>(compare: C) -> FilterFn {
            Box::new(move |regions, arg, _, _, _| {
                Ok((regions.to_vec(), Some(Box::new(move |s, _| {
                    compare((((s.pos as f32 + 0.5) * 0.005).floor()) as f64, arg)
                }))))
            })
        }
        immediate().eq(filter(|a,b|a==b)).neq(filter(|a,b|a!=b))
            .lt(filter(|a,b|a<b)).lte(filter(|a,b|a<=b))
            .gt(filter(|a,b|a>b)).gte(filter(|a,b|a>=b))
    })
}
fn random_cond() -> Condition {
    Condition::new(SamplePolicy::Random)
}

fn noop_all(mut c: Condition) -> Condition {
    c.filter_eq = noop_filter();
    c.filter_neq = noop_filter();
    c.filter_lt = noop_filter();
    c.filter_lte = noop_filter();
    c.filter_gt = noop_filter();
    c.filter_gte = noop_filter();
    c
}

fn noop_immediate() -> Condition {
    noop_all(immediate())
}
fn noop_random() -> Condition {
    noop_all(random_cond())
}
fn noop_erlang_random(k: i32, lam: f64) -> Condition {
    noop_all(Condition::new(SamplePolicy::Erlang(k, lam)))
}

type ValueFn = fn(&CourseData, &HorseParameters, &RaceParametersWithSkillId) -> f64;

fn value_filter(get_value: ValueFn) -> Condition {
    fn mk(get_value: ValueFn, cmp: fn(f64, f64) -> bool) -> FilterFn {
        Box::new(move |regions, value, course, horse, extra| {
            let v = get_value(course, horse, extra);
            Ok((if cmp(v, value) { regions.to_vec() } else { Vec::new() }, None))
        })
    }
    immediate()
        .eq(mk(get_value, |v, a| v == a))
        .neq(mk(get_value, |v, a| v != a))
        .lt(mk(get_value, |v, a| v < a))
        .lte(mk(get_value, |v, a| v <= a))
        .gt(mk(get_value, |v, a| v > a))
        .gte(mk(get_value, |v, a| v >= a))
}

type LiveValueFn = fn(&RaceSolver, Option<&RaceField>) -> f64;

fn roster_condition(get: fn(&RaceParametersWithSkillId) -> Option<f64>) -> Condition {
    fn mk(get: fn(&RaceParametersWithSkillId) -> Option<f64>, cmp: fn(f64, f64) -> bool) -> FilterFn {
        Box::new(move |regions, arg, _course, _horse, extra| {
            Ok((if get(extra).map(|v| cmp(v, arg)).unwrap_or(false) { regions.to_vec() } else { Vec::new() }, None))
        })
    }
    immediate().eq(mk(get, |v,a| v==a)).neq(mk(get, |v,a| v!=a))
        .lt(mk(get, |v,a| v<a)).lte(mk(get, |v,a| v<=a))
        .gt(mk(get, |v,a| v>a)).gte(mk(get, |v,a| v>=a))
}

fn same_style_rate_condition() -> Condition {
    // Client 0x35c7d30 rounds the RHS percentage to a runner count.
    fn mk(cmp: fn(f64, f64) -> bool) -> FilterFn {
        Box::new(move |regions, arg, _course, _horse, extra| {
            let matches = extra.params.same_style_count.zip(extra.params.num_umas)
                .filter(|(_, n)| *n > 0).map(|(count, n)| {
                    let threshold = (((arg as f32 / 100.0_f32) * n as f32) + 0.5_f32).trunc();
                    cmp(count as f64, threshold as f64)
                }).unwrap_or(false);
            Ok((if matches { regions.to_vec() } else { Vec::new() }, None))
        })
    }
    immediate().eq(mk(|v,a| v==a)).neq(mk(|v,a| v!=a))
        .lt(mk(|v,a| v<a)).lte(mk(|v,a| v<=a))
        .gt(mk(|v,a| v>a)).gte(mk(|v,a| v>=a))
}
type StaticFilterFn = fn(&[Region], f64, &CourseData, &HorseParameters, &RaceParametersWithSkillId) -> Vec<Region>;

fn identity_transform(arg: f64, _s: &RaceSolver, _f: Option<&RaceField>) -> f64 {
    arg
}
fn passthrough_static(regions: &[Region], _arg: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> Vec<Region> {
    regions.to_vec()
}

/// Port of `_live_condition`: builds a full six-comparator Condition whose
/// runtime value comes from `get_value(state, field)` each tick (`field`
/// `None` -> condition is "not yet determinable", i.e. false, matching the
/// pre-race gate-skill activation round where no field exists yet).
fn live_condition<V, T>(get_value: V, transform_arg: T, static_filter: StaticFilterFn) -> Condition
where V: Fn(&RaceSolver, Option<&RaceField>) -> f64 + Copy + Send + Sync + 'static,
      T: Fn(f64, &RaceSolver, Option<&RaceField>) -> f64 + Copy + Send + Sync + 'static {
    // Keep function items as concrete types so tick predicates can inline the
    // getter, threshold transform and comparison instead of three indirect calls.
    fn mk<V, T, C>(get_value: V, transform_arg: T, static_filter: StaticFilterFn, cmp: C) -> FilterFn
    where V: Fn(&RaceSolver, Option<&RaceField>) -> f64 + Copy + Send + Sync + 'static,
          T: Fn(f64, &RaceSolver, Option<&RaceField>) -> f64 + Copy + Send + Sync + 'static,
          C: Fn(f64, f64) -> bool + Copy + Send + Sync + 'static {
        Box::new(move |regions, arg, course, horse, extra| {
            let restricted = static_filter(regions, arg, course, horse, extra);
            let check: DynCond = Box::new(move |s: &RaceSolver, field: Option<&RaceField>| {
                if field.is_none() {
                    return false;
                }
                cmp(get_value(s, field), transform_arg(arg, s, field))
            });
            Ok((restricted, Some(check)))
        })
    }
    immediate()
        .eq(mk(get_value, transform_arg, static_filter, |v, a| v == a))
        .neq(mk(get_value, transform_arg, static_filter, |v, a| v != a))
        .lt(mk(get_value, transform_arg, static_filter, |v, a| v < a))
        .lte(mk(get_value, transform_arg, static_filter, |v, a| v <= a))
        .gt(mk(get_value, transform_arg, static_filter, |v, a| v > a))
        .gte(mk(get_value, transform_arg, static_filter, |v, a| v >= a))
}

fn live_simple<V>(get_value: V) -> Condition
where V: Fn(&RaceSolver, Option<&RaceField>) -> f64 + Copy + Send + Sync + 'static {
    live_condition(get_value, identity_transform, passthrough_static)
}

fn lane_type_value(s: &RaceSolver, _field: Option<&RaceField>) -> f64 {
    // HorseRaceInfo.get_Lane: inclusive float32 bands, independent of width.
    [0.2_f32, 0.4_f32, 0.6_f32].iter().filter(|&&edge| (s.lane as f32) > edge).count() as f64
}

fn behind_in_condition() -> Condition {
    fn mk(cmp: fn(f64, f64) -> bool) -> FilterFn {
        Box::new(move |regions, arg, _course, _horse, _extra| {
            let check: DynCond = Box::new(move |s, field| {
                // Last place rejects every comparison, including ==0.
                field.and_then(|f| s.field_index.and_then(|i| f.lane_gap_to_behind(i, s.lane)))
                    .map(|gap| cmp(if gap > 0.0 { 1.0 } else { 0.0 }, arg)).unwrap_or(false)
            });
            Ok((regions.to_vec(), Some(check)))
        })
    }
    immediate().eq(mk(|a,b| a==b)).neq(mk(|a,b| a!=b))
        .lt(mk(|a,b| a<b)).lte(mk(|a,b| a<=b))
        .gt(mk(|a,b| a>b)).gte(mk(|a,b| a>=b))
}

fn live_with_static<V>(get_value: V, static_filter: StaticFilterFn) -> Condition
where V: Fn(&RaceSolver, Option<&RaceField>) -> f64 + Copy + Send + Sync + 'static {
    live_condition(get_value, identity_transform, static_filter)
}

fn order_of_value(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| f.order_of(s.field_index.unwrap()) as f64).unwrap_or(0.0)
}

fn order_filter<T>(get_threshold: T) -> Condition
where T: Fn(f64, &RaceSolver, Option<&RaceField>) -> f64 + Copy + Send + Sync + 'static {
    live_condition(order_of_value, get_threshold, passthrough_static)
}

fn order_get_pos(arg: f64, _s: &RaceSolver, _f: Option<&RaceField>) -> f64 {
    arg
}
fn order_rate_get_pos(rate: f64, _s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    let n = field.map(|f| f.n as f64).unwrap_or(0.0);
    (n * (rate / 100.0)).round()
}

/// `order_rate_inXX_continue` / `_outXX_continue`: true once the horse has
/// been continuously in/out of the band since at least accumulatetime t=5s.
/// Per-solver continuity state lives in one of the seven named `Cell`s on
/// `RaceSolver` (`order_band_*`) rather than Python's dynamic
/// `setattr`/`getattr`, since Rust has no equivalent dynamic-attribute
/// mechanism and this codebase only ever needs these seven fixed bands.
fn order_in_filter(rate: f64, band: fn(&RaceSolver) -> &std::cell::Cell<Option<f64>>) -> Condition {
    let feq: FilterFn = Box::new(move |regions, one, _c, _h, _e| {
        if one != 1.0 {
            return Err("order_in_filter expects arg==1".to_string());
        }
        let cell_fn = band;
        let check: DynCond = Box::new(move |s: &RaceSolver, field: Option<&RaceField>| {
            let field = match field {
                Some(f) => f,
                None => return false,
            };
            let idx = s.field_index.unwrap();
            let in_band = (field.order_of(idx) as f64) <= (rate * field.n as f64).round();
            if s.retail.is_some() {
                matches!(cell_fn(s).get(), Some(start) if !start.is_nan())
            } else { continuous_band_check(cell_fn(s), s.accumulatetime.get(), in_band) }
        });
        Ok((regions.to_vec(), Some(check)))
    });
    immediate().eq(feq)
}

fn order_out_filter(rate: f64, band: fn(&RaceSolver) -> &std::cell::Cell<Option<f64>>) -> Condition {
    let feq: FilterFn = Box::new(move |regions, one, _c, _h, _e| {
        if one != 1.0 {
            return Err("order_out_filter expects arg==1".to_string());
        }
        let cell_fn = band;
        let check: DynCond = Box::new(move |s: &RaceSolver, field: Option<&RaceField>| {
            let field = match field {
                Some(f) => f,
                None => return false,
            };
            let idx = s.field_index.unwrap();
            // v69: out bands are inclusive, rank >= round(n*rate) (doc 13.189).
            let in_band = (field.order_of(idx) as f64) >= (rate * field.n as f64).round();
            if s.retail.is_some() {
                matches!(cell_fn(s).get(), Some(start) if !start.is_nan())
            } else { continuous_band_check(cell_fn(s), s.accumulatetime.get(), in_band) }
        });
        Ok((regions.to_vec(), Some(check)))
    });
    immediate().eq(feq)
}

fn band_track(cell: &std::cell::Cell<Option<f64>>, t: f64, in_band: bool) {
    // v27: "continue" = in the band at every rank pass since tracking began (5 s); leaving it
    // once latches the condition false for the rest of the race (NaN marks a broken run).
    // Keep Pushing Ahead (order_rate_in20_continue) never fired for a runner that was 4th-7th at
    // 5.3-7.5 s and 1st-2nd afterwards (v25-prospective case 7); reset-and-rearm fired it at 50 s.
    if t < 5.0 { return; }
    match cell.get() {
        Some(s) if s.is_nan() => {}
        _ if !in_band => cell.set(Some(f64::NAN)),
        None => cell.set(Some(t)),
        _ => {}
    }
}

fn continuous_band_check(cell: &std::cell::Cell<Option<f64>>, t: f64, in_band: bool) -> bool {
    band_track(cell, t, in_band);
    t >= 5.0 && in_band && matches!(cell.get(), Some(s) if !s.is_nan())
}

pub(crate) fn track_order_bands(s: &RaceSolver, rank: usize, thresholds: &[f64; 5]) {
    let t = s.accumulatetime.get();
    let rank = rank as f64;
    band_track(&s.order_band_in_200, t, rank <= thresholds[0]);
    band_track(&s.order_band_in_400, t, rank <= thresholds[1]);
    band_track(&s.order_band_in_500, t, rank <= thresholds[2]);
    band_track(&s.order_band_in_800, t, rank <= thresholds[4]);
    // v69: out bands are inclusive, rank >= round(n*rate) (doc 13.189).
    band_track(&s.order_band_out_200, t, rank >= thresholds[0]);
    band_track(&s.order_band_out_400, t, rank >= thresholds[1]);
    band_track(&s.order_band_out_500, t, rank >= thresholds[2]);
    band_track(&s.order_band_out_700, t, rank >= thresholds[3]);
}

fn section_random(start: f64, end: f64) -> Condition {
    let f = move |regions: &[Region], _arg: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId| {
        let bounds = Region::new(start * (course.distance / 24.0), end * (course.distance / 24.0));
        Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
    };
    let mk = || -> FilterFn { Box::new(f) };
    random_cond().eq(mk()).neq(mk()).lt(mk()).lte(mk()).gt(mk()).gte(mk())
}

// ---------------------------------------------------------------------------
// Skill-id tables transcribed verbatim from ActivationConditions.ts.

const CORNER_RANDOM_QUAD_SKILLS: &[&str] = &[
    "200331", "200332", "200333", "200341", "200342", "200343", "200351", "200352", "200353",
    "200971", "200972", "201041", "201042", "201111", "201112", "201181", "201182",
    "201251", "201252", "201321", "201322", "201391", "201392", "201461", "201462",
];

const PHASE_FUDGE_SKILLS: &[&str] = &["100591", "900591", "110261", "910261", "110191", "910191", "120451", "920451", "101502121"];

const DIRT_GRADE_TRACK_IDS: &[i32] = &[10101, 10103, 10104, 10105];

fn is_sorted_by_start<T>(items: &[T], start: impl Fn(&T) -> f64) -> bool {
    let mut last = -1.0;
    for it in items {
        let s = start(it);
        if s <= last {
            return false;
        }
        last = s;
    }
    true
}

// ---------------------------------------------------------------------------
// Static filter functions (one section per Python function, same order).

fn accumulatetime_condition() -> Condition {
    // Client floors the float32 race clock; there is no distance restriction.
    fn mk(cmp: fn(f64, f64) -> bool) -> FilterFn {
        Box::new(move |regions, arg, _course, _horse, _extra| {
            let check: DynCond = Box::new(move |s, _field| {
                cmp((s.accumulatetime.get() as f32).floor() as f64, arg)
            });
            Ok((regions.to_vec(), Some(check)))
        })
    }
    immediate().eq(mk(|v,a| v==a)).neq(mk(|v,a| v!=a))
        .lt(mk(|v,a| v<a)).lte(mk(|v,a| v<=a))
        .gt(mk(|v,a| v>a)).gte(mk(|v,a| v>=a))
}

fn all_corner_random_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("all_corner_random expects arg==1".to_string());
    }
    let corners: Vec<Region> = course.corners.iter().map(|c| Region::new(c.start, c.start + c.length)).collect();
    Ok((rmap_multi(regions, |r| corners.iter().map(|c| r.intersect(c)).collect()), None))
}

fn corner_filter_eq(regions: &[Region], corner_num: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if !is_sorted_by_start(&course.corners, |c| c.start) {
        return Err("corners not sorted".to_string());
    }
    let corner_num = corner_num as i64;
    if corner_num == 0 {
        let mut last_end = 0.0;
        let mut non_corners = Vec::new();
        for c in &course.corners {
            non_corners.push(Region::new(last_end, c.start));
            last_end = c.start + c.length;
        }
        if last_end != course.distance {
            non_corners.push(Region::new(last_end, course.distance));
        }
        Ok((rmap_multi(regions, |r| non_corners.iter().map(|s| r.intersect(s)).collect()), None))
    } else if (course.corners.len() as i64) + corner_num >= 5 {
        let mut corners = Vec::new();
        let mut idx = course.corners.len() as i64 + corner_num - 5;
        while idx >= 0 {
            let corner = &course.corners[idx as usize];
            corners.push(Region::new(corner.start, corner.start + corner.length));
            idx -= 4;
        }
        corners.reverse();
        Ok((rmap_multi(regions, |r| corners.iter().map(|c| r.intersect(c)).collect()), None))
    } else {
        Ok((Vec::new(), None))
    }
}

fn corner_filter_neq(regions: &[Region], corner_num: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if corner_num != 0.0 {
        return Err("corner!= only supports 0".to_string());
    }
    let corners: Vec<Region> = course.corners.iter().map(|c| Region::new(c.start, c.start + c.length)).collect();
    Ok((rmap_multi(regions, |r| corners.iter().map(|c| r.intersect(c)).collect()), None))
}

fn corner_random_filter_eq(regions: &[Region], corner_num: f64, course: &CourseData, _h: &HorseParameters, extra: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if course.corners.is_empty() {
        return Ok((Vec::new(), None));
    }
    if !is_sorted_by_start(&course.corners, |c| c.start) {
        return Err("corners not sorted".to_string());
    }
    let corner_num = corner_num as i64;
    if CORNER_RANDOM_QUAD_SKILLS.contains(&extra.skill_id.as_str()) {
        if corner_num == 1 {
            let corner = &course.corners[(course.corners.len() as i64 - 4).max(0) as usize];
            let bounds = Region::new(corner.start, corner.start + corner.length);
            return Ok((rmap_one(regions, |r| r.intersect(&bounds)), None));
        }
        return Ok((Vec::new(), None));
    }
    if (course.corners.len() as i64) + corner_num >= 5 {
        let corner = &course.corners[(course.corners.len() as i64 + corner_num - 5) as usize];
        let bounds = Region::new(corner.start, corner.start + corner.length);
        Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
    } else {
        Ok((Vec::new(), None))
    }
}

fn distance_rate_filter_lte(regions: &[Region], rate: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new(0.0, course.distance * rate / 100.0);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn distance_rate_filter_gte(regions: &[Region], rate: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new(course.distance * rate / 100.0, course.distance);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn distance_rate_after_random_filter_eq(regions: &[Region], rate: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new(course.distance * rate / 100.0, course.distance);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn distance_type_filter_eq(regions: &[Region], distance_type: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    Ok((if course.distance_type as f64 == distance_type { regions.to_vec() } else { Vec::new() }, None))
}

fn distance_type_filter_neq(regions: &[Region], distance_type: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    Ok((if course.distance_type as f64 != distance_type { regions.to_vec() } else { Vec::new() }, None))
}

const FURLONG_METERS: f64 = 201.168;

fn furlong_filter_eq(regions: &[Region], n: f64, _course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new((n - 1.0) * FURLONG_METERS, n * FURLONG_METERS);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn down_slope_random_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("down_slope_random expects arg==1".to_string());
    }
    let slopes: Vec<Region> = course.slopes.iter().filter(|s| s.slope < 0.0).map(|s| Region::new(s.start, s.start + s.length)).collect();
    Ok((rmap_multi(regions, |r| slopes.iter().map(|s| r.intersect(s)).collect()), None))
}

fn hp_per_filter_lte(regions: &[Region], hp_per: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let check: DynCond = Box::new(move |s, _field| if s.retail.is_some() {
        hp_condition_percent(s) <= hp_per
    } else { s.hp.hp_ratio_remaining() <= hp_per / 100. });
    Ok((regions.to_vec(), Some(check)))
}

fn hp_per_filter_gte(regions: &[Region], hp_per: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let check: DynCond = Box::new(move |s, _field| if s.retail.is_some() {
        hp_condition_percent(s) >= hp_per
    } else { s.hp.hp_ratio_remaining() >= hp_per / 100. });
    Ok((regions.to_vec(), Some(check)))
}

fn hp_condition_percent(s: &RaceSolver) -> f64 {
    match &s.hp {
        // Store the HP ratio as a float, but retain the wider percentage
        // product until integer truncation. A ratio of 0.71_f32 is just
        // below 71%; rounding the product to float would delay <=70 triggers.
        crate::hp_policy::HpPolicy::Game(hp) => (((hp.hp as f32 / hp.max_hp as f32) as f64 * 100.) as i32) as f64,
        _ => 100.,
    }
}

// Has THIS horse activated a recovery (effect type 9) skill yet? See the
// Python filter's docstring.
fn is_activate_heal_skill_filter_eq(regions: &[Region], flag: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let want = flag == 1.0;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| (s.activate_count_heal > 0) == want);
    Ok((regions.to_vec(), Some(check)))
}

thread_local! {
    /// v75: is_activate_any_skill nodes created while applying one condition (builder log).
    static ANYSKILL_NODES: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub fn take_anyskill_nodes() -> Vec<usize> {
    ANYSKILL_NODES.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

fn is_activate_any_skill_filter_eq(regions: &[Region], one: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("is_activate_any_skill expects arg==1".to_string());
    }
    // A parser node can be reused for multiple samples; mutable observation
    // state must therefore live on the solver rather than inside this closure.
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT_OBSERVER: AtomicUsize = AtomicUsize::new(1);
    let node = NEXT_OBSERVER.fetch_add(1, Ordering::Relaxed);
    ANYSKILL_NODES.with(|log| log.borrow_mut().push(node));
    let own_id: i64 = _e.skill_id.parse().unwrap_or(i64::MAX);
    let _ = own_id;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| {
        if s.retail.is_some() && !s.anyskill_result.borrow().is_empty() {
            // v75: the skill pass updates this trigger at every check of its detail
            // (RaceSolver::observe_anyskill); this reads that check's result.
            return s.anyskill_result.borrow().get(&node).copied().unwrap_or(false);
        }
        if s.retail.is_some() {
            // v70: own activations since this skill's place in the previous tick's scan. A
            // runner's skills are scanned in ascending id (every template skill has priority 0),
            // so that is the previous tick's activations of higher ids plus this tick's so far.
            // A lower-id activation in the previous tick was already visible to that tick's
            // check and no longer counts (v33 counted it). Official census of every 120011/920011
            // carrier in the audited corpus (2026-10-07): 120011 2532/2532 evaluations, 920011
            // 342/346; the four misses are c10708 phase-change ticks (final corner start equals
            // the phase-2 boundary) after a lower-id cast on the previous tick.
            let previous_after = s.activated_ids_previous_tick.iter().any(|&id| id > own_id);
            let this_tick = s.activate_count.iter().sum::<i32>() - s.activate_total_at_tick_start;
            return previous_after || this_tick > 0;
        }
        let mut observers = s.skill_activation_observers.borrow_mut();
        let state = observers.entry(node).or_insert((0, 0));
        let current = s.activate_count.iter().sum::<i32>() - state.1;
        let changed = current > state.0;
        state.0 = current;
        if changed { state.1 += 1; }
        changed
    });
    Ok((regions.to_vec(), Some(check)))
}

fn is_activate_other_skill_detail_filter_eq(regions: &[Region], one: f64, _c: &CourseData, _h: &HorseParameters, extra: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("is_activate_other_skill_detail expects arg==1".to_string());
    }
    let skill_id = extra.skill_id.clone();
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| s.used_skills.contains(&skill_id));
    Ok((regions.to_vec(), Some(check)))
}

fn is_basis_distance_filter_eq(regions: &[Region], flag: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if flag != 0.0 && flag != 1.0 {
        return Err("is_basis_distance expects 0 or 1".to_string());
    }
    let v = ((course.distance as i64) % 400).min(1);
    Ok((if v as f64 != flag { regions.to_vec() } else { Vec::new() }, None))
}

fn is_badstart_filter_eq(regions: &[Region], flag: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if flag != 0.0 && flag != 1.0 {
        return Err("is_badstart expects 0 or 1".to_string());
    }
    let check: DynCond = if flag != 0.0 {
        Box::new(|s: &RaceSolver, _field: Option<&RaceField>| s.start_delay > 0.08)
    } else {
        Box::new(|s: &RaceSolver, _field: Option<&RaceField>| s.start_delay <= 0.08)
    };
    Ok((regions.to_vec(), Some(check)))
}

fn is_dirtgrade_filter_eq(regions: &[Region], flag: f64, course: &CourseData, _h: &HorseParameters, e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let value = e.params.is_dirtgrade.unwrap_or_else(|| DIRT_GRADE_TRACK_IDS.contains(&course.race_track_id));
    Ok((if (value as i32 as f64) == flag { regions.to_vec() } else { Vec::new() }, None))
}

fn is_dirtgrade_filter_neq(regions: &[Region], flag: f64, course: &CourseData, _h: &HorseParameters, e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let value = e.params.is_dirtgrade.unwrap_or_else(|| DIRT_GRADE_TRACK_IDS.contains(&course.race_track_id));
    Ok((if (value as i32 as f64) != flag { regions.to_vec() } else { Vec::new() }, None))
}

// Doc: tight-corner tracks -- Sapporo, Hakodate, Fukushima, Kokura, Kawasaki,
// Funabashi (track_id, not course_set_id).
const TIGHT_TRACK_IDS: &[i32] = &[10001, 10002, 10004, 10010, 10103, 10104];

fn is_tight_track_filter_eq(regions: &[Region], flag: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if flag != 0.0 && flag != 1.0 {
        return Err("is_tight_track expects 0 or 1".to_string());
    }
    let is_tight = TIGHT_TRACK_IDS.contains(&course.race_track_id);
    Ok((if (flag != 0.0) == is_tight { regions.to_vec() } else { Vec::new() }, None))
}

fn is_finalcorner_filter_eq(regions: &[Region], flag: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if flag != 0.0 && flag != 1.0 {
        return Err("is_finalcorner expects 0 or 1".to_string());
    }
    if !is_sorted_by_start(&course.corners, |c| c.start) {
        return Err("corners not sorted".to_string());
    }
    if course.corners.is_empty() {
        return Ok((Vec::new(), None));
    }
    let final_start = course.corners.last().unwrap().start;
    let bounds = if flag != 0.0 { Region::new(final_start, course.distance) } else { Region::new(0.0, final_start) };
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn is_finalcorner_laterhalf_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("is_finalcorner_laterhalf expects arg==1".to_string());
    }
    if !is_sorted_by_start(&course.corners, |c| c.start) {
        return Err("corners not sorted".to_string());
    }
    if course.corners.is_empty() {
        return Ok((Vec::new(), None));
    }
    let fc = course.corners.last().unwrap();
    let bounds = Region::new((fc.start + fc.start + fc.length) / 2.0, fc.start + fc.length);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn is_finalcorner_random_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("is_finalcorner_random expects arg==1".to_string());
    }
    if !is_sorted_by_start(&course.corners, |c| c.start) {
        return Err("corners not sorted".to_string());
    }
    if course.corners.is_empty() {
        return Ok((Vec::new(), None));
    }
    let fc = course.corners.last().unwrap();
    let bounds = Region::new(fc.start, fc.start + fc.length);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn is_hp_empty_onetime_filter_eq(regions: &[Region], one: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("is_hp_empty_onetime expects arg==1".to_string());
    }
    let previous_hp = std::cell::Cell::new(None::<f64>);
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| {
        if s.retail.is_none() { return !s.hp.has_remaining_hp(); }
        let hp = match &s.hp { crate::hp_policy::HpPolicy::Game(hp) => hp.hp, _ => return false };
        let previous = previous_hp.get().unwrap_or(hp);
        if previous <= 0. { previous_hp.set(Some(previous)); return false; }
        previous_hp.set(Some(hp));
        hp <= 0.
    });
    Ok((regions.to_vec(), Some(check)))
}

fn is_lastspurt_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("is_lastspurt expects arg==1".to_string());
    }
    let bounds = Region::new(crate::course_data::phase_start(course.distance, 2), course.distance);
    // v21: the retail frame sets the last-spurt flag from this frame's position
    // before skills are evaluated, so a skill conditioned on is_lastspurt can
    // fire on the flag's first frame. The plan itself is still (re)calculated
    // in the movement update. Early corpus casts of is_lastspurt skills:
    // 47 one-frame-late -> 0 (475 -> 613 matched).
    let check: DynCond = Box::new(|s: &RaceSolver, _field: Option<&RaceField>| s.is_last_spurt
        || (s.phase >= 2 && s.retail.as_ref().is_some_and(|r| {
            let spurt = &r.spurt;
            matches!(spurt.result, 1 | 2) && spurt.start > 0. && !spurt.pending
                && s.pos as f32 >= spurt.start
        })));
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), Some(check)))
}

fn is_last_straight_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("is_last_straight expects arg==1".to_string());
    }
    if !is_sorted_by_start(&course.straights, |s| s.start) {
        return Err("straights not sorted".to_string());
    }
    let last = course.straights.last().ok_or("no straights")?;
    let bounds = Region::new(last.start, last.end);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn is_last_straight_onetime_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("is_last_straight_onetime expects arg==1".to_string());
    }
    if !is_sorted_by_start(&course.straights, |s| s.start) {
        return Err("straights not sorted".to_string());
    }
    let last_start = course.straights.last().ok_or("no straights")?.start;
    let trigger = Region::new(last_start, last_start + 10.0);
    // A one-time entry condition must not become true on a later tick when
    // the runner's rank changes while still inside the distance window.
    let check: DynCond = Box::new(|s, _| s.retail.as_ref()
        .map_or(true, |r| r.last_straight_entry_tick));
    Ok((rmap_one(regions, |r| r.intersect(&trigger)), Some(check)))
}

fn is_temptation_filter_eq(regions: &[Region], b: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| (s.is_kakari as i64 as f64) == b);
    Ok((regions.to_vec(), Some(check)))
}

fn is_used_skill_id_filter_eq(regions: &[Region], skill_id: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let want = format!("{}", skill_id as i64);
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| s.used_skills.contains(&want));
    Ok((regions.to_vec(), Some(check)))
}

fn lastspurt_condition() -> Condition {
    fn mk(cmp: fn(f64, f64) -> bool) -> FilterFn {
        Box::new(move |regions, arg, _, _, _| {
            let check: DynCond = Box::new(move |s, _| s.last_spurt_calc_count > 0 &&
                cmp(match s.last_spurt_calc_result { 1 => 2.0, 2 => 1.0, _ => 0.0 }, arg));
            Ok((regions.to_vec(), Some(check)))
        })
    }
    immediate().eq(mk(|a,b| a==b)).neq(mk(|a,b| a!=b))
        .lt(mk(|a,b| a<b)).lte(mk(|a,b| a<=b))
        .gt(mk(|a,b| a>b)).gte(mk(|a,b| a>=b))
}

fn phase_filter_eq(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, extra: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let phase_i = phase as i32;
    let fudge = if PHASE_FUDGE_SKILLS.contains(&extra.skill_id.as_str()) { 10.0 } else { 0.0 };
    let bounds = Region::new(crate::course_data::phase_start(course.distance, phase_i), crate::course_data::phase_end(course.distance, phase_i) + fudge);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn phase_filter_lt(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if phase <= 0.0 {
        return Err("phase< requires phase>0".to_string());
    }
    let bounds = Region::new(0.0, crate::course_data::phase_start(course.distance, phase as i32));
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn phase_filter_lte(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new(0.0, crate::course_data::phase_end(course.distance, phase as i32));
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn phase_filter_gt(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if phase >= 3.0 {
        return Err("phase> requires phase<3".to_string());
    }
    let bounds = Region::new(crate::course_data::phase_start(course.distance, phase as i32 + 1), course.distance);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn phase_filter_gte(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new(crate::course_data::phase_start(course.distance, phase as i32), course.distance);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn phase_corner_random_filter_eq(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let phase_i = phase as i32;
    let phase_start = crate::course_data::phase_start(course.distance, phase_i);
    let phase_end = crate::course_data::phase_end(course.distance, phase_i);
    let mut corners = Vec::new();
    for c in &course.corners {
        let c_end = c.start + c.length;
        if (phase_start <= c.start && c.start < phase_end) || (phase_start <= c_end && c_end < phase_end) {
            corners.push(Region::new(c.start.max(phase_start), c_end.min(phase_end)));
        }
    }
    Ok((rmap_multi(regions, |r| corners.iter().map(|c| r.intersect(c)).collect()), None))
}

fn phase_firsthalf_filter_eq(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let phase_i = phase as i32;
    let start = crate::course_data::phase_start(course.distance, phase_i);
    let end = crate::course_data::phase_end(course.distance, phase_i);
    let bounds = Region::new(start, start + (end - start) / 2.0);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn phase_firstquarter_filter_eq(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let phase_i = phase as i32;
    let start = crate::course_data::phase_start(course.distance, phase_i);
    let end = crate::course_data::phase_end(course.distance, phase_i);
    let bounds = Region::new(start, start + (end - start) / 4.0);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn phase_laterhalf_filter_eq(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let phase_i = phase as i32;
    let start = crate::course_data::phase_start(course.distance, phase_i);
    let end = crate::course_data::phase_end(course.distance, phase_i);
    let bounds = Region::new((start + end) / 2.0, end);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn phase_laterhalf_random_filter_eq(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let phase_i = phase as i32;
    let start = crate::course_data::phase_start(course.distance, phase_i);
    let end = crate::course_data::phase_end(course.distance, phase_i);
    let bounds = Region::new((start + end) / 2.0, end);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn phase_random_filter_eq(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let phase_i = phase as i32;
    let bounds = Region::new(crate::course_data::phase_start(course.distance, phase_i), crate::course_data::phase_end(course.distance, phase_i));
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn phase_straight_random_filter_eq(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let phase_i = phase as i32;
    let phase_bounds = Region::new(crate::course_data::phase_start(course.distance, phase_i), crate::course_data::phase_end(course.distance, phase_i));
    let straights: Vec<Region> = course.straights.iter().map(|s| Region::new(s.start, s.end)).collect();
    let mapped = rmap_multi(regions, |r| straights.iter().map(|s| r.intersect(s)).collect());
    Ok((rmap_one(&mapped, |r| r.intersect(&phase_bounds)), None))
}

fn phase_first_half_straight_random_filter_eq(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let phase_i = phase as i32;
    let start = crate::course_data::phase_start(course.distance, phase_i);
    let end = crate::course_data::phase_end(course.distance, phase_i);
    let half_bounds = Region::new(start, start + (end - start) / 2.0);
    let straights: Vec<Region> = course.straights.iter().map(|s| Region::new(s.start, s.end)).collect();
    let mapped = rmap_multi(regions, |r| straights.iter().map(|s| r.intersect(s)).collect());
    Ok((rmap_one(&mapped, |r| r.intersect(&half_bounds)), None))
}

fn phase_latter_half_straight_random_filter_eq(regions: &[Region], phase: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let phase_i = phase as i32;
    let start = crate::course_data::phase_start(course.distance, phase_i);
    let end = crate::course_data::phase_end(course.distance, phase_i);
    let half_bounds = Region::new(start + (end - start) / 2.0, end);
    let straights: Vec<Region> = course.straights.iter().map(|s| Region::new(s.start, s.end)).collect();
    let mapped = rmap_multi(regions, |r| straights.iter().map(|s| r.intersect(s)).collect());
    Ok((rmap_one(&mapped, |r| r.intersect(&half_bounds)), None))
}

fn gate_block(gate_number: i64, num_umas: i64) -> i64 {
    if num_umas < 1 || gate_number < 0 || gate_number >= num_umas { return 1; }
    if num_umas <= 8 { return gate_number + 1; }
    // Extracted client table through 18; balanced extension for custom rooms.
    let per_post = num_umas / 8;
    let small_posts = 8 - num_umas % 8;
    let cutoff = small_posts * per_post;
    if gate_number < cutoff { gate_number / per_post + 1 }
    else { small_posts + (gate_number - cutoff) / (per_post + 1) + 1 }
}

fn or_default_num_umas(v: Option<i32>) -> i64 {
    match v {
        Some(n) if n != 0 => n as i64,
        _ => 9,
    }
}

fn post_number_filter_eq(regions: &[Region], post: f64, _c: &CourseData, _h: &HorseParameters, extra: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let num_umas = or_default_num_umas(extra.params.num_umas);
    let post = post as i64;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| gate_block(s.gate_index as i64, num_umas) == post);
    Ok((regions.to_vec(), Some(check)))
}

fn post_number_filter_lte(regions: &[Region], post: f64, _c: &CourseData, _h: &HorseParameters, extra: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let num_umas = or_default_num_umas(extra.params.num_umas);
    let post = post as i64;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| gate_block(s.gate_index as i64, num_umas) <= post);
    Ok((regions.to_vec(), Some(check)))
}

fn post_number_filter_gte(regions: &[Region], post: f64, _c: &CourseData, _h: &HorseParameters, extra: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let num_umas = or_default_num_umas(extra.params.num_umas);
    let post = post as i64;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| gate_block(s.gate_index as i64, num_umas) >= post);
    Ok((regions.to_vec(), Some(check)))
}

fn random_lot_filter_eq(regions: &[Region], lot: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| (s.random_lot as f64) < lot);
    Ok((regions.to_vec(), Some(check)))
}

fn remain_distance_filter_eq(regions: &[Region], remain: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new(course.distance - remain, course.distance - remain + 1.0);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn remain_distance_filter_lte(regions: &[Region], remain: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new(course.distance - remain, course.distance);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn remain_distance_filter_gte(regions: &[Region], remain: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new(0.0, course.distance - remain);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn run_at_full_speed_random_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("run_at_full_speed_random expects arg==1".to_string());
    }
    let bounds = Region::new(crate::course_data::phase_start(course.distance, 3), course.distance);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn running_style_filter_eq(regions: &[Region], strategy: f64, _c: &CourseData, horse: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let strategy = Strategy::from_i32(strategy as i32)?;
    Ok((if strategy_matches(horse.strategy, strategy) { regions.to_vec() } else { Vec::new() }, None))
}

fn slope_filter_eq(regions: &[Region], slope_type: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let slope_type = slope_type as i64;
    if !(0..=2).contains(&slope_type) {
        return Err("slope_type must be 0,1,2".to_string());
    }
    if !is_sorted_by_start(&course.slopes, |s| s.start) {
        return Err("slopes not sorted".to_string());
    }
    let slopes: Vec<_> = course.slopes.iter().filter(|s| (slope_type != 2 && s.slope > 0.0) || (slope_type != 1 && s.slope < 0.0)).collect();
    let slope_r: Vec<Region> = if slope_type == 0 {
        let mut last_end = 0.0;
        let mut out = Vec::new();
        for s in &slopes {
            out.push(Region::new(last_end, s.start));
            last_end = s.start + s.length;
        }
        if last_end != course.distance {
            out.push(Region::new(last_end, course.distance));
        }
        out
    } else {
        slopes.iter().map(|s| Region::new(s.start, s.start + s.length)).collect()
    };
    Ok((rmap_multi(regions, |r| slope_r.iter().map(|s| r.intersect(s)).collect()), None))
}

fn straight_front_type_filter_eq(regions: &[Region], front_type: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let front_type = front_type as i32;
    if front_type != 1 && front_type != 2 {
        return Err("front_type must be 1 or 2".to_string());
    }
    let straights: Vec<Region> = course.straights.iter().filter(|s| s.front_type == front_type).map(|s| Region::new(s.start, s.end)).collect();
    Ok((rmap_multi(regions, |r| straights.iter().map(|s| r.intersect(s)).collect()), None))
}

fn straight_random_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("straight_random expects arg==1".to_string());
    }
    let straights: Vec<Region> = course.straights.iter().map(|s| Region::new(s.start, s.end)).collect();
    Ok((rmap_multi(regions, |r| straights.iter().map(|s| r.intersect(s)).collect()), None))
}

fn last_straight_random_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("last_straight_random expects arg==1".to_string());
    }
    if !is_sorted_by_start(&course.straights, |s| s.start) {
        return Err("straights not sorted".to_string());
    }
    let last = match course.straights.last() {
        Some(s) => s,
        None => return Ok((Vec::new(), None)),
    };
    let bounds = Region::new(last.start, last.end);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn temptation_count_filter_eq(regions: &[Region], n: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let n = n as i32;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| s.temptation_count == n);
    Ok((regions.to_vec(), Some(check)))
}

fn up_slope_random_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("up_slope_random expects arg==1".to_string());
    }
    let slopes: Vec<Region> = course.slopes.iter().filter(|s| s.slope > 0.0).map(|s| Region::new(s.start, s.start + s.length)).collect();
    Ok((rmap_multi(regions, |r| slopes.iter().map(|s| r.intersect(s)).collect()), None))
}

fn up_slope_random_later_half_filter_eq(regions: &[Region], one: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if one != 1.0 {
        return Err("up_slope_random_later_half expects arg==1".to_string());
    }
    let half = Region::new(course.distance / 2.0, course.distance);
    let slopes: Vec<Region> = course.slopes.iter().filter(|s| s.slope > 0.0).map(|s| Region::new(s.start, s.start + s.length)).collect();
    let mapped = rmap_multi(regions, |r| slopes.iter().map(|s| r.intersect(s)).collect());
    Ok((rmap_one(&mapped, |r| r.intersect(&half)), None))
}

fn compete_fight_count_filter_gt(regions: &[Region], arg: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    // Count actual mechanic activations; the solver controls duel eligibility.
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| s.compete_fight_count as f64 > arg);
    Ok((regions.to_vec(), Some(check)))
}

fn is_overtaking_value(s: &RaceSolver, _field: Option<&RaceField>) -> f64 {
    // Client IsOverTake checks its passing-target list, not rank improvement.
    s.retail.as_ref().map_or(!s.passing_targets.is_empty(),|r|!r.lane.targets.is_empty()) as i64 as f64
}

fn change_order_up_end_after_static(regions: &[Region], _a: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> Vec<Region> {
    let bounds = Region::new(crate::course_data::phase_start(course.distance, 2), course.distance);
    rmap_one(regions, |r| r.intersect(&bounds))
}

fn change_order_up_finalcorner_after_static(regions: &[Region], _a: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> Vec<Region> {
    if course.corners.is_empty() {
        return Vec::new();
    }
    let final_corner_start = course.corners.last().unwrap().start;
    let bounds = Region::new(final_corner_start, course.distance);
    rmap_one(regions, |r| r.intersect(&bounds))
}

fn change_order_up_middle_static(regions: &[Region], _a: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> Vec<Region> {
    let bounds = Region::new(crate::course_data::phase_start(course.distance, 1), crate::course_data::phase_end(course.distance, 1));
    rmap_one(regions, |r| r.intersect(&bounds))
}

fn activate_count_all_filter_lte(regions: &[Region], n: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let n = n as i32;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| s.activate_count.iter().sum::<i32>() <= n);
    Ok((regions.to_vec(), Some(check)))
}

fn activate_count_all_filter_gte(regions: &[Region], n: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let n = n as i32;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| s.activate_count.iter().sum::<i32>() >= n);
    Ok((regions.to_vec(), Some(check)))
}

fn activate_count_end_after_filter_gte(regions: &[Region], n: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let n = n as i32;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| s.activate_count[2] >= n);
    Ok((regions.to_vec(), Some(check)))
}

fn activate_count_heal_filter_gte(regions: &[Region], n: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let n = n as i32;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| s.activate_count_heal >= n);
    Ok((regions.to_vec(), Some(check)))
}

// Skills THIS horse activated past the halfway mark of the course -- the
// DISTANCE half, not a phase. See the Python filter's docstring.
fn activate_count_later_half_filter_gte(regions: &[Region], n: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let n = n as i32;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| s.activate_count_later_half >= n);
    Ok((regions.to_vec(), Some(check)))
}

fn activate_count_middle_filter_gte(regions: &[Region], n: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let n = n as i32;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| s.activate_count[1] >= n);
    Ok((regions.to_vec(), Some(check)))
}

fn activate_count_start_filter_gte(regions: &[Region], n: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let n = n as i32;
    let check: DynCond = Box::new(move |s: &RaceSolver, _field: Option<&RaceField>| s.activate_count[0] >= n);
    Ok((regions.to_vec(), Some(check)))
}

// -- value_filter getters (all capture nothing / a fixed Copy value, so are
// plain function pointers -- see `ValueFn`) ---------------------------------

fn v_base_power(_c: &CourseData, h: &HorseParameters, _e: &RaceParametersWithSkillId) -> f64 { h.power }
fn v_base_speed(_c: &CourseData, h: &HorseParameters, _e: &RaceParametersWithSkillId) -> f64 { h.speed }
fn v_base_stamina(_c: &CourseData, h: &HorseParameters, _e: &RaceParametersWithSkillId) -> f64 { h.stamina }
fn v_base_guts(_c: &CourseData, h: &HorseParameters, _e: &RaceParametersWithSkillId) -> f64 { h.guts }
fn v_base_wiz(_c: &CourseData, h: &HorseParameters, _e: &RaceParametersWithSkillId) -> f64 { h.wisdom }
fn v_corner_count(c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> f64 { c.corners.len() as f64 }
fn v_course_distance(c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> f64 { c.distance }
fn v_grade(_c: &CourseData, _h: &HorseParameters, e: &RaceParametersWithSkillId) -> f64 { e.params.grade as i32 as f64 }
fn v_ground_condition(_c: &CourseData, _h: &HorseParameters, e: &RaceParametersWithSkillId) -> f64 { e.params.ground_condition as i32 as f64 }
fn v_ground_type(c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> f64 { c.surface as f64 }
fn v_motivation(_c: &CourseData, _h: &HorseParameters, e: &RaceParametersWithSkillId) -> f64 { (e.params.mood + 3) as f64 }
fn v_popularity(_c: &CourseData, _h: &HorseParameters, e: &RaceParametersWithSkillId) -> f64 { e.params.popularity as f64 }
fn v_rotation(c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> f64 { c.turn as f64 }
fn v_season(_c: &CourseData, _h: &HorseParameters, e: &RaceParametersWithSkillId) -> f64 { e.params.season as i32 as f64 }
fn v_time(_c: &CourseData, _h: &HorseParameters, e: &RaceParametersWithSkillId) -> f64 { e.params.time as i32 as f64 }
fn v_track_id(c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> f64 { c.race_track_id as f64 }
fn v_weather(_c: &CourseData, _h: &HorseParameters, e: &RaceParametersWithSkillId) -> f64 { e.params.weather as i32 as f64 }

// -- live_condition value getters --------------------------------------

fn lv_visiblehorse(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    if let Some(retail) = &s.retail {
        return retail.lane.visible.len() as f64;
    }
    field.map_or(0., |f| f.solvers.iter().enumerate().filter(|(i, other)| {
        if Some(*i) == s.field_index { return false; }
        let other = other.borrow();
        crate::retail_lane::is_visible((other.pos - s.pos) as f32,
            (other.lane - s.lane) as f32, s.visible_distance() as f32)
    }).count() as f64)
}

fn bashin_condition(ahead: bool) -> Condition {
    fn mk(ahead: bool, cmp: fn(f64, f64) -> bool) -> FilterFn {
        Box::new(move |regions, arg, _course, _horse, _extra| {
            let check: DynCond = Box::new(move |s, field| {
                // Client CheckActivate rejects missing neighbours for every
                // comparator; they are not an infinitely distant opponent.
                let gap = field.and_then(|f| s.field_index.and_then(|i| {
                    if ahead { f.distance_to_ahead(i, s.pos) }
                    else { f.distance_to_behind(i, s.pos) }
                }));
                gap.map(|d| cmp(d / 2.5, arg)).unwrap_or(false)
            });
            Ok((regions.to_vec(), Some(check)))
        })
    }
    immediate().eq(mk(ahead, |a,b| a==b)).neq(mk(ahead, |a,b| a!=b))
        .lt(mk(ahead, |a,b| a<b)).lte(mk(ahead, |a,b| a<=b))
        .gt(mk(ahead, |a,b| a>b)).gte(mk(ahead, |a,b| a>=b))
}
fn lv_behind_near_lane_time(s: &RaceSolver, _f: Option<&RaceField>) -> f64 { s.behind_near_lane_timer.get().floor() }
fn lv_behind_near_lane_time_set1(s: &RaceSolver, _f: Option<&RaceField>) -> f64 { s.behind_near_lane_set1_timer.get().floor() }
// Client 0x35c7930 reads min(front, side) then truncates to integer seconds.
fn lv_blocked_all_continuetime(s: &RaceSolver, _f: Option<&RaceField>) -> f64 {
    s.blocked_front_timer.get().min(s.blocked_side_timer.get()).trunc()
}
fn lv_blocked_front(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| f.front_blocker(s.field_index.unwrap(), s.pos, s.lane).is_some() as i64 as f64).unwrap_or(0.0)
}
fn lv_blocked_front_continuetime(s: &RaceSolver, _f: Option<&RaceField>) -> f64 { s.blocked_front_timer.get().trunc() }
fn lv_blocked_side_continuetime(s: &RaceSolver, _f: Option<&RaceField>) -> f64 { s.blocked_side_timer.get().trunc() }
fn lv_distance_diff_rate(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| {
        let (first, last) = f.ranked_distance_bounds(s.field_index.unwrap(), s.pos);
        distance_spread_percentage(first as f32, last as f32, s.pos as f32)
    }).unwrap_or(0.0)
}
// SkillTriggerHorseDistanceDiffRate (0x35c6500); the denominator is the
// field's spread, not the course length. Match Mathf.Approximately at zero.
fn distance_spread_percentage(first: f32, last: f32, own: f32) -> f64 {
    let spread = first - last;
    if spread.abs() < (1e-6_f32 * spread.abs()).max(8.0 * f32::from_bits(1)) {
        return 0.0;
    }
    (((first - own) / spread) * 100.0).floor() as f64
}
fn leader_gap(s: &RaceSolver, field: &RaceField) -> f32 {
    let (first, _) = field.ranked_distance_bounds(s.field_index.unwrap(), s.pos);
    first as f32 - s.pos as f32
}
fn lv_distance_diff_top(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| leader_gap(s, f).floor() as f64).unwrap_or(0.0)
}
fn lv_distance_diff_top_float(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| ((leader_gap(s, f) * 10.0).floor() / 10.0) as f64).unwrap_or(0.0)
}
fn lv_infront_near_lane_time(s: &RaceSolver, _f: Option<&RaceField>) -> f64 { s.infront_near_lane_timer.get().floor() }
fn lv_is_move_lane(s: &RaceSolver, _f: Option<&RaceField>) -> f64 {
    if s.move_lane_continue_time >= 1. {
        1. + (s.move_lane_continue_direction != 0) as i32 as f64
    } else { 0. }
}
fn lv_is_surrounded(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| f.is_surrounded(s.field_index.unwrap(), s.pos, s.lane) as i64 as f64).unwrap_or(0.0)
}
fn lv_near_count(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| f.near_count(s.field_index.unwrap(), s.pos, s.lane) as f64).unwrap_or(0.0)
}
fn lv_near_infront_count(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| f.near_infront_count(s.field_index.unwrap(), s.pos, s.lane) as f64).unwrap_or(0.0)
}
fn lv_temptation_count_behind(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| f.temptation_count_behind(s.field_index.unwrap(), false) as f64).unwrap_or(0.0)
}
fn lv_temptation_count_infront(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| f.temptation_count_infront(s.field_index.unwrap(), false) as f64).unwrap_or(0.0)
}
fn lv_temptation_opponent_count_behind(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| f.temptation_count_behind(s.field_index.unwrap(), true) as f64).unwrap_or(0.0)
}
fn lv_temptation_opponent_count_infront(s: &RaceSolver, field: Option<&RaceField>) -> f64 {
    field.map(|f| f.temptation_count_infront(s.field_index.unwrap(), true) as f64).unwrap_or(0.0)
}
fn running_style_temptation_count_live<const OPPONENTS: bool>(strategy: Strategy) -> Condition {
    // Condition needs a plain fn pointer (LiveValueFn), not a closure over
    // `strategy` -- dispatch through the small match below instead.
    fn nige<const O: bool>(s: &RaceSolver, field: Option<&RaceField>) -> f64 { field.map(|f| f.running_style_temptation_count(Strategy::Nige, s.field_index.unwrap(), s.is_kakari, O) as f64).unwrap_or(0.0) }
    fn senko<const O: bool>(s: &RaceSolver, field: Option<&RaceField>) -> f64 { field.map(|f| f.running_style_temptation_count(Strategy::Senkou, s.field_index.unwrap(), s.is_kakari, O) as f64).unwrap_or(0.0) }
    fn sashi<const O: bool>(s: &RaceSolver, field: Option<&RaceField>) -> f64 { field.map(|f| f.running_style_temptation_count(Strategy::Sasi, s.field_index.unwrap(), s.is_kakari, O) as f64).unwrap_or(0.0) }
    fn oikomi<const O: bool>(s: &RaceSolver, field: Option<&RaceField>) -> f64 { field.map(|f| f.running_style_temptation_count(Strategy::Oikomi, s.field_index.unwrap(), s.is_kakari, O) as f64).unwrap_or(0.0) }
    let get_value: LiveValueFn = match strategy {
        Strategy::Nige => nige::<OPPONENTS>,
        Strategy::Senkou => senko::<OPPONENTS>,
        Strategy::Sasi => sashi::<OPPONENTS>,
        Strategy::Oikomi => oikomi::<OPPONENTS>,
        Strategy::Oonige => nige::<OPPONENTS>,
    };
    live_simple(get_value)
}

fn cell_order_band_in_200(s: &RaceSolver) -> &std::cell::Cell<Option<f64>> { &s.order_band_in_200 }
fn cell_order_band_in_400(s: &RaceSolver) -> &std::cell::Cell<Option<f64>> { &s.order_band_in_400 }
fn cell_order_band_in_500(s: &RaceSolver) -> &std::cell::Cell<Option<f64>> { &s.order_band_in_500 }
fn cell_order_band_in_800(s: &RaceSolver) -> &std::cell::Cell<Option<f64>> { &s.order_band_in_800 }
fn cell_order_band_out_200(s: &RaceSolver) -> &std::cell::Cell<Option<f64>> { &s.order_band_out_200 }
fn cell_order_band_out_400(s: &RaceSolver) -> &std::cell::Cell<Option<f64>> { &s.order_band_out_400 }
fn cell_order_band_out_500(s: &RaceSolver) -> &std::cell::Cell<Option<f64>> { &s.order_band_out_500 }
fn cell_order_band_out_700(s: &RaceSolver) -> &std::cell::Cell<Option<f64>> { &s.order_band_out_700 }

// ---------------------------------------------------------------------------

/// Looks up `name` in `table`. `table` itself must be `'static` -- both
/// condition tables are built once and leaked (see `conditions_table`/
/// `conditions_table_acr` in race_solver_builder.rs) -- so `BTreeMap::get`
/// already hands back a `&'static Condition` on its own; no unsafe lifetime
/// extension needed. That `'static`-ness is what lets `Op::Cmp` hold a bare
/// `&'static Condition` with no lifetime parameter threaded through the
/// rest of the engine.
pub fn lookup_condition(table: &'static BTreeMap<String, Condition>, name: &str) -> Option<&'static Condition> {
    table.get(name)
}

#[derive(Clone, Copy)]
pub enum CmpKind {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
}

pub enum Op {
    Fixed(crate::retail_initialization::Windows),
    /// Immutable parsed structure is shared; sampled windows belong to this call.
    Shared(std::rc::Rc<Op>, Vec<crate::retail_initialization::Windows>),
    Sampled(usize),
    Cmp { cond: &'static Condition, kind: CmpKind, arg: f64 },
    And(Box<Op>, Box<Op>, SamplePolicy),
    Or(Box<Op>, Box<Op>, SamplePolicy),
}

impl Op {
    pub fn sample_policy(&self) -> &SamplePolicy {
        match self {
            Op::Fixed(_) | Op::Sampled(_) => &SamplePolicy::Immediate,
            Op::Shared(root, _) => root.sample_policy(),
            Op::Cmp { cond, .. } => &cond.sample_policy,
            Op::And(_, _, sp) | Op::Or(_, _, sp) => sp,
        }
    }

    pub fn apply(&self, regions: &[Region], course: &CourseData, horse: &HorseParameters, extra: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
        self.apply_sampled(regions, course, horse, extra, &[])
    }

    fn apply_sampled(&self, regions: &[Region], course: &CourseData, horse: &HorseParameters, extra: &RaceParametersWithSkillId,
        sampled: &[crate::retail_initialization::Windows]) -> CondResult<(Vec<Region>, OptCond)> {
        match self {
            Op::Shared(root, windows) => root.apply_sampled(regions, course, horse, extra, windows),
            Op::Sampled(index) => Self::apply_windows(&sampled[*index], regions),
            Op::Fixed(crate::retail_initialization::Windows::Lottery(enabled)) =>
                Ok((if *enabled {regions.to_vec()} else {Vec::new()},None)),
            Op::Fixed(crate::retail_initialization::Windows::Distances(windows)) => {
                Ok((rmap_multi(regions, |r| windows.iter().map(|&(start,end)|
                    r.intersect(&Region::new(start as f64,end as f64))).collect()),None))
            }
            Op::Cmp { cond, kind, arg } => {
                let f = match kind {
                    CmpKind::Eq => &cond.filter_eq,
                    CmpKind::Neq => &cond.filter_neq,
                    CmpKind::Lt => &cond.filter_lt,
                    CmpKind::Lte => &cond.filter_lte,
                    CmpKind::Gt => &cond.filter_gt,
                    CmpKind::Gte => &cond.filter_gte,
                };
                f(regions, *arg, course, horse, extra)
            }
            Op::And(left, right, _) => {
                let (leftval, leftcond) = left.apply_sampled(regions, course, horse, extra, sampled)?;
                let (rightval, rightcond) = right.apply_sampled(&leftval, course, horse, extra, sampled)?;
                match (leftcond, rightcond) {
                    (None, None) => Ok((rightval, None)),
                    (Some(cond), None) | (None, Some(cond)) => Ok((rightval, Some(cond))),
                    (Some(lc), Some(rc)) => {
                        let combined: DynCond = Box::new(move |s: &RaceSolver, field: Option<&RaceField>| lc(s, field) && rc(s, field));
                        Ok((rightval, Some(combined)))
                    }
                }
            }
            Op::Or(left, right, _) => {
                let (leftval, leftcond) = left.apply_sampled(regions, course, horse, extra, sampled)?;
                let (rightval, rightcond) = right.apply_sampled(regions, course, horse, extra, sampled)?;
                let merged = union(&leftval, &rightval);
                let lc = leftcond.unwrap_or_else(|| Box::new(|_: &RaceSolver, _: Option<&RaceField>| true));
                let rc = rightcond.unwrap_or_else(|| Box::new(|_: &RaceSolver, _: Option<&RaceField>| true));
                let combined: DynCond = Box::new(move |s: &RaceSolver, field: Option<&RaceField>| {
                    (leftval.iter().any(|r| r.start <= s.pos && s.pos < r.end) && lc(s, field))
                        || (rightval.iter().any(|r| r.start <= s.pos && s.pos < r.end) && rc(s, field))
                });
                Ok((merged, Some(combined)))
            }
        }
    }

    fn apply_windows(windows: &crate::retail_initialization::Windows, regions: &[Region]) -> CondResult<(Vec<Region>, OptCond)> {
        use crate::retail_initialization::Windows;
        Ok(match windows {
            Windows::Lottery(enabled) => (if *enabled { regions.to_vec() } else { Vec::new() }, None),
            Windows::Distances(windows) => (rmap_multi(regions, |r| windows.iter().map(|&(start, end)|
                r.intersect(&Region::new(start as f64, end as f64))).collect()), None),
        })
    }

    /// Replace seed-dependent leaves after a successful parse. The cached tree
    /// never retains sampled windows or predicates created by `apply`.
    pub(crate) fn separate_samples(&mut self, windows: &mut Vec<crate::retail_initialization::Windows>) {
        match self {
            Op::Fixed(_) => {
                let Op::Fixed(value) = std::mem::replace(self, Op::Sampled(windows.len())) else { unreachable!() };
                windows.push(value);
            }
            Op::And(left, right, _) | Op::Or(left, right, _) => {
                left.separate_samples(windows);
                right.separate_samples(windows);
            }
            _ => {}
        }
    }
}

pub fn make_and(left: Op, right: Op) -> CondResult<Op> {
    let sp = left.sample_policy().reconcile(right.sample_policy())?;
    Ok(Op::And(Box::new(left), Box::new(right), sp))
}

pub fn make_or(left: Op, right: Op) -> CondResult<Op> {
    let sp = left.sample_policy().reconcile(right.sample_policy())?;
    Ok(Op::Or(Box::new(left), Box::new(right), sp))
}

// ---------------------------------------------------------------------------
// The condition tables.

pub fn build_conditions() -> BTreeMap<String, Condition> {
    let mut c: BTreeMap<String, Condition> = BTreeMap::new();
    c.insert("accumulatetime".into(), accumulatetime_condition());
    c.insert(
        "activate_count_all".into(),
        immediate().lte(Box::new(activate_count_all_filter_lte)).gte(Box::new(activate_count_all_filter_gte)),
    );
    c.insert("activate_count_end_after".into(), immediate().gte(Box::new(activate_count_end_after_filter_gte)));
    c.insert("activate_count_heal".into(), immediate().gte(Box::new(activate_count_heal_filter_gte)));
    c.insert("activate_count_later_half".into(), immediate().gte(Box::new(activate_count_later_half_filter_gte)));
    c.insert("activate_count_middle".into(), immediate().gte(Box::new(activate_count_middle_filter_gte)));
    c.insert("activate_count_start".into(), immediate().gte(Box::new(activate_count_start_filter_gte)));
    c.insert(
        "all_corner_random".into(),
        Condition::new(SamplePolicy::AllCornerRandom).eq(Box::new(all_corner_random_filter_eq)),
    );
    c.insert("always".into(), noop_immediate());
    c.insert("base_power".into(), value_filter(v_base_power));
    c.insert("base_speed".into(), value_filter(v_base_speed));
    c.insert("base_stamina".into(), value_filter(v_base_stamina));
    c.insert("base_guts".into(), value_filter(v_base_guts));
    c.insert("base_wiz".into(), value_filter(v_base_wiz));
    c.insert("bashin_diff_behind".into(), bashin_condition(false));
    c.insert("bashin_diff_infront".into(), bashin_condition(true));
    c.insert("behind_near_lane_time".into(), live_simple(lv_behind_near_lane_time));
    c.insert("behind_near_lane_time_set1".into(), live_simple(lv_behind_near_lane_time_set1));
    c.insert("blocked_all_continuetime".into(), live_simple(lv_blocked_all_continuetime));
    c.insert("blocked_front".into(), live_simple(lv_blocked_front));
    c.insert("blocked_front_continuetime".into(), live_simple(lv_blocked_front_continuetime));
    c.insert("blocked_side_continuetime".into(), live_simple(lv_blocked_side_continuetime));
    c.insert("change_order_onetime".into(), live_simple(|s,f| { let f=f.unwrap(); let i=s.field_index.unwrap(); f.order_of(i) as f64 - f.previous_order_of(i) as f64 }));
    c.insert("change_order_up_end_after".into(), live_simple(|s,_| s.phase_overtakes[2] as f64));
    c.insert("change_order_up_finalcorner_after".into(), live_simple(|s,_| s.final_corner_overtakes as f64));
    c.insert("change_order_up_middle".into(), live_simple(|s,_| s.phase_overtakes[1] as f64));
    c.insert("compete_fight_count".into(), immediate().gt(Box::new(compete_fight_count_filter_gt)));
    c.insert("corner".into(), immediate().eq(Box::new(corner_filter_eq)).neq(Box::new(corner_filter_neq)));
    c.insert("corner_count".into(), value_filter(v_corner_count));
    c.insert("corner_random".into(), random_cond().eq(Box::new(corner_random_filter_eq)));
    c.insert("course_distance".into(), value_filter(v_course_distance));
    c.insert("distance_diff_rate".into(), live_simple(lv_distance_diff_rate));
    c.insert("distance_diff_top".into(), live_simple(lv_distance_diff_top));
    c.insert("distance_diff_top_float".into(), live_simple(lv_distance_diff_top_float));
    c.insert(
        "distance_rate".into(),
        immediate().lte(Box::new(distance_rate_filter_lte)).gte(Box::new(distance_rate_filter_gte)),
    );
    c.insert("distance_rate_after_random".into(), random_cond().eq(Box::new(distance_rate_after_random_filter_eq)));
    c.insert(
        "distance_type".into(),
        immediate().eq(Box::new(distance_type_filter_eq)).neq(Box::new(distance_type_filter_neq)),
    );
    c.insert("down_slope_random".into(), random_cond().eq(Box::new(down_slope_random_filter_eq)));
    c.insert("furlong".into(), immediate().eq(Box::new(furlong_filter_eq)));
    c.insert("grade".into(), value_filter(v_grade));
    c.insert("ground_condition".into(), value_filter(v_ground_condition));
    c.insert("ground_type".into(), value_filter(v_ground_type));
    c.insert("hp_per".into(), immediate().lte(Box::new(hp_per_filter_lte)).gte(Box::new(hp_per_filter_gte)));
    c.insert("infront_near_lane_time".into(), live_simple(lv_infront_near_lane_time));
    c.insert("is_activate_any_skill".into(), immediate().eq(Box::new(is_activate_any_skill_filter_eq)));
    c.insert("is_activate_heal_skill".into(), immediate().eq(Box::new(is_activate_heal_skill_filter_eq)));
    c.insert("is_activate_other_skill_detail".into(), immediate().eq(Box::new(is_activate_other_skill_detail_filter_eq)));
    c.insert("is_basis_distance".into(), immediate().eq(Box::new(is_basis_distance_filter_eq)));
    c.insert("is_badstart".into(), immediate().eq(Box::new(is_badstart_filter_eq)));
    c.insert("is_behind_in".into(), behind_in_condition());
    c.insert("is_dirtgrade".into(), immediate().eq(Box::new(is_dirtgrade_filter_eq)).neq(Box::new(is_dirtgrade_filter_neq)));
    c.insert("is_tight_track".into(), immediate().eq(Box::new(is_tight_track_filter_eq)));
    c.insert("is_exist_skill_id".into(), noop_immediate());
    c.insert("is_finalcorner".into(), immediate().eq(Box::new(is_finalcorner_filter_eq)));
    c.insert("is_finalcorner_laterhalf".into(), immediate().eq(Box::new(is_finalcorner_laterhalf_filter_eq)));
    c.insert("is_finalcorner_random".into(), random_cond().eq(Box::new(is_finalcorner_random_filter_eq)));
    c.insert("is_hp_empty_onetime".into(), immediate().eq(Box::new(is_hp_empty_onetime_filter_eq)));
    c.insert("is_lastspurt".into(), immediate().eq(Box::new(is_lastspurt_filter_eq)));
    c.insert("is_last_straight".into(), immediate().eq(Box::new(is_last_straight_filter_eq)));
    c.insert("is_last_straight_onetime".into(), immediate().eq(Box::new(is_last_straight_onetime_filter_eq)));
    c.insert("is_move_lane".into(), live_simple(lv_is_move_lane));
    // Cached at trigger construction in the client, before passive stats/HP.
    c.insert("is_exist_chara_id".into(), immediate().eq(Box::new(|regions, arg, _, _, extra| {
        Ok((if extra.params.chara_ids.as_ref().is_some_and(|ids| ids.contains(&(arg as i64))) { regions.to_vec() } else { vec![] }, None))
    })).neq(Box::new(|regions, arg, _, _, extra| {
        Ok((if extra.params.chara_ids.as_ref().is_some_and(|ids| !ids.contains(&(arg as i64))) { regions.to_vec() } else { vec![] }, None))
    })));
    c.insert("is_other_character_activate_advantage_skill".into(), immediate().eq(Box::new(|regions, arg, _, _, _| {
        Ok((regions.to_vec(), Some(Box::new(move |s, field| arg.fract() == 0.0 && field.is_some_and(|f| f.other_advantage_active(s, arg as i32))))))
    })));
    c.insert("remain_distance_viewer_id".into(), live_simple(|s,f| f.unwrap().viewer_remaining(s)));
    c.insert("is_overtake".into(), live_simple(is_overtaking_value));
    c.insert("is_surrounded".into(), live_simple(lv_is_surrounded));
    c.insert("is_temptation".into(), immediate().eq(Box::new(is_temptation_filter_eq)));
    c.insert("is_used_skill_id".into(), immediate().eq(Box::new(is_used_skill_id_filter_eq)));
    c.insert("lane_type".into(), live_simple(lane_type_value));
    c.insert("lastspurt".into(), lastspurt_condition());
    c.insert("motivation".into(), value_filter(v_motivation));
    c.insert("near_count".into(), live_simple(lv_near_count));
    c.insert("near_infront_count".into(), live_simple(lv_near_infront_count));
    c.insert("order".into(), order_filter(order_get_pos));
    c.insert("order_rate".into(), order_filter(order_rate_get_pos));
    c.insert("order_rate_in20_continue".into(), order_in_filter(0.2, cell_order_band_in_200));
    c.insert("order_rate_in40_continue".into(), order_in_filter(0.4, cell_order_band_in_400));
    c.insert("order_rate_in50_continue".into(), order_in_filter(0.5, cell_order_band_in_500));
    c.insert("order_rate_in80_continue".into(), order_in_filter(0.8, cell_order_band_in_800));
    c.insert("order_rate_out20_continue".into(), order_out_filter(0.2, cell_order_band_out_200));
    c.insert("order_rate_out40_continue".into(), order_out_filter(0.4, cell_order_band_out_400));
    c.insert("order_rate_out50_continue".into(), order_out_filter(0.5, cell_order_band_out_500));
    c.insert("order_rate_out70_continue".into(), order_out_filter(0.7, cell_order_band_out_700));
    c.insert("overtake_target_no_order_up_time".into(), live_simple(|s,_| s.overtake_target_no_order_up_time.floor()));
    c.insert("overtake_target_no_order_down_time".into(), live_simple(|s,_| s.overtake_target_no_order_down_time.floor()));
    c.insert("overtake_target_time".into(), live_simple(|s,_| s.overtake_target_time.floor()));
    c.insert(
        "phase".into(),
        Condition::new(SamplePolicy::Immediate)
            .eq(Box::new(phase_filter_eq))
            .lt(Box::new(phase_filter_lt))
            .lte(Box::new(phase_filter_lte))
            .gt(Box::new(phase_filter_gt))
            .gte(Box::new(phase_filter_gte)),
    );
    c.insert("phase_corner_random".into(), random_cond().eq(Box::new(phase_corner_random_filter_eq)));
    c.insert("phase_firsthalf".into(), immediate().eq(Box::new(phase_firsthalf_filter_eq)));
    c.insert("phase_firsthalf_random".into(), random_cond().eq(Box::new(phase_firsthalf_filter_eq)));
    c.insert(
        "phase_first_half_straight_random".into(),
        Condition::new(SamplePolicy::StraightRandom).eq(Box::new(phase_first_half_straight_random_filter_eq)),
    );
    c.insert("phase_firstquarter".into(), immediate().eq(Box::new(phase_firstquarter_filter_eq)));
    c.insert("phase_firstquarter_random".into(), random_cond().eq(Box::new(phase_firstquarter_filter_eq)));
    c.insert("phase_laterhalf".into(), immediate().eq(Box::new(phase_laterhalf_filter_eq)));
    c.insert("phase_laterhalf_random".into(), random_cond().eq(Box::new(phase_laterhalf_random_filter_eq)));
    c.insert(
        "phase_latter_half_straight_random".into(),
        Condition::new(SamplePolicy::StraightRandom).eq(Box::new(phase_latter_half_straight_random_filter_eq)),
    );
    c.insert("phase_random".into(), random_cond().eq(Box::new(phase_random_filter_eq)));
    c.insert(
        "phase_straight_random".into(),
        Condition::new(SamplePolicy::StraightRandom).eq(Box::new(phase_straight_random_filter_eq)),
    );
    c.insert("popularity".into(), value_filter(v_popularity));
    c.insert(
        "post_number".into(),
        immediate().eq(Box::new(post_number_filter_eq)).lte(Box::new(post_number_filter_lte)).gte(Box::new(post_number_filter_gte)),
    );
    c.insert("random_lot".into(), immediate().eq(Box::new(random_lot_filter_eq)));
    c.insert(
        "remain_distance".into(),
        immediate().eq(Box::new(remain_distance_filter_eq)).lte(Box::new(remain_distance_filter_lte)).gte(Box::new(remain_distance_filter_gte)),
    );
    c.insert("rotation".into(), value_filter(v_rotation));
    c.insert("run_at_full_speed_random".into(), random_cond().eq(Box::new(run_at_full_speed_random_filter_eq)));
    c.insert("running_style".into(), immediate().eq(Box::new(running_style_filter_eq)));
    c.insert("running_style_count_same".into(), roster_condition(|e| e.params.same_style_count.map(|n| n as f64)));
    c.insert("running_style_count_same_rate".into(), same_style_rate_condition());
    c.insert("running_style_count_nige_otherself".into(), roster_condition(|e| e.params.other_style_counts.map(|n| n[0] as f64)));
    c.insert("running_style_count_senko_otherself".into(), roster_condition(|e| e.params.other_style_counts.map(|n| n[1] as f64)));
    c.insert("running_style_count_sashi_otherself".into(), roster_condition(|e| e.params.other_style_counts.map(|n| n[2] as f64)));
    c.insert("running_style_count_oikomi_otherself".into(), roster_condition(|e| e.params.other_style_counts.map(|n| n[3] as f64)));
    c.insert("running_style_equal_popularity_one".into(), roster_condition(|e| e.params.favorite_matches.map(|v| if v {1.0} else {0.0})));
    c.insert("running_style_temptation_count_nige".into(), running_style_temptation_count_live::<false>(Strategy::Nige));
    c.insert("running_style_temptation_count_senko".into(), running_style_temptation_count_live::<false>(Strategy::Senkou));
    c.insert("running_style_temptation_count_sashi".into(), running_style_temptation_count_live::<false>(Strategy::Sasi));
    c.insert("running_style_temptation_count_oikomi".into(), running_style_temptation_count_live::<false>(Strategy::Oikomi));
    // Opponents exclude self and equal nonzero team IDs.
    c.insert("running_style_temptation_opponent_count_nige".into(), running_style_temptation_count_live::<true>(Strategy::Nige));
    c.insert("running_style_temptation_opponent_count_senko".into(), running_style_temptation_count_live::<true>(Strategy::Senkou));
    c.insert("running_style_temptation_opponent_count_sashi".into(), running_style_temptation_count_live::<true>(Strategy::Sasi));
    c.insert("running_style_temptation_opponent_count_oikomi".into(), running_style_temptation_count_live::<true>(Strategy::Oikomi));
    c.insert("same_skill_horse_count".into(), roster_condition(|e| e.params.skill_counts.as_ref().map(|counts| *counts.get(&e.skill_id).unwrap_or(&0) as f64)));
    c.insert("season".into(), value_filter(v_season));
    c.insert("slope".into(), immediate().eq(Box::new(slope_filter_eq)));
    c.insert("straight_front_type".into(), immediate().eq(Box::new(straight_front_type_filter_eq)));
    c.insert(
        "straight_random".into(),
        Condition::new(SamplePolicy::StraightRandom).eq(Box::new(straight_random_filter_eq)),
    );
    c.insert(
        "last_straight_random".into(),
        Condition::new(SamplePolicy::StraightRandom).eq(Box::new(last_straight_random_filter_eq)),
    );
    c.insert("temptation_count".into(), immediate().eq(Box::new(temptation_count_filter_eq)));
    c.insert("temptation_count_behind".into(), live_simple(lv_temptation_count_behind));
    c.insert("temptation_count_infront".into(), live_simple(lv_temptation_count_infront));
    c.insert("temptation_opponent_count_behind".into(), live_simple(lv_temptation_opponent_count_behind));
    c.insert("temptation_opponent_count_infront".into(), live_simple(lv_temptation_opponent_count_infront));
    c.insert("time".into(), value_filter(v_time));
    c.insert("track_id".into(), value_filter(v_track_id));
    c.insert("up_slope_random".into(), random_cond().eq(Box::new(up_slope_random_filter_eq)));
    c.insert("up_slope_random_later_half".into(), random_cond().eq(Box::new(up_slope_random_later_half_filter_eq)));
    c.insert("visiblehorse".into(), live_simple(lv_visiblehorse));
    c.insert("weather".into(), value_filter(v_weather));
    c
}

fn activate_count_all_random_filter_gte(regions: &[Region], n: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    if n == 7.0 {
        let out: Vec<Region> = regions.iter().map(|r| Region::new(r.start, r.start + 11.0)).collect();
        return Ok((out, None));
    }
    let lo = (n / 23.0 - 0.2).min(0.6) * course.distance;
    let hi = (n / 23.0 + 0.2).min(1.0) * course.distance;
    let bounds = Region::new(lo, hi);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn activate_count_all_random_filter_lte(_regions: &[Region], _n: f64, _c: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    Ok((Vec::new(), None))
}

fn acr_activate_count_end_after_filter_gte(regions: &[Region], _n: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new(crate::course_data::phase_start(course.distance, 2), crate::course_data::phase_end(course.distance, 3));
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn acr_activate_count_later_half_filter_gte(regions: &[Region], _n: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new(course.distance / 2.0, course.distance);
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn acr_activate_count_middle_filter_gte(regions: &[Region], n: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let start = crate::course_data::phase_start(course.distance, 1);
    let end = crate::course_data::phase_end(course.distance, 1);
    let bounds = Region::new(start, start + n / 10.0 * (end - start));
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

fn acr_activate_count_start_filter_gte(regions: &[Region], _n: f64, course: &CourseData, _h: &HorseParameters, _e: &RaceParametersWithSkillId) -> CondResult<(Vec<Region>, OptCond)> {
    let bounds = Region::new(crate::course_data::phase_start(course.distance, 0), crate::course_data::phase_end(course.distance, 0));
    Ok((rmap_one(regions, |r| r.intersect(&bounds)), None))
}

/// Port of `_build_conditions_with_activate_counts_as_random` (used via
/// `.withActivateCountsAsRandom()`): the same table with six entries
/// swapped for probabilistic-activation-count variants.
pub fn build_conditions_with_activate_counts_as_random() -> BTreeMap<String, Condition> {
    let mut c = build_conditions();
    c.insert(
        "activate_count_all".into(),
        random_cond().gte(Box::new(activate_count_all_random_filter_gte)).lte(Box::new(activate_count_all_random_filter_lte)),
    );
    c.insert("activate_count_end_after".into(), random_cond().gte(Box::new(acr_activate_count_end_after_filter_gte)));
    c.insert("activate_count_heal".into(), noop_random());
    c.insert("activate_count_later_half".into(), random_cond().gte(Box::new(acr_activate_count_later_half_filter_gte)));
    c.insert("activate_count_middle".into(), random_cond().gte(Box::new(acr_activate_count_middle_filter_gte)));
    c.insert("activate_count_start".into(), immediate().gte(Box::new(acr_activate_count_start_filter_gte)));
    c
}

#[cfg(test)]
mod trigger_contract_tests {
    use super::{gate_block, distance_spread_percentage};

    #[test]
    fn distance_percentage_uses_field_spread_and_floors() {
        assert_eq!(distance_spread_percentage(100.0, 80.0, 100.0), 0.0);
        assert_eq!(distance_spread_percentage(100.0, 80.0, 94.0), 30.0);
        assert_eq!(distance_spread_percentage(100.0, 80.0, 94.02), 29.0);
        assert_eq!(distance_spread_percentage(100.0, 80.0, 90.0), 50.0);
        assert_eq!(distance_spread_percentage(100.0, 80.0, 80.0), 100.0);
        assert_eq!(distance_spread_percentage(100.0, 100.0, 100.0), 0.0);
        assert_eq!(distance_spread_percentage(100.0, 90.0, 100.2), -2.0);
    }

    #[test]
    fn large_field_extends_balanced_outer_post_grouping() {
        for size in 19..=120 {
            let posts: Vec<_> = (0..size).map(|gate| gate_block(gate, size)).collect();
            assert_eq!(posts[0], 1);
            assert_eq!(*posts.last().unwrap(), 8);
            assert!(posts.windows(2).all(|w| w[0] <= w[1]));
            let counts: Vec<_> = (1..=8).map(|p| posts.iter().filter(|&&v| v==p).count()).collect();
            assert!(counts.iter().max().unwrap() - counts.iter().min().unwrap() <= 1);
        }
    }

    #[test]
    fn ordinary_eighteen_runner_posts_match_extracted_client_table() {
        let expected = [1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 7, 8, 8, 8];
        for (gate, post) in expected.into_iter().enumerate() {
            assert_eq!(gate_block(gate as i64, 18), post);
        }
        assert_eq!(gate_block(8, 9), 8);
        assert_eq!(gate_block(0, 9), 1);
        assert_eq!(gate_block(18, 18), 1);
    }
}
