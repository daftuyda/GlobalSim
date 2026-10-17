//! Port of race_solver.py -- the core per-tick physics/skill-activation
//! loop. Uses the same velocity-Verlet-style integration, the same three
//! compensated-sum (Kahan-Babuska-Neumaier) accumulators for skill
//! modifiers, and precomputes `base_target_speed`/`base_accel` arrays
//! exactly once at construction time (a subtlety worth flagging: green/
//! stat-boost skills that fire mid-race mutate `self.horse` but do NOT
//! retroactively change these precomputed arrays, matching the original
//! engine's behavior bit-for-bit).
//!
//! `field: Option<&RaceField>` is threaded as an explicit parameter through
//! every method that needs multi-horse awareness (matching Python's
//! `self.field`, which the field-coordination layer -- not part of the
//! original engine -- sets once the whole roster is built) rather than
//! stored on the struct, so this file needs no self-referential pointer or
//! unsafe code; see race_field.rs's module doc for why.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, BTreeMap};
use std::rc::Rc;

use crate::activation_conditions::DynCond;
use crate::course_data::CourseData;
use crate::horse_types::{strategy_matches, HorseParameters, Strategy};
use crate::hp_policy::{HpPolicy, HpTickCtx};
use crate::race_field::{RaceField, HORSE_LANE};
use crate::random_gen::Rule30CARng;
use crate::region::Region;
#[path = "race_solver_retail.rs"]
mod retail_impl;
pub(crate) use retail_impl::{RetailRuntimeConfig, RetailRuntimeState};

// Extracted Global RaceParamDefine: LaneGap and solver.lane both use
// course-width units, not metres. CompeteTop has separate entry/active caps.
const SPOT_STRUGGLE_LANE_GAP: f64 = 0.165;
const DUEL_LANE_GAP: f64 = 0.25;
const SPOT_STRUGGLE_CHECK_END_SECTION: f64 = 6.0;
const SPOT_STRUGGLE_END_SECTION: f64 = 8.0;

// ---------------------------------------------------------------------------
// Speed / acceleration tables (Object.freeze([...]) in RaceSolver.ts)

// index 0 unused (strategies are 1-indexed)
const SPEED_STRATEGY_PHASE_COEFFICIENT: [[f64; 3]; 6] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.98, 0.962],
    [0.978, 0.991, 0.975],
    [0.938, 0.998, 0.994],
    [0.931, 1.0, 1.0],
    [1.063, 0.962, 0.95],
];
const SPEED_DISTANCE_PROFICIENCY_MODIFIER: [f64; 8] = [1.05, 1.0, 0.9, 0.8, 0.6, 0.4, 0.2, 0.1];

fn base_speed(course: &CourseData) -> f64 {
    20.0 - (course.distance - 2000.0) / 1000.0
}

fn base_target_speed(horse: &HorseParameters, course: &CourseData, phase: usize) -> f64 {
    base_speed(course) * SPEED_STRATEGY_PHASE_COEFFICIENT[horse.strategy as usize][phase]
        + (if phase == 2 { 1.0 } else { 0.0 }) * (500.0 * horse.speed).sqrt() * SPEED_DISTANCE_PROFICIENCY_MODIFIER[horse.distance_aptitude as usize] * 0.002
}

fn last_spurt_speed(horse: &HorseParameters, course: &CourseData) -> f64 {
    (base_target_speed(horse, course, 2) + 0.01 * base_speed(course)) * 1.05
        + (500.0 * horse.speed).sqrt() * SPEED_DISTANCE_PROFICIENCY_MODIFIER[horse.distance_aptitude as usize] * 0.002
        + (450.0 * horse.guts).powf(0.597) * 0.0001
}

const ACCEL_STRATEGY_PHASE_COEFFICIENT: [[f64; 3]; 6] = [
    [0.0, 0.0, 0.0],
    [1.0, 1.0, 0.996],
    [0.985, 1.0, 0.996],
    [0.975, 1.0, 1.0],
    [0.945, 1.0, 0.997],
    [1.17, 0.94, 0.956],
];
const ACCEL_GROUND_TYPE_PROFICIENCY_MODIFIER: [f64; 8] = [1.05, 1.0, 0.9, 0.8, 0.7, 0.5, 0.3, 0.1];
const ACCEL_DISTANCE_PROFICIENCY_MODIFIER: [f64; 8] = [1.0, 1.0, 1.0, 1.0, 1.0, 0.6, 0.5, 0.4];

const BASE_ACCEL: f64 = 0.0006;
const UPHILL_BASE_ACCEL: f64 = 0.0004;

fn calc_base_accel(base_accel_const: f64, horse: &HorseParameters, phase: usize) -> f64 {
    base_accel_const
        * (500.0 * horse.power).sqrt()
        * ACCEL_STRATEGY_PHASE_COEFFICIENT[horse.strategy as usize][phase]
        * ACCEL_GROUND_TYPE_PROFICIENCY_MODIFIER[horse.surface_aptitude as usize]
        * ACCEL_DISTANCE_PROFICIENCY_MODIFIER[horse.distance_aptitude as usize]
}

const PHASE_DECELERATION: [f64; 3] = [-1.2, -0.8, -1.0];

const STRATEGY_PROFICIENCY_MODIFIER_APPROX: [f64; 8] = [1.1, 1.0, 0.85, 0.75, 0.6, 0.4, 0.2, 0.1];

fn compete_dist_threshold(s: Strategy) -> f64 {
    match s {
        Strategy::Oonige | Strategy::Nige => 0.0,
        Strategy::Senkou => 2.5,
        Strategy::Sasi => 5.0,
        Strategy::Oikomi => 10.0,
    }
}
fn compete_strategy_coef(s: Strategy) -> f64 {
    match s {
        Strategy::Oonige => 0.2,
        Strategy::Nige => 0.8,
        _ => 1.0,
    }
}
fn compete_stamina_strategy_coef(s: Strategy) -> f64 {
    match s {
        Strategy::Oonige => 1.5,
        Strategy::Nige => 1.2,
        _ => 1.0,
    }
}
// Secure Lead's real DesirableLead formula uses a full per-strategy-pair
// coefficient matrix the doc summary this port was built from only
// captured a few example entries of -- approximated with one flat
// coefficient instead of reconstructing a guessed-at full matrix.
const SECURE_LEAD_COEF_APPROX: f64 = 3.0;
fn secure_lead_strategy_coef(s: Strategy) -> f64 {
    match s {
        Strategy::Oonige => 0.2,
        Strategy::Nige => 1.0,
        Strategy::Senkou => 1.0,
        Strategy::Sasi => 0.8,
        Strategy::Oikomi => 1.0,
    }
}
fn secure_lead_stamina_strategy_coef(s: Strategy) -> f64 {
    match s {
        Strategy::Oonige => 1.2,
        Strategy::Nige => 1.0,
        Strategy::Senkou => 0.8,
        Strategy::Sasi => 0.8,
        Strategy::Oikomi => 0.8,
    }
}

/// Shared course-length coefficient table (doc: identical brackets used by
/// both Compete Before Spurt and Secure Lead's stamina-cost formulas).
fn competition_distance_coefficient(distance: f64) -> f64 {
    if distance < 1801.0 {
        0.3
    } else if distance < 2101.0 {
        0.5
    } else if distance < 2201.0 {
        0.8
    } else if distance < 2401.0 {
        1.0
    } else if distance < 2601.0 {
        1.1
    } else {
        1.2
    }
}

// JS's arrays only have indices 0-4 (Oonige=5 reads as `undefined`, silently
// NaN-poisoning threshold math that is never actually read for Oonige/Nige
// horses since position-keep is a no-op for them). Index 5 is NaN so Rust
// indexing doesn't panic where JS wouldn't have crashed either.
const POSITION_KEEP_BASE_MIN_THRESHOLD: [f64; 6] = [0.0, 0.0, 3.0, 6.5, 7.5, f64::NAN];
const POSITION_KEEP_BASE_MAX_THRESHOLD: [f64; 6] = [0.0, 0.0, 5.0, 7.0, 8.0, f64::NAN];

fn position_keep_course_factor(distance: f64) -> f64 {
    (0.0008 * (distance - 1000.0) + 1.0).max(1.0)
}
fn position_keep_min_threshold(strategy: Strategy, distance: f64) -> f64 {
    let factor = if strategy == Strategy::Senkou { 1.0 } else { position_keep_course_factor(distance) };
    POSITION_KEEP_BASE_MIN_THRESHOLD[strategy as usize] * factor
}
fn position_keep_max_threshold(strategy: Strategy, distance: f64) -> f64 {
    POSITION_KEEP_BASE_MAX_THRESHOLD[strategy as usize] * position_keep_course_factor(distance)
}

// ---------------------------------------------------------------------------

pub type Timer = Rc<Cell<f64>>;

fn new_timer_value(t: f64) -> Timer {
    Rc::new(Cell::new(t))
}

/// Kahan-Babuska-Neumaier sum: for any sequence, adding a value in any order
/// interleaved with subtracting it in any other order results in
/// acc+err == 0.0 (needed since skills add/remove modifiers at arbitrary
/// times over a long race).
#[derive(Clone, Copy)]
pub struct CompensatedAccumulator {
    pub acc: f64,
    pub err: f64,
}

impl CompensatedAccumulator {
    fn new(acc: f64) -> Self {
        CompensatedAccumulator { acc, err: 0.0 }
    }

    pub fn add(&mut self, n: f64) {
        let t = self.acc + n;
        if self.acc.abs() >= n.abs() {
            self.err += (self.acc - t) + n;
        } else {
            self.err += (n - t) + self.acc;
        }
        self.acc = t;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Perspective {
    Self_ = 1,
    Other = 2,
    Any = 3,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkillType {
    Noop = 0,
    SpeedUp = 1,
    StaminaUp = 2,
    PowerUp = 3,
    GutsUp = 4,
    WisdomUp = 5,
    Runaway = 6,
    Vision = 8,
    SetTargetLane = 35,
    Recovery = 9,
    MultiplyStartDelay = 10,
    PreferOutsideLane = 11,
    PreferInsideLane = 12,
    ExtendKakari = 13,
    SetStartDelay = 14,
    CurrentSpeed = 21,
    CurrentSpeedWithNaturalDeceleration = 22,
    TargetSpeed = 27,
    LaneMoveSpeedUp = 28,
    ModifyKakariChance = 29,
    Accel = 31,
    AllStatusUp = 32,
    ActivateRandomSkill = 36,
    ActivateRandomGold = 37,
    DebuffImmunity = 38,
    ExtendEvolvedDuration = 42,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum SkillRarity {
    White = 1,
    Gold = 2,
    Unique = 3,
    Evolution = 6,
}

#[derive(Clone, Copy)]
pub struct SkillEffect {
    pub effect_type: SkillType,
    pub base_duration: f64,
    pub modifier: f64,
    pub value_usage: i32,
    pub duration_usage: i32,
    pub additional_activate: i32,
    pub target: i32,
    pub target_value: i32,
    pub target_is_debuff: bool,
}

impl SkillEffect {
    fn duration_coefficient(&self, remaining_hp: Option<f64>, leader_gap: Option<f64>, retail: bool) -> f64 {
        // Extracted Global Skill duration parameters; compare with strict <.
        if self.duration_usage == 2 {
            if let Some(gap) = leader_gap {
                if retail {
                    // Round the normalized gap before adding the minimum.
                    // Keeping the division wide can change the recorded duration.
                    let ratio = gap.max(0.0) as f32 / 62.5_f32;
                    return (0.8_f32 + ratio).min(1.6_f32) as f64;
                }
                return (0.800000011920929 + gap.max(0.0) / 62.5).min(1.600000023841858);
            }
        }
        if let Some(hp) = remaining_hp {
            let tiers: &[(f64, f64)] = match self.duration_usage {
                3 => &[(2000.0, 1.0), (2400.0, 1.5), (2600.0, 2.0), (2800.0, 2.200000047683716),
                       (3000.0, 2.5), (3200.0, 3.0), (3500.0, 3.5)],
                7 => &[(1500.0, 1.0), (1800.0, 1.5), (2000.0, 2.0), (2100.0, 2.5)],
                _ => return 1.0,
            };
            return tiers.iter().find(|(bound, _)| hp < *bound).map(|(_, coef)| *coef)
                .unwrap_or(if self.duration_usage == 3 { 4.0 } else { 3.0 });
        }
        1.0
    }

    fn activation_modifier(&self, rng: &mut Rule30CARng, activated_green_count: usize) -> f64 {
        if self.effect_type != SkillType::Noop && self.value_usage == 14 {
            // Global Skill.ActivateSpecificTagGroupSkillCountCoefAscArray.
            let count = activated_green_count;
            return self.modifier * if count < 3 { 0.0 } else if count < 5 { 1.0 }
                else if count < 6 { 2.0 } else { 3.0 };
        }
        if self.effect_type != SkillType::Noop && matches!(self.value_usage, 8 | 9) {
            // Global Skill.AbiiltyValueUsageRandom{1,2}Array, selected by
            // CreateCalculator for usages 8/9; draw once per activation.
            let roll = rng.uniform(100.0);
            let coef = if roll < 60 { 0.0 } else if roll < 90 {
                0.019999999552965164
            } else { 0.03999999910593033 };
            self.modifier * coef
        } else {
            self.modifier
        }
    }
}

pub struct PendingSkill {
    pub random_activation_order: Option<usize>,
    pub detail_index: usize,
    pub priority: i32,
    pub skill_id: String,
    /// Parsed once; the identifier is immutable while a skill is pending.
    pub numeric_skill_id: Option<i64>,
    pub has_green_tag: bool,
    pub perspective: Perspective,
    pub rarity: SkillRarity,
    pub trigger: Region,
    pub extra_condition: DynCond,
    pub prerequisite: crate::activation_conditions::OptCond,
    pub prerequisite_satisfied: bool,
    pub cooldown: f64,
    pub cooldown_key: String,
    pub windows: Vec<Region>,
    pub effects: Vec<SkillEffect>,
    /// v75: is_activate_any_skill trigger nodes of this detail's main condition.
    pub anyskill_nodes: Vec<usize>,
}

struct RandomSkillReservation {
    skill_id: String,
    cooldown_key: String,
    perspective: Perspective,
    rarity: SkillRarity,
    effects: Vec<SkillEffect>,
    has_green_tag: bool,
    detail_index: usize,
}

struct RunningSkill {
    detail_index: usize,
    skill_id: String,
    numeric_skill_id: i64,
    perspective: Perspective,
    rarity: SkillRarity,
    timer: Timer,
    delayed: Vec<(SkillEffect, usize)>,
    extend_on_overtake: bool,
    extensions: usize,
    /// v74 retail: own activations recorded since this skill's last turn in the skill pass.
    unapplied: usize,
}

struct ActiveSkill {
    /// v16: deterministic receiver token (was the timer's heap address).
    token: u64,
    skill_id: String,
    perspective: Perspective,
    duration_timer: Timer,
    modifier: f64,
    natural_deceleration: bool,
    effect_type: SkillType,
}

pub type SkillHook = Box<dyn FnMut(&mut RaceSolver, &str, Perspective)>;

#[derive(Default)]
struct SkillScanIndex {
    pending_len: usize,
    prerequisites: Vec<usize>,
    priority_zero: Vec<usize>,
    priority_one: Vec<usize>,
    windows: Vec<SkillScanWindow>,
}

/// Small, immutable fields needed to dismiss an inactive retail detail. Keep
/// the full record (strings, closures, effects and vectors) off that hot path.
#[derive(Clone, Copy)]
struct SkillScanWindow {
    start: f64,
    end: f64,
    numeric_id: i64,
    priority: i32,
    retained: bool,
    observed: bool,
}

fn noop_hook() -> SkillHook {
    Box::new(|_s, _id, _p| {})
}

#[derive(Default, Clone)]
pub struct RaceContext {
    pub team_id: i64,
    pub chara_id: i64,
    pub fans: f64,
    pub wins: f64,
    pub team_stats: Option<[f64; 5]>,
}

pub struct RaceSolver {
    pub(crate) retail: Option<RetailRuntimeState>,
    retail_target_modifiers: crate::retail_physics::AdditiveReceiver,
    retail_accel_modifiers: crate::retail_physics::AdditiveReceiver,
    modifier_token_counter: u64,
    pub race_context: RaceContext,
    pub horse: HorseParameters,
    pub course: &'static CourseData,
    pub hp: HpPolicy,
    pub pacer: Option<Box<RaceSolver>>,
    pub rng: Rule30CARng,
    pub gate_index: i32,
    /// Set once the whole roster's `RaceField` is built. `None` during the
    /// very first ("gate skill") activation round inside `new()`, since no
    /// other horse exists yet at that point.
    pub field_index: Option<usize>,
    pub initial_ability_activations: Vec<(i32, f64)>,

    pub pending_skills: Vec<PendingSkill>,
    skill_scan: SkillScanIndex,
    pub pending_removal: BTreeSet<String>,
    random_any_request: f64,
    random_gold_request: f64,
    retail_initializing: bool,
    retail_stat_values: [Option<f32>; 5],
    pub(crate) random_gold_reservations: BTreeSet<String>,
    random_active_details: BTreeMap<(String, usize), Timer>,
    waiting_skill_cooldowns: BTreeMap<String, (String, usize, f64)>,
    /// Skills sorted by trigger position, so the per-tick scan only looks at
    /// the ones the horse has actually reached instead of re-testing every
    /// unfired skill on all ~1800 frames. Only valid when `pending_removal`
    /// can never be populated -- see `armed_scan`'s construction-time check.
    armed_scan: bool,
    /// Parallel to `pending_skills`: `pending_seq[j]` is the original
    /// insertion-order sequence number of `pending_skills[j]`. Stays sorted
    /// ascending as items are removed (both shrink together), which is what
    /// lets a `seq` be mapped back to its current index via binary search.
    pending_seq: Vec<usize>,
    /// Sequence numbers of currently-armed (trigger.start already reached)
    /// skills, kept sorted ascending; scanned in descending order (nearest
    /// to insertion order last) to match the full-scan's iteration order.
    armed: Vec<usize>,
    /// (trigger_start, seq) for not-yet-armed skills, sorted descending by
    /// trigger_start so `.pop()` yields the nearest one.
    pending_queue: Vec<(f64, usize)>,

    pub skill_detail_events: Vec<(String, usize, f64)>,
    // Per original ability: None is unobserved, Some(empty) is selected nobody.
    // This is selector evidence, not confirmation that every modifier applied.
    pub additional_target_events: Vec<(String, usize, f64, Vec<Option<Vec<usize>>>)>,
    pub skill_target_events: Vec<(String, usize, f64, Vec<Option<Vec<usize>>>)>,
    pub used_skills: BTreeSet<String>,
    skill_cooldowns: BTreeMap<String, f64>,
    running_skills: Vec<RunningSkill>,
    pending_overtakes: usize,
    pub late_corner_overtakes: usize,
    pub phase_overtakes: [usize; 3],
    pub final_corner_overtakes: usize,
    pub overtake_target_time: f64,
    pub passing_targets: Vec<usize>,
    pub overtake_target_no_order_up_time: f64,
    pub overtake_target_no_order_down_time: f64,
    pub activated_green_skills: BTreeSet<String>,
    gorosi_rng: Rule30CARng,
    pace_effect_rng: Rule30CARng,

    pub timers: Vec<Timer>,
    pub accumulatetime: Timer,

    pub gate_roll: i64,
    pub random_lot: i64,
    pub phase: i32,
    next_phase_transition: f64,

    active_target_speed_skills: Vec<ActiveSkill>,
    active_current_speed_skills: Vec<ActiveSkill>,
    active_accel_skills: Vec<ActiveSkill>,
    /// Doc: Lane Move Speed Up (ability type 28) -- not a modifier added on
    /// activation; while active AND the horse moved lane on the PREVIOUS
    /// frame, MoveLaneModifier = sqrt(0.0002*Power) is added to target
    /// speed each such tick (see update_target_speed).
    active_lane_speed_skills: Vec<ActiveSkill>,
    active_aux_skills: Vec<ActiveSkill>,
    pub activate_count: [i32; 3],
    pub skill_activation_observers: RefCell<BTreeMap<usize, (i32, i32)>>,
    /// v75 retail: SkillTriggerIsActivateAnySkill state per node (last value, true count)
    /// and the result of its latest check.
    pub anyskill_observer: RefCell<BTreeMap<usize, (i32, i32)>>,
    pub anyskill_result: RefCell<BTreeMap<usize, bool>>,
    pub activate_count_heal: i32,
    // Activations past the halfway mark of the course -- the DISTANCE half,
    // not a phase; see activation_conditions' activate_count_later_half.
    pub activate_count_later_half: i32,
    pub activate_count_last_frame: i32,
    /// v14 retail: own activations counted during the previous tick (is_activate_any_skill).
    pub activate_count_previous_tick: i32,
    pub activate_total_at_tick_start: i32,
    /// v70 retail is_activate_any_skill: own skill ids activated in the previous tick and so far
    /// in this tick, in activation order.
    pub activated_ids_previous_tick: Vec<i64>,
    pub activated_ids_this_tick: Vec<i64>,
    /// v74: highest skill id whose turn in this tick's skill pass has been taken.
    additional_turn_floor: i64,
    unapplied_running_skills: usize,
    retiring_additional_skills: bool,

    on_skill_activate: SkillHook,
    on_skill_deactivate: SkillHook,

    section_length: f64,
    pub is_pace_down: bool,
    pos_keep_min_threshold: f64,
    pos_keep_max_threshold: f64,
    pos_keep_cooldown: f64,
    pos_keep_check_interval: f64,
    pos_keep_continue_distance: f64,
    pos_keep_effect_end: f64,
    pos_keep_end: f64,
    pos_keep_speed_coef: f64,
    pos_keep_mode: Option<PosKeepMode>,
    pos_keep_effect_exit_distance: f64,
    pos_keep_effect_start: f64,

    modifiers_target_speed: CompensatedAccumulator,
    modifiers_current_speed: CompensatedAccumulator,
    modifiers_accel: CompensatedAccumulator,
    modifiers_one_frame_accel: f64,
    modifiers_special_skill_duration_scaling: f64,
    pub(crate) modifiers_kakari_chance: f64,

    pub start_delay: f64,
    pub pos: f64,
    accel: f64,
    pub current_speed: f64,
    pub(crate) target_speed: f64,
    is_downhill_mode: bool,
    pub is_last_spurt: bool,
    pub actual_last_spurt_start_time: Option<f64>,
    pub actual_last_spurt_start_distance: Option<f64>,

    pub lane: f64,
    lane_target: f64,
    lane_change_speed: f64,
    extra_move_lane: Option<f64>,
    pub blocked_front_timer: Cell<f64>,
    pub blocked_side_timer: Cell<f64>,
    pub blocked_all_timer: Cell<f64>,
    pub infront_near_lane_timer: Cell<f64>,
    pub behind_near_lane_timer: Cell<f64>,
    pub behind_near_lane_set1_timer: Cell<f64>,
    pub moved_lane_this_tick: bool,
    pub move_lane_continue_time: f32,
    pub move_lane_continue_direction: i32,

    pub experimental_later_stamina_mechanics: bool,
    spot_struggle_active: bool,
    /// v8 conserve power accumulation (see update_retail_pace).
    pub(crate) cp_acc: f64,
    pub(crate) cp_clean: crate::conserve_power::ConservePowerState,
    pub(crate) cp_struggle: bool,
    pub(crate) cp_accel_token: Option<u64>,
    pub(crate) cp_kakari: bool,
    pub(crate) cp_activating: bool,
    pub spot_struggle_started: bool,
    spot_struggle_distance_exited: bool,
    spot_struggle_cutoff_latched: bool,
    spot_struggle_timer: Timer,
    spot_struggle_modifier: f64,
    compete_spurt_active: bool,
    compete_spurt_timer: Timer,
    compete_spurt_cooldown: Timer,
    compete_spurt_modifier: f64,
    secure_lead_active: bool,
    secure_lead_timer: Timer,
    secure_lead_cooldown: Timer,
    secure_lead_modifier: f64,
    stamina_keep_active: bool,
    duel_continuous_timer: f64,
    duel_proximity: BTreeMap<usize, f64>,
    duel_target_index: Option<usize>,
    duel_active: bool,
    // Experimental first-assignment duel groups; not recovered server semantics.
    duel_group: Option<usize>,
    pub compete_fight_count: i32,
    duel_timer: Timer,
    duel_target_speed_modifier: f64,
    duel_accel_modifier: f64,

    min_speed: f64,
    start_dash: bool,

    n_hills: usize,
    hill_start: Vec<f64>,
    hill_end: Vec<f64>,
    hill_idx: i64,
    hill_rng: Vec<Rule30CARng>,
    downhill_timer: Timer,
    slope_per: f64,

    base_target_speed: [f64; 3],
    pub last_spurt_speed: f64,
    pub last_spurt_transition: f64,
    pub last_spurt_calc_count: i32,
    pub last_spurt_calc_result: i32,

    pub kakari_start: f64,
    kakari_duration: f64,
    kakari_timer: Timer,
    pub is_kakari: bool,
    pub rushing_mode: u8,
    position_keep_rushing: bool,
    pub temptation_count: i32,

    pub(crate) section_modifier: Vec<f64>,
    base_accel: [f64; 6],


    pub order_band_in_200: Cell<Option<f64>>,
    pub order_band_in_400: Cell<Option<f64>>,
    pub order_band_in_500: Cell<Option<f64>>,
    pub order_band_in_800: Cell<Option<f64>>,
    pub order_band_out_200: Cell<Option<f64>>,
    pub order_band_out_400: Cell<Option<f64>>,
    pub order_band_out_500: Cell<Option<f64>>,
    pub order_band_out_700: Cell<Option<f64>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PosKeepMode {
    PaceDown,
    PaceUp,
    PaceUpEx,
    SpeedUp,
    Overtake,
}

impl RaceSolver {
    fn add_stat_modifier(&mut self, index: usize, modifier: f64) {
        let retail = self.retail_initializing || self.retail.is_some();
        let field = match index {
            0 => &mut self.horse.speed, 1 => &mut self.horse.stamina,
            2 => &mut self.horse.power, 3 => &mut self.horse.guts,
            4 => &mut self.horse.wisdom, _ => unreachable!(),
        };
        if retail {
            let value = self.retail_stat_values[index].unwrap_or(*field as f32) + modifier as f32;
            self.retail_stat_values[index] = Some(value);
            *field = value.clamp(1.0, 2000.0) as f64;
        } else {
            *field = (*field + modifier).max(1.0);
        }
    }

    pub(crate) fn final_corner_half_value(&self) -> Option<bool> {
        let geometry = self.retail.as_ref()?.config.event_geometry.as_ref()?;
        let (start, end) = geometry.final_corner_half?;
        Some(start <= self.pos as f32 && self.pos as f32 <= end)
    }
    pub(crate) fn corner_value(&self, final_flag: bool) -> i32 {
        self.retail.as_ref().map_or(0, |r| if final_flag {r.final_corner as i32} else {r.corner})
    }
    pub(crate) fn slope_type(&self) -> i32 {
        if self.slope_per > 0.0 { 1 } else if self.slope_per < 0.0 { 2 } else { 0 }
    }
    pub fn has_active_skill(&self, skill_id: &str, perspective: Perspective) -> bool {
        self.active_target_speed_skills.iter()
            .chain(self.active_current_speed_skills.iter())
            .chain(self.active_accel_skills.iter())
            .chain(self.active_lane_speed_skills.iter())
            .chain(self.active_aux_skills.iter())
            .any(|s| s.skill_id == skill_id && s.perspective == perspective)
    }

    pub fn remaining_skill_duration(&self, skill_id: &str, perspective: Perspective) -> f64 {
        self.active_target_speed_skills.iter()
            .chain(self.active_current_speed_skills.iter())
            .chain(self.active_accel_skills.iter())
            .chain(self.active_lane_speed_skills.iter())
            .chain(self.active_aux_skills.iter())
            .filter(|s| s.skill_id == skill_id && s.perspective == perspective && s.duration_timer.get().is_finite())
            .map(|s| (-s.duration_timer.get()).max(0.0))
            .fold(0.0, f64::max)
    }

    pub fn set_starting_lane(&mut self, lane: f64) {
        self.lane = lane;
        self.lane_target = lane;
    }

    pub fn is_spot_struggling(&self) -> bool {
        self.spot_struggle_active
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        horse: HorseParameters,
        course: &'static CourseData,
        mut rng: Rule30CARng,
        skills: Vec<PendingSkill>,
        hp: HpPolicy,
        pacer: Option<Box<RaceSolver>>,
        on_skill_activate: Option<SkillHook>,
        on_skill_deactivate: Option<SkillHook>,
        gate_index: i32,
        retail_initializing: bool,
    ) -> RaceSolver {
        let armed_scan = !skills.iter().any(|sk| sk.effects.iter().any(|ef| matches!(ef.effect_type, SkillType::ActivateRandomGold | SkillType::ActivateRandomSkill)));
        let pending_seq: Vec<usize> = (0..skills.len()).collect();
        let mut pending_queue: Vec<(f64, usize)> = skills.iter().enumerate().map(|(seq, sk)| (sk.trigger.start, seq)).collect();
        // descending: pop() yields the nearest
        pending_queue.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let gorosi_rng = Rule30CARng::new(rng.int32(), 0);
        let pace_effect_rng = Rule30CARng::new(rng.int32(), 0);

        let strategy = horse.strategy;
        let distance = course.distance;

        let mut solver = RaceSolver {
            retail: None,
            retail_target_modifiers: Default::default(),
            retail_accel_modifiers: Default::default(),
            modifier_token_counter: 0,
            horse,
            course,
            hp,
            pacer,
            rng,
            gate_index,
            field_index: None,
            initial_ability_activations: Vec::new(),
            race_context: Default::default(),

            pending_skills: skills,
            skill_scan: SkillScanIndex::default(),
            pending_removal: BTreeSet::new(),
            random_any_request: 0.0,
            random_gold_request: 0.0,
            retail_initializing,
            retail_stat_values: [None; 5],
            random_gold_reservations: BTreeSet::new(),
            random_active_details: BTreeMap::new(),
            waiting_skill_cooldowns: BTreeMap::new(),
            armed_scan,
            pending_seq,
            armed: Vec::new(),
            pending_queue,

            skill_detail_events: Vec::new(),
            skill_target_events: Vec::new(),
            additional_target_events: Vec::new(),
            used_skills: BTreeSet::new(),
            skill_cooldowns: BTreeMap::new(), running_skills: Vec::new(),
            pending_overtakes: 0, late_corner_overtakes: 0,
            phase_overtakes: [0; 3], final_corner_overtakes: 0,
            overtake_target_time: 0.0, overtake_target_no_order_up_time: 0.0,
            passing_targets: Vec::new(),
            overtake_target_no_order_down_time: 0.0,
            activated_green_skills: BTreeSet::new(),
            gorosi_rng,
            pace_effect_rng,

            timers: Vec::new(),
            accumulatetime: new_timer_value(0.0), // placeholder, replaced below

            gate_roll: 0,
            random_lot: 0,
            phase: 0,
            next_phase_transition: crate::course_data::phase_start(distance, 1),

            active_target_speed_skills: Vec::new(),
            active_current_speed_skills: Vec::new(),
            active_accel_skills: Vec::new(),
            active_lane_speed_skills: Vec::new(),
            active_aux_skills: Vec::new(),
            activate_count: [0, 0, 0],
            skill_activation_observers: RefCell::new(BTreeMap::new()),
            anyskill_observer: RefCell::new(BTreeMap::new()),
            anyskill_result: RefCell::new(BTreeMap::new()),
            activate_count_heal: 0,
            activate_count_later_half: 0,
            activate_count_last_frame: 0,
            activate_count_previous_tick: 0,
            activate_total_at_tick_start: 0,
            activated_ids_previous_tick: Vec::new(),
            activated_ids_this_tick: Vec::new(),
            additional_turn_floor: i64::MIN,
            unapplied_running_skills: 0,
            retiring_additional_skills: false,

            on_skill_activate: on_skill_activate.unwrap_or_else(noop_hook),
            on_skill_deactivate: on_skill_deactivate.unwrap_or_else(noop_hook),

            section_length: distance / 24.0,
            is_pace_down: false,
            pos_keep_min_threshold: position_keep_min_threshold(strategy, distance),
            pos_keep_max_threshold: position_keep_max_threshold(strategy, distance),
            pos_keep_cooldown: 0.0,
            pos_keep_check_interval: 0.0,
            pos_keep_continue_distance: (distance / 24.0).floor() * if strategy == Strategy::Oonige { 3.0 } else { 1.0 },
            pos_keep_effect_end: 0.0,
            pos_keep_end: (distance / 24.0).floor() * 10.0,
            pos_keep_speed_coef: 1.0,
            pos_keep_mode: None,
            pos_keep_effect_exit_distance: 0.0,
            pos_keep_effect_start: 0.0,

            modifiers_target_speed: CompensatedAccumulator::new(0.0),
            modifiers_current_speed: CompensatedAccumulator::new(0.0),
            modifiers_accel: CompensatedAccumulator::new(0.0),
            modifiers_one_frame_accel: 0.0,
            modifiers_special_skill_duration_scaling: 1.0,
            modifiers_kakari_chance: 0.0,

            start_delay: 0.0,
            pos: 0.0,
            accel: 0.0,
            current_speed: 3.0,
            target_speed: 0.85 * base_speed(course),
            is_downhill_mode: false,
            is_last_spurt: false,
            actual_last_spurt_start_time: None,
            actual_last_spurt_start_distance: None,

            lane: gate_index as f64 * (1.0 / 18.0),
            lane_target: 0.0,
            lane_change_speed: 0.0,
            extra_move_lane: None,
            blocked_front_timer: Cell::new(0.0),
            blocked_side_timer: Cell::new(0.0),
            blocked_all_timer: Cell::new(0.0),
            infront_near_lane_timer: Cell::new(0.0),
            behind_near_lane_timer: Cell::new(0.0),
            behind_near_lane_set1_timer: Cell::new(0.0),
            moved_lane_this_tick: false,
            move_lane_continue_time: 0.,
            move_lane_continue_direction: 0,

            experimental_later_stamina_mechanics: false,
            spot_struggle_active: false,
            cp_acc: 0.0,
            cp_clean: Default::default(),
            cp_struggle: false,
            cp_accel_token: None,
            cp_kakari: false,
            cp_activating: false,
            spot_struggle_started: false,
            spot_struggle_distance_exited: false,
            spot_struggle_cutoff_latched: false,
            spot_struggle_timer: new_timer_value(0.0),
            spot_struggle_modifier: 0.0,
            compete_spurt_active: false,
            compete_spurt_timer: new_timer_value(0.0),
            compete_spurt_cooldown: new_timer_value(0.0),
            compete_spurt_modifier: 0.0,
            secure_lead_active: false,
            secure_lead_timer: new_timer_value(0.0),
            secure_lead_cooldown: new_timer_value(0.0),
            secure_lead_modifier: 0.0,
            stamina_keep_active: false,
            duel_continuous_timer: 0.0,
            duel_proximity: BTreeMap::new(),
            duel_target_index: None,
            duel_active: false,
            duel_group: None,
            compete_fight_count: 0,
            duel_timer: new_timer_value(0.0),
            duel_target_speed_modifier: 0.0,
            duel_accel_modifier: 0.0,

            min_speed: 0.0,
            start_dash: true,

            n_hills: 0,
            hill_start: Vec::new(),
            hill_end: Vec::new(),
            hill_idx: -1,
            hill_rng: Vec::new(),
            downhill_timer: new_timer_value(0.0),
            slope_per: 0.0,

            base_target_speed: [0.0; 3],
            last_spurt_speed: 0.0,
            last_spurt_transition: -1.0,
            last_spurt_calc_count: 0,
            last_spurt_calc_result: 8,

            kakari_start: 0.0,
            kakari_duration: 0.0,
            kakari_timer: new_timer_value(0.0),
            is_kakari: false,
            rushing_mode: 0,
            position_keep_rushing: false,
            temptation_count: 0,

            section_modifier: Vec::new(),
            base_accel: [0.0; 6],


            order_band_in_200: Cell::new(None),
            order_band_in_400: Cell::new(None),
            order_band_in_500: Cell::new(None),
            order_band_in_800: Cell::new(None),
            order_band_out_200: Cell::new(None),
            order_band_out_400: Cell::new(None),
            order_band_out_500: Cell::new(None),
            order_band_out_700: Cell::new(None),
        };

        solver.accumulatetime = solver.new_timer(0.0);

        // Competition-mechanics timers (see each `update_*` method below):
        // must be registered via `new_timer()` -- not left as the bare
        // placeholder `Cell` the struct literal above gives them -- so
        // `step()`'s per-frame `for tm in &self.timers` loop actually
        // advances them. Without this they never tick, so a timer-based
        // exit condition (e.g. `spot_struggle_timer.get() >= 0.0`) never
        // fires and the mechanic never turns back off once triggered.
        solver.spot_struggle_timer = solver.new_timer(0.0);
        solver.compete_spurt_timer = solver.new_timer(0.0);
        solver.compete_spurt_cooldown = solver.new_timer(0.0);
        solver.secure_lead_timer = solver.new_timer(0.0);
        solver.secure_lead_cooldown = solver.new_timer(0.0);
        solver.duel_timer = solver.new_timer(0.0);

        // Fields set from `rng`, matching Python's exact draw order.
        // bit of a hack, see RaceSolver.ts's comment on gateRoll: n%k is
        // uniformly distributed for a random n only when n_max === k-1 (mod
        // k) for every k; the smallest such n_max for k in [1,18] is
        // lcm(1..18)-1, so draw up to lcm(1..18).
        solver.gate_roll = solver.rng_uniform(12252240.0) as i64;
        solver.random_lot = solver.rng_uniform(100.0) as i64;

        // must come before the first round of skill activations so concen
        // etc can modify it
        solver.start_delay = 0.1 * solver.rng.random();
        if let Some(p) = solver.pacer.as_mut() {
            p.start_delay = 0.0;
        }

        // gate skills (before minSpeed: green skills can modify guts).
        // Must run -- and `init_hills()` below must run -- before the
        // kakari/section_modifier rng draws further down: this is the exact
        // rng-consumption order race_solver.py's `__init__` uses, and this
        // whole block only exists to keep the two engines' rng streams in
        // lockstep (init_hills() draws 2 * len(course.slopes) rng.int32()
        // calls to seed hill_rng, plus a downhill_check() roll per hill
        // starting at the gate).
        solver.process_skill_activations(None);
        // v24: the gate check runs twice (both at t=0). A skill that counts same-frame
        // activations of skills later in its list fires in the repeat: official t=0 order is
        // Concentration, Early Lead, Runaway, Groundwork (activate_count_start>=3), although
        // Groundwork precedes Runaway in the list. The repeat keeps the first check's
        // random-gold draw and adds to its activation count.
        if solver.retail_initializing {
            let reservations = solver.random_gold_reservations.clone();
            let counted = solver.activate_count_last_frame;
            solver.process_skill_priority(None, None);
            solver.random_gold_reservations = reservations;
            solver.activate_count_last_frame += counted;
        }
        solver.retail_initializing = false;
        solver.min_speed = 0.85 * base_speed(solver.course) + (200.0 * solver.horse.guts).sqrt() * 0.001;
        solver.start_dash = true;
        solver.modifiers_accel.add(24.0); // start dash accel

        // after skill activations, since greens can affect downhill check if
        // starting on a hill
        solver.init_hills();

        // also must come after the first round of skill activations
        solver.base_target_speed = [base_target_speed(&solver.horse, solver.course, 0), base_target_speed(&solver.horse, solver.course, 1), base_target_speed(&solver.horse, solver.course, 2)];
        solver.last_spurt_speed = last_spurt_speed(&solver.horse, solver.course);
        solver.last_spurt_transition = -1.0;

        // roll for section first, then check, so rng always advances twice
        // regardless of outcome (avoids data-dependent rng advancement)
        solver.kakari_start = (2.0 + solver.rng_uniform(7.0)) * solver.section_length;
        if solver.rng.random() > (0.65 / (0.1 * solver.horse.wisdom + 1.0).log10()).powi(2) + solver.modifiers_kakari_chance {
            solver.kakari_start = solver.course.distance + 9999.0;
        }
        let r1 = solver.rng.random();
        let r2 = solver.rng.random();
        let r3 = solver.rng.random();
        let seq = [0.0, r1, r2, r3, 1.0];
        let idx = seq.iter().position(|&x| x > 0.45).unwrap();
        solver.kakari_duration = 3.0 * idx as f64;
        solver.kakari_timer = solver.new_timer(0.0);
        solver.is_kakari = false;
        solver.temptation_count = 0;

        solver.section_modifier = Vec::with_capacity(25);
        let max_v = solver.horse.wisdom / 5500.0 * (solver.horse.wisdom * 0.1).log10();
        let section_base_speed = base_speed(solver.course);
        for _ in 0..24 {
            let factor = (max_v - 0.65 + solver.rng.random() * 0.65) / 100.0;
            solver.section_modifier.push(section_base_speed * factor);
        }
        solver.section_modifier.push(0.0); // tick after race is done / one uma runs off the end


        solver.hp.init(&solver.horse);

        solver.base_accel = [0, 1, 2, 0, 1, 2].iter().enumerate().map(|(i, &p)| {
            calc_base_accel(if i > 2 { UPHILL_BASE_ACCEL } else { BASE_ACCEL }, &solver.horse, p)
        }).collect::<Vec<f64>>().try_into().unwrap();

        solver
    }

    pub(crate) fn rng_uniform(&mut self, upper: f64) -> f64 {
        if let Some(r) = &self.retail {
            r.config.random.borrow_mut().integer(0, upper as i32) as f64
        } else { self.rng.uniform(upper) as f64 }
    }

    // Calling `self.on_skill_activate(self, ...)` directly would borrow
    // `self.on_skill_activate` mutably (required for `FnMut`) while also
    // passing `self` as `&mut RaceSolver` -- two overlapping mutable
    // borrows of `self`. Swapping the hook out for a no-op placeholder for
    // the duration of the call sidesteps this with no behavioral
    // difference (the hook itself never re-enters the solver it was called
    // from, so it never observes the temporary swap).
    fn call_activate_hook(&mut self, skill_id: &str, perspective: Perspective) {
        let mut hook = std::mem::replace(&mut self.on_skill_activate, noop_hook());
        hook(self, skill_id, perspective);
        self.on_skill_activate = hook;
    }

    fn call_deactivate_hook(&mut self, skill_id: &str, perspective: Perspective) {
        let mut hook = std::mem::replace(&mut self.on_skill_deactivate, noop_hook());
        hook(self, skill_id, perspective);
        self.on_skill_deactivate = hook;
    }

    // -- setup helpers --------------------------------------------------

    // Negative skill duration is permanent in SkillParamModifierBase.
    fn new_skill_timer(&mut self, duration: f64) -> Timer {
        let duration=if self.retail.is_some() {duration as f32 as f64} else {duration};
        self.new_timer(if duration < 0.0 { f64::NEG_INFINITY } else { -duration })
    }

    /// v16: skill receiver tokens are a per-solver counter instead of heap
    /// addresses. Values 1..=4 are reserved for the fixed competition
    /// modifiers (spot struggle, compete, secure lead, duel), so start at 1<<32.
    fn next_modifier_token(&mut self) -> u64 {
        self.modifier_token_counter += 1;
        (1u64 << 32) + self.modifier_token_counter
    }

    fn new_timer(&mut self, t: f64) -> Timer {
        let tm = new_timer_value(t);
        self.timers.push(tm.clone());
        tm
    }

    /// Stop ticking a finished skill's duration timer. `step()` advances
    /// every timer in `self.timers` each frame, and nothing reads a timer
    /// again once its skill has been dropped from the active-skill lists,
    /// so this is purely bookkeeping.
    fn release_timer(&mut self, tm: &Timer) {
        if let Some(pos) = self.timers.iter().position(|t| Rc::ptr_eq(t, tm)) {
            self.timers.remove(pos);
        }
    }

    fn init_hills(&mut self) {
        // slopes must be sorted by start for the sequential-consumption
        // logic below
        self.n_hills = self.course.slopes.len();
        self.hill_start = self.course.slopes.iter().map(|s| s.start).rev().collect();
        self.hill_end = self.course.slopes.iter().map(|s| s.start + s.length).rev().collect();
        self.hill_idx = -1;

        // separate rng per hill so a downhill proc on one hill doesn't
        // affect how many times the rng is rolled on a later hill
        self.hill_rng = self.course.slopes.iter().map(|_| Rule30CARng::new(self.rng.int32(), self.rng.int32())).collect();
        self.downhill_timer = self.new_timer(0.0);

        if self.hill_start.last() == Some(&0.0) {
            self.hill_idx = 0;
            self.slope_per = self.course.slopes[0].slope;
            self.downhill_timer.set(0.0);
            let roll = self.hill_rng[0].random();
            self.downhill_check(roll);
            self.hill_start.pop();
        } else {
            self.slope_per = 0.0;
        }
    }

    // -- per-tick ---------------------------------------------------------

    fn get_max_speed(&self) -> f64 {
        if self.start_dash {
            self.target_speed.min(0.85 * base_speed(self.course))
        } else if self.current_speed + self.modifiers_one_frame_accel > self.target_speed {
            9999.0 // allow decelerating if targetSpeed drops
        } else {
            self.target_speed
        }
    }

    pub fn step(&mut self, dt: f64, field: Option<&RaceField>) {
        let dt = self.step_forward(dt, field);
        if dt > 0.0 {
            self.step_lateral(dt, field);
            self.step_field_conditions(dt, field);
        }
    }

    /// Return the effective moving time after any partial gate delay.
    pub(crate) fn step_forward(&mut self, mut dt: f64, field: Option<&RaceField>) -> f64 {
        if self.retail.is_some() { return self.step_forward_retail(dt as f32, field); }
        if self.accumulatetime.get() < self.start_delay {
            let partial_frame = self.start_delay - self.accumulatetime.get();
            if partial_frame < dt {
                for tm in &self.timers {
                    tm.set(tm.get() + partial_frame);
                }
                dt -= partial_frame;
            } else {
                for tm in &self.timers {
                    tm.set(tm.get() + dt);
                }
                return 0.0;
            }
        }

        if self.pos < self.pos_keep_end {
            if let Some(pacer) = self.pacer.as_mut() {
                pacer.step(dt, field);
            }
        }

        let halfv = (self.current_speed + 0.5 * dt * self.accel).min(self.get_max_speed());
        let displacement = halfv + self.modifiers_current_speed.acc + self.modifiers_current_speed.err;
        self.pos += displacement * dt;
        let hp_ctx = HpTickCtx { is_pace_down: self.is_pace_down, is_downhill_mode: self.is_downhill_mode, is_kakari: self.is_kakari, phase: self.phase, current_speed: self.current_speed,
            is_compete_top:self.spot_struggle_active,is_runaway:self.horse.strategy==Strategy::Oonige };
        self.hp.tick(hp_ctx, dt);
        for tm in &self.timers {
            tm.set(tm.get() + dt);
        }
        self.update_hills();
        self.update_phase();
        self.process_skill_activations(field);
        self.update_kakari();
        self.update_position_keep(dt, field);
        self.update_last_spurt_state();
        self.update_target_speed();
        self.apply_forces();
        self.current_speed = (halfv + 0.5 * dt * self.accel + self.modifiers_one_frame_accel).min(self.get_max_speed());
        if !self.start_dash && self.current_speed < self.min_speed {
            self.current_speed = self.min_speed;
        } else if self.start_dash && self.current_speed >= 0.85 * base_speed(self.course) {
            self.start_dash = false;
            self.modifiers_accel.add(-24.0);
        }
        self.modifiers_one_frame_accel = 0.0;
        dt
    }

    pub(crate) fn step_lateral(&mut self, dt: f64, field: Option<&RaceField>) {
        self.update_blocking(dt, field);
        self.update_lane(dt, field);
    }

    pub(crate) fn step_field_conditions(&mut self, dt: f64, field: Option<&RaceField>) {
        self.update_near_lane_timers(dt, field);
        self.update_competition_mechanics(dt, field);
    }

    pub(crate) fn update_blocking(&mut self, dt: f64, field: Option<&RaceField>) {
        let field = match field {
            Some(f) => f,
            None => {
                self.blocked_front_timer.set(0.0);
                self.blocked_side_timer.set(0.0);
                self.blocked_all_timer.set(0.0);
                return;
            }
        };
        let i = self.field_index.unwrap();
        let blocker = field.front_blocker(i, self.pos, self.lane);
        if let Some((other_idx, dist_gap)) = blocker {
            let other_speed = field.solvers[other_idx].borrow().current_speed;
            let cap = (0.988 + 0.012 * (dist_gap / 2.0)) * other_speed;
            if self.retail.is_none() { self.current_speed = self.current_speed.min(cap); }
            self.blocked_front_timer.set(self.blocked_front_timer.get() + dt);
        } else {
            self.blocked_front_timer.set(0.0);
        }
        let (inner_blocked, outer_blocked) = field.side_room(i, self.pos, self.lane);
        let side_blocked = inner_blocked || outer_blocked;
        self.blocked_side_timer.set(if side_blocked { self.blocked_side_timer.get() + dt } else { 0.0 });
        let all_blocked = blocker.is_some() && side_blocked;
        self.blocked_all_timer.set(if all_blocked { self.blocked_all_timer.get() + dt } else { 0.0 });
    }

    /// The retail field has just sampled its front blocker. No movement or
    /// callbacks intervene before these counters, so reuse that exact sample.
    pub(crate) fn update_retail_blocking(&mut self, dt: f64, side: bool) {
        let front = self.retail.as_ref().unwrap().front_blocker.is_some();
        self.blocked_front_timer.set(if front { self.blocked_front_timer.get() + dt } else { 0. });
        self.blocked_side_timer.set(if side { self.blocked_side_timer.get() + dt } else { 0. });
        self.blocked_all_timer.set(if front && side { self.blocked_all_timer.get() + dt } else { 0. });
    }

    fn update_lane_target(&mut self, field: &RaceField, others: &[crate::lane_selection::Row]) {
        let i = self.field_index.unwrap();
        if !self.hp.has_remaining_hp() {
            self.lane_target = self.lane;
            return;
        }
        if self.is_pace_down {
            self.lane_target = 0.18;
            return;
        }
        if let Some(last_corner) = self.course.corners.last() {
            if self.extra_move_lane.is_none() && self.pos >= last_corner.start {
                let lane_distance = self.lane;
                self.extra_move_lane = Some((lane_distance / 0.1).max(0.0).min(1.0) * 0.5 + self.rng.random() * 0.1);
            }
        }
        let (inner_blocked, outer_blocked) = field.side_room(i, self.pos, self.lane);
        if let Some(passing) = crate::lane_selection::passing_lane(self.pos,self.lane,
                self.phase,self.course.max_lane,self.visible_distance(),others,&self.passing_targets,self.lane_preference_flags()) {
            self.lane_target = passing;
            return;
        }
        // Preserve a clear escape. Final-corner lane is only an outward
        // preference on the final straight, never a fixed-lane constraint.
        let blocked_direction = if self.lane_target > self.lane { outer_blocked } else { inner_blocked };
        if (self.lane_target - self.lane).abs() > 0.5 * HORSE_LANE && !blocked_direction {
            return;
        }
        let on_final_straight = self.course.straights.last().is_some_and(|s| self.pos >= s.start);
        if on_final_straight && self.extra_move_lane.is_some_and(|extra| extra > self.lane) && !outer_blocked {
            self.lane_target = self.extra_move_lane.unwrap();
        } else if self.phase < 2 && !inner_blocked {
            self.lane_target = (self.lane - 0.05).max(0.0);
        } else {
            self.lane_target = self.lane;
        }
    }

    pub(crate) fn lane_preference_flags(&self) -> u8 {
        let outside=self.active_aux_skills.iter().any(|s|s.effect_type==SkillType::PreferOutsideLane);
        let inside=self.active_aux_skills.iter().any(|s|s.effect_type==SkillType::PreferInsideLane);
        (if outside {1} else {0}) | (if inside {2} else {0})
    }

    pub(crate) fn visible_distance(&self) -> f64 {
        // Stacked vision reductions can remove the entire visible corridor.
        // Front blocking still applies, but passing search must not see a
        // nearby blocker through the generic controller's positive floor.
        let minimum = if self.retail.is_some() { 0.0 } else { 0.1 };
        (20.0 + self.active_aux_skills.iter().filter(|s| s.effect_type == SkillType::Vision)
            .map(|s| s.modifier).sum::<f64>()).max(minimum)
    }

    pub(crate) fn apply_external_effect(&mut self, id: &str, rarity: SkillRarity, effect: SkillEffect, duration: f64, own: bool, field: &RaceField) {
        self.apply_activation(id, if own { Perspective::Self_ } else { Perspective::Other }, rarity,
                              &[effect], false, Some(field), true, Some(duration), false, 0);
    }

    fn refresh_stat_effects(&mut self) {
        self.min_speed = 0.85 * base_speed(self.course) + (200.0 * self.horse.guts).sqrt() * 0.001;
        self.base_target_speed = [0, 1, 2].map(|p| base_target_speed(&self.horse, self.course, p));
        self.last_spurt_speed = last_spurt_speed(&self.horse, self.course);
        self.base_accel = [0, 1, 2, 0, 1, 2].iter().enumerate().map(|(i, &p)| {
            calc_base_accel(if i > 2 { UPHILL_BASE_ACCEL } else { BASE_ACCEL }, &self.horse, p)
        }).collect::<Vec<f64>>().try_into().unwrap();
        let spent = match &self.hp { HpPolicy::Game(g) => g.max_hp - g.hp, _ => 0.0 };
        self.hp.init(&self.horse);
        self.hp.spend(spent);
    }

    fn update_lane(&mut self, dt: f64, field: Option<&RaceField>) {
        let field = match field {
            Some(f) => f,
            None => return,
        };
        let i = self.field_index.unwrap();
        let others: Vec<crate::lane_selection::Row> = (0..field.n).filter(|&j| j != i).filter_map(|j| {
            let s = field.solvers[j].borrow();
            if s.pos < self.course.distance { Some((s.pos,s.lane,s.current_speed,s.target_speed)) } else { None }
        }).collect();
        self.passing_targets = crate::lane_selection::collect_targets(self.pos,self.lane,
            self.current_speed,self.target_speed,self.visible_distance(),&others);
        // The null controller retains its target even with a lane skill.
        if self.course.empty_in_move {
            if let Some(fixed) = self.active_aux_skills.iter().rev().find(|s| s.effect_type == SkillType::SetTargetLane) {
                self.lane_target = fixed.modifier;
            } else { self.update_lane_target(field, &others); }
        }
        self.lane_target = self.lane_target.max(0.0).min(self.course.max_lane);
        let lane_distance = (self.lane_target - self.lane).abs();
        if lane_distance < 1e-9 {
            self.lane_change_speed = 0.0;
            self.moved_lane_this_tick = false;
            self.move_lane_continue_time = 0.;
            return;
        }
        let lane_modifier = if self.phase >= 2 { 1.0 + field.order_of(i) as f64 * 0.01 }
            else if self.pos <= self.course.first_move_lane_point { 1.0 + self.lane / self.course.max_lane.max(1e-9) * 0.05 }
            else { 1.0 };
        let previous_direction = self.move_lane_continue_direction;
        self.move_lane_continue_direction = (self.lane_target >= self.lane) as i32;
        let target_lane_change_speed = 0.02 * (0.3 + 0.001 * self.horse.power.clamp(1.0,2000.0)) * lane_modifier;
        let lane_change_accel = 0.02 * 1.5;
        if self.lane_change_speed < target_lane_change_speed {
            self.lane_change_speed = target_lane_change_speed.min(self.lane_change_speed + lane_change_accel * dt);
        } else if self.lane_change_speed > target_lane_change_speed {
            self.lane_change_speed = target_lane_change_speed.max(self.lane_change_speed - lane_change_accel * dt);
        }
        let mut actual_speed = (self.lane_change_speed + self.active_lane_speed_skills.iter()
            .map(|skill| skill.modifier).sum::<f64>()).max(0.0).min(0.6);
        let direction = if self.lane_target > self.lane { 1.0 } else { -1.0 };
        if direction < 0.0 { actual_speed *= 1.0 + self.lane; }
        let step = actual_speed * dt * direction;
        let previous_lane = self.lane;
        if step.abs() >= lane_distance {
            self.lane = self.lane_target;
            self.lane_change_speed = 0.0;
        } else {
            self.lane += step;
        }
        self.lane = self.lane.max(0.0).min(self.course.max_lane);
        // Sweep the field's existing side-block envelope, also for forced
        // final-corner targets. Current side_room flags alone miss crossings
        // made by a large lateral step. Already-overlapping horses may separate.
        for j in 0..field.n {
            if j == i {
                continue;
            }
            let other = field.solvers[j].borrow();
            if other.pos >= self.course.distance || (other.pos - self.pos).abs() >= 1.05 {
                continue;
            }
            if previous_lane < other.lane && self.lane > previous_lane {
                self.lane = self.lane.min(previous_lane.max(other.lane - 2.0 * HORSE_LANE));
            } else if previous_lane > other.lane && self.lane < previous_lane {
                self.lane = self.lane.max(previous_lane.min(other.lane + 2.0 * HORSE_LANE));
            }
        }
        self.moved_lane_this_tick = (self.lane - previous_lane).abs() > 1e-9;
        if self.moved_lane_this_tick && previous_direction == self.move_lane_continue_direction {
            self.move_lane_continue_time += dt as f32;
        }
    }

    fn update_near_lane_timers(&mut self, dt: f64, field: Option<&RaceField>) {
        let field = match field {
            Some(f) => f,
            None => {
                self.infront_near_lane_timer.set(0.0);
                self.behind_near_lane_timer.set(0.0);
                self.behind_near_lane_set1_timer.set(0.0);
                return;
            }
        };
        let i = self.field_index.unwrap();
        if field.order_of(i) != field.previous_order_of(i) {
            self.infront_near_lane_timer.set(0.0);
            self.behind_near_lane_timer.set(0.0);
            self.behind_near_lane_set1_timer.set(0.0);
            return;
        }
        let ahead_gap = field.distance_to_ahead(i, self.pos);
        let ahead_lane_gap = field.lane_gap_to_ahead(i, self.lane);
        let near_ahead = ahead_gap.is_some() && ahead_gap.unwrap().abs() <= 2.5 && ahead_lane_gap.unwrap().abs() <= 0.0556_f32 as f64;
        self.infront_near_lane_timer.set(if near_ahead { self.infront_near_lane_timer.get() + dt } else { 0.0 });

        let behind_gap = field.distance_to_behind(i, self.pos);
        let behind_lane_gap = field.lane_gap_to_behind(i, self.lane);
        let near_behind = behind_gap.is_some() && behind_gap.unwrap().abs() <= 2.5 && behind_lane_gap.unwrap().abs() <= 0.0556_f32 as f64;
        self.behind_near_lane_timer.set(if near_behind { self.behind_near_lane_timer.get() + dt } else { 0.0 });
        // Global Skill.BehindNearParamArray[0], distinct from the narrow timer.
        let near_set1 = behind_gap.is_some() && behind_gap.unwrap().abs() <= 5.0
            && behind_lane_gap.unwrap().abs() <= 0.15_f32 as f64;
        self.behind_near_lane_set1_timer.set(if near_set1 { self.behind_near_lane_set1_timer.get() + dt } else { 0.0 });
    }

    fn update_kakari(&mut self) {
        if self.temptation_count == 0 && self.pos >= self.kakari_start {
            let roll = if strategy_matches(self.horse.strategy, Strategy::Nige) { 0 }
                else { self.rng.uniform(100.0) };
            self.rushing_mode = crate::rushing::select_mode(self.horse.strategy, roll);
            self.is_kakari = true;
            self.temptation_count = 1;
            self.kakari_timer.set(-self.kakari_duration);
            self.call_activate_hook("kakari", Perspective::Self_);
        } else if self.is_kakari && self.kakari_timer.get() >= 0.0 {
            self.is_kakari = false;
            self.rushing_mode = 0;
            self.call_deactivate_hook("kakari", Perspective::Self_);
        }
    }

    fn update_position_keep(&mut self, dt: f64, field: Option<&RaceField>) {
        if field.is_none() { return; }
        if self.position_keep_rushing != self.is_kakari {
            self.position_keep_rushing = self.is_kakari;
            self.clear_position_keep();
            self.pos_keep_cooldown = 0.0;
            self.pos_keep_check_interval = 0.0;
        }
        let was_active = self.pos_keep_mode.is_some();
        if was_active && self.pos >= self.pos_keep_effect_end {
            self.clear_position_keep();
            return;
        }
        if !was_active {
            if self.pos < 0.0 || self.pos >= self.pos_keep_end { return; }
            // Client CheckStart freezes both clocks while active; cooldown
            // runs before the interval. Every attempted check consumes 2s.
            if self.pos_keep_cooldown > 0.0 {
                self.pos_keep_cooldown -= dt;
                if self.pos_keep_cooldown > 0.0 { return; }
            }
            if self.pos_keep_check_interval > 0.0 {
                self.pos_keep_check_interval -= dt;
                if self.pos_keep_check_interval > 0.0 { return; }
            }
            self.pos_keep_check_interval = 2.0;
        }
        let style = crate::rushing::pacing_style(self.horse.strategy, if self.is_kakari {self.rushing_mode} else {0});
        let f = field.unwrap();
        let i = self.field_index.unwrap();
        if self.pos_keep_mode == Some(PosKeepMode::PaceUpEx) {
            if !f.later_style_ahead(i, self.horse.strategy) { self.clear_position_keep(); }
        } else if !was_active && (
            (strategy_matches(style, Strategy::Nige) && !self.is_kakari
                && f.later_style_ahead(i, self.horse.strategy))
            || (!strategy_matches(style, Strategy::Nige)
                && f.pacemaker_is_later_style_ahead(i, self.horse.strategy))) {
            self.pos_keep_mode = Some(PosKeepMode::PaceUpEx);
            self.is_pace_down = false;
            self.pos_keep_speed_coef = 2.0;
        } else if strategy_matches(style, Strategy::Nige) {
            self.update_position_keep_front_runner(dt, field);
        } else {
            self.update_position_keep_non_nige(dt, field);
        }
        if was_active && self.pos_keep_mode.is_none() {
            self.clear_position_keep();
        } else if !was_active && self.pos_keep_mode.is_some() {
            self.pos_keep_effect_end = self.pos + self.pos_keep_continue_distance;
        }
    }

    fn clear_position_keep(&mut self) {
        self.pos_keep_mode = None;
        self.is_pace_down = false;
        self.pos_keep_speed_coef = 1.0;
        self.pos_keep_cooldown = 1.0;
    }

    fn position_keep_wisdom_check(&mut self, coef: f64) -> bool {
        if self.is_kakari { return true; }
        let pct = (coef * (self.horse.wisdom.max(1.0) * 0.1).log10()).max(0.0);
        self.rng.random() < pct.min(100.0) / 100.0
    }

    /// Legacy hazard approximation for competition checks only.
    fn roll_competition_wisdom(&mut self, coef: f64, dt: f64) -> bool {
        let pct = (coef * (self.horse.wisdom.max(1.0) * 0.1).log10()).max(0.0);
        let per_tick_chance = 1.0 - (1.0 - pct.min(100.0) / 100.0).powf(dt / 2.0);
        self.rng.random() < per_tick_chance
    }

    fn update_position_keep_non_nige(&mut self, _dt: f64, field: Option<&RaceField>) {
        let field = match field {
            None => {
                self.pos_keep_mode = None;
                self.is_pace_down = false;
                self.pos_keep_speed_coef = 1.0;
                return;
            }
            Some(f) => f,
        };
        let i = self.field_index.unwrap();
        if field.pacemaker() == Some(i) {
            self.pos_keep_mode = None;
            self.is_pace_down = false;
            self.pos_keep_speed_coef = 1.0;
            return;
        }
        let gap = field.gap_to_pacemaker(i, self.pos);
        let style = crate::rushing::pacing_style(self.horse.strategy, if self.is_kakari {self.rushing_mode} else {0});
        let low = position_keep_min_threshold(style, self.course.distance);
        let high = position_keep_max_threshold(style, self.course.distance);
        let no_speed_skills = !(self.active_target_speed_skills.iter().map(|s| s.modifier).sum::<f64>() > 0.0
            || self.pace_cancel_current_speed() > 0.0);

        if self.pos_keep_mode == Some(PosKeepMode::PaceDown) {
            if gap >= self.pos_keep_effect_exit_distance || !no_speed_skills {
                self.pos_keep_mode = None;
                self.is_pace_down = false;
                self.pos_keep_speed_coef = 1.0;
            }
            return;
        }
        if self.pos_keep_mode == Some(PosKeepMode::PaceUp) {
            if gap <= self.pos_keep_effect_exit_distance {
                self.pos_keep_mode = None;
                self.pos_keep_speed_coef = 1.0;
            }
            return;
        }

        if gap < low && no_speed_skills {
            self.pos_keep_mode = Some(PosKeepMode::PaceDown);
            self.is_pace_down = true;
            self.pos_keep_effect_start = self.pos;
            let lo = low;
            let hi = if self.phase == 1 { lo + 0.5 * (high - lo) } else { high };
            self.pos_keep_effect_exit_distance = lo + self.pace_effect_rng.random() * (hi - lo);
            self.pos_keep_speed_coef = if self.phase == 1 { 0.945 } else { 0.915 };
        } else if gap > high && self.position_keep_wisdom_check(15.0) {
            self.pos_keep_mode = Some(PosKeepMode::PaceUp);
            let (lo, hi) = (low, high);
            self.pos_keep_effect_exit_distance = lo + self.pace_effect_rng.random() * (hi - lo);
            self.pos_keep_speed_coef = 1.04;
        }
    }

    fn update_position_keep_front_runner(&mut self, _dt: f64, field: Option<&RaceField>) {
        let field = match field {
            Some(f) => f,
            None => return,
        };
        let i = self.field_index.unwrap();
        let is_leader = field.order_of(i) == 1;
        let choose_speed_up = is_leader || (self.is_kakari && matches!(self.rushing_mode, 3 | 4));
        let gap_2nd = field.distance_to_behind(i, self.pos);
        let solo = field.ordinary_front_runner_count() == 1;
        let is_oonige = self.horse.strategy == Strategy::Oonige;
        let speed_up_threshold = if is_oonige { 17.5 } else if solo { 12.5 } else { 4.5 };
        let overtake_exit_threshold = if is_oonige { 27.5 } else { 10.0 };

        if self.pos_keep_mode == Some(PosKeepMode::SpeedUp) {
            if gap_2nd.is_none() || gap_2nd.unwrap() >= speed_up_threshold {
                self.pos_keep_mode = None;
                self.pos_keep_speed_coef = 1.0;
            }
            return;
        }
        if self.pos_keep_mode == Some(PosKeepMode::Overtake) {
            if is_leader && (gap_2nd.is_none() || gap_2nd.unwrap() >= overtake_exit_threshold) {
                self.pos_keep_mode = None;
                self.pos_keep_speed_coef = 1.0;
            }
            return;
        }

        // position_keep_wisdom_check needs &mut self, but is_leader/gap_2nd
        // were already computed above from `field` (now unused past this
        // point), so this doesn't conflict.
        if choose_speed_up && gap_2nd.is_some() && gap_2nd.unwrap() < speed_up_threshold {
            if self.position_keep_wisdom_check(20.0) {
                self.pos_keep_mode = Some(PosKeepMode::SpeedUp);
                self.pos_keep_speed_coef = 1.04;
            }
        } else if !choose_speed_up && self.position_keep_wisdom_check(20.0) {
            self.pos_keep_mode = Some(PosKeepMode::Overtake);
            self.pos_keep_speed_coef = 1.05;
        }
    }

    fn update_last_spurt_state(&mut self) {
        if self.is_last_spurt || self.phase < 2 {
            return;
        }
        if self.last_spurt_transition == -1.0 {
            let (transition, speed) = self.hp.get_last_spurt_pair(self.pos, self.last_spurt_speed, self.base_target_speed[2]);
            self.last_spurt_transition = transition;
            self.last_spurt_speed = speed;
            self.last_spurt_calc_count += 1;
            self.last_spurt_calc_result = if transition == -1.0 { 1 } else if transition < self.course.distance { 2 } else { 4 };
        }
        if self.pos >= self.last_spurt_transition {
            self.is_last_spurt = true;
            self.actual_last_spurt_start_time = Some(self.accumulatetime.get());
            self.actual_last_spurt_start_distance = Some(self.pos);
        }
    }

    fn update_target_speed(&mut self) {
        if self.is_last_spurt {
            self.target_speed = self.last_spurt_speed;
        } else {
            self.target_speed = self.base_target_speed[self.phase as usize];
            let idx = (self.pos / self.section_length) as usize;
            self.target_speed += self.section_modifier[idx];
        }
        self.target_speed *= self.pos_keep_speed_coef;
        self.target_speed += self.modifiers_target_speed.acc + self.modifiers_target_speed.err;

        if self.is_downhill_mode && self.slope_per < 0.0 {
            self.target_speed += 0.3 + self.slope_per.abs() / 100000.0;
        } else if self.hill_idx != -1 && self.slope_per > 0.0 {
            self.target_speed -= self.slope_per / 10000.0 * 200.0 / self.horse.power.clamp(1.0,2000.0);
        }

        // Doc: MoveLaneModifier -- applied when affected by a skill that
        // modifies lane move speed (ability type 28) AND lane movement
        // happened on the PREVIOUS frame. This runs before update_lane(dt)
        // in step(), so moved_lane_this_tick still holds last tick's value
        // here -- exactly "the previous frame", no extra state needed.
        if !self.active_lane_speed_skills.is_empty() && self.moved_lane_this_tick {
            self.target_speed += (0.0002 * self.horse.power.clamp(1.0,2000.0)).sqrt();
        }

        // Target speed cannot exceed 30 m/s (Game:Mechanics, Target Speed
        // section). Applied after every additive/uphill/downhill modifier.
        // Loaded client overrides all additive effects when HP is empty.
        if !self.hp.has_remaining_hp() { self.target_speed=self.min_speed; }
        self.target_speed = self.target_speed.clamp(13.0,30.0);
    }

    fn apply_forces(&mut self) {
        if !self.hp.has_remaining_hp() {
            self.accel = -1.2;
            return;
        }
        if self.current_speed > self.target_speed {
            self.accel = if self.is_pace_down { -0.5 } else { PHASE_DECELERATION[self.phase as usize] };
            return;
        }
        self.accel = self.base_accel[(self.slope_per > 0.0) as usize * 3 + self.phase as usize];
        self.accel += self.modifiers_accel.acc + self.modifiers_accel.err;
    }

    fn downhill_check(&mut self, roll: f64) {
        if self.slope_per < 0.0 && roll < self.horse.wisdom * 0.0004 {
            self.call_activate_hook("downhill", Perspective::Self_);
            self.is_downhill_mode = true;
        }
    }

    fn update_hills(&mut self) {
        if self.hill_idx == -1 && self.hill_start.last().map_or(false, |&s| self.pos >= s) {
            self.hill_idx = (self.n_hills - self.hill_start.len()) as i64;
            self.slope_per = self.course.slopes[self.hill_idx as usize].slope;
            self.downhill_timer.set(0.0);
            let roll = self.hill_rng[self.hill_idx as usize].random();
            self.downhill_check(roll);
            self.hill_start.pop();
        } else if self.hill_idx != -1 && self.hill_end.last().map_or(false, |&e| self.pos > e) {
            self.hill_idx = -1;
            self.slope_per = 0.0;
            self.hill_end.pop();
            if self.is_downhill_mode {
                self.call_deactivate_hook("downhill", Perspective::Self_);
            }
            self.is_downhill_mode = false;
        }
        // Re-roll once per second while still on a hill: a chance to drop
        // out of downhill mode, or (if not yet in it) another chance to
        // enter it -- matches race_solver.py's periodic re-check, which the
        // single entry-time roll above does not capture on its own.
        if self.downhill_timer.get() >= 1.0 && self.hill_idx != -1 {
            let roll = self.hill_rng[self.hill_idx as usize].random();
            if self.is_downhill_mode && roll > 0.8 {
                self.call_deactivate_hook("downhill", Perspective::Self_);
                self.is_downhill_mode = false;
            } else if !self.is_downhill_mode {
                self.downhill_check(roll);
            }
            self.downhill_timer.set(0.0);
        }
    }

    fn update_phase(&mut self) {
        // Phase is deliberately capped at 2 for speed-modifier purposes --
        // phase 3 (from 5/6 distance) is treated the same as phase 2 and is
        // never reached via this transition (matches race_solver.py).
        if self.pos >= self.next_phase_transition && self.phase < 2 {
            self.phase += 1;
            self.next_phase_transition = crate::course_data::phase_start(self.course.distance, self.phase + 1);
        }
    }

    // -- skill activation -------------------------------------------------

    pub(crate) fn record_overtakes(&mut self, count: usize) {
        self.pending_overtakes += count;
        self.phase_overtakes[(self.phase as usize).min(2)] += count;
        // The rank pass follows movement, but final-corner history reads the
        // course-event flag from the start of that tick. A pass on the entry
        // tick belongs to the preceding corner state.
        let after_final_corner = self.retail.as_ref().map_or_else(
            || self.course.corners.last().is_some_and(|c| self.pos >= c.start),
            |retail| retail.final_corner);
        if after_final_corner { self.final_corner_overtakes += count; }
        let late_corner = self.retail.as_ref().map_or_else(
            || self.pos >= self.course.distance * 2.0 / 3.0
                && self.course.corners.iter().any(|c| c.start <= self.pos && self.pos < c.start + c.length),
            // Rank updates run after movement. The phase and corner flags still
            // describe the start of that tick, including boundary crossings.
            |retail| retail.phase >= 2 && retail.corner != 0);
        if late_corner {
            self.late_corner_overtakes += count;
        }
    }

    fn process_skill_history(&mut self, field: Option<&RaceField>) {
        // v79: SkillDetail.UpdateSkill (0x35ce900) applies the order-up extension
        // (SkillAbilityTimeIncrementOrderUp.UpdateAbilityTime reads IsCurOrderUp) before it
        // subtracts the tick from LeftTime and stops the detail. A skill whose time runs out
        // on the tick after an overtake is therefore extended, not ended. Its effect receivers
        // were already updated in PreUpdate (SkillModifierReceiver.UpdateModifiers), so one
        // that ran out this tick stays expired (retail expire_skill_effects runs the effect
        // expiry first). unseen-runner-fresh-01 0117: 100531 keeps two 0.108 accelerations.
        let extend_first = self.retail.is_some();
        if extend_first {
            for _ in 0..self.pending_overtakes {
                self.extend_running_skills_on_overtake();
            }
        }
        self.retire_finished_running_skills();
        for _ in 0..self.pending_overtakes {
            if !extend_first {
                self.extend_running_skills_on_overtake();
            }
            self.additional_skill_event(1, None, field);
        }
        self.pending_overtakes = 0;
    }

    // Receivers expire in PreUpdate, but SkillDetail.UpdateSkill handles
    // queued additional activations before stopping the running detail.
    // Keep that detail through its final ordered turn. Earlier activations in
    // that pass can still add a bonus; its original receivers remain expired.
    fn retire_finished_running_skills(&mut self) -> bool {
        let mut retired = false;
        self.retiring_additional_skills = false;
        for i in (0..self.running_skills.len()).rev() {
            if self.running_skills[i].timer.get() >= 0.0 {
                let runtime = &self.running_skills[i];
                if self.retail.is_some() && runtime.numeric_skill_id > self.additional_turn_floor
                    && runtime.delayed.iter().any(|(effect, _)| matches!(effect.additional_activate, 2 | 3)) {
                    self.retiring_additional_skills = true;
                    continue;
                }
                let runtime = self.running_skills.remove(i);
                if runtime.unapplied > 0 { self.unapplied_running_skills -= 1; }
                self.release_timer(&runtime.timer);
                retired = true;
            }
        }
        retired
    }

    fn extend_running_skills_on_overtake(&mut self) {
        for runtime in &mut self.running_skills {
            if runtime.extensions >= 3 || !runtime.extend_on_overtake { continue; }
            let extension = self.course.distance / 1000.0;
            runtime.timer.set(runtime.timer.get() - extension);
            runtime.extensions += 1;
            for active in [&mut self.active_target_speed_skills, &mut self.active_current_speed_skills,
                           &mut self.active_accel_skills, &mut self.active_lane_speed_skills, &mut self.active_aux_skills] {
                for effect in active {
                    if effect.skill_id == runtime.skill_id && effect.perspective == runtime.perspective {
                        effect.duration_timer.set(effect.duration_timer.get() - extension);
                    }
                }
            }
        }
    }

    fn additional_skill_event(&mut self, event_type: i32, exclude: Option<&str>, field: Option<&RaceField>) {
        if event_type == 2 && self.retail.is_some() {
            // v74: SkillDetail.UpdateSkill (0x35ce900) adds these increments during the running
            // skill's own turn in the per-horse ascending-id skill pass, counting the manager's
            // recently-activated list (an entry leaves it at its own skill's next turn). An
            // activation after the running skill's turn is therefore applied on the next tick.
            for runtime in &mut self.running_skills {
                if Some(runtime.skill_id.as_str()) == exclude
                    || (runtime.timer.get() >= 0.0 && runtime.numeric_skill_id <= self.additional_turn_floor) { continue; }
                if runtime.delayed.iter().any(|(e, _)| matches!(e.additional_activate, 2 | 3)) {
                    if runtime.unapplied == 0 { self.unapplied_running_skills += 1; }
                    runtime.unapplied += 1;
                }
            }
            return;
        }
        let mut applications = Vec::new();
        for runtime in &mut self.running_skills {
            if Some(runtime.skill_id.as_str()) == exclude || runtime.timer.get() >= 0.0 { continue; }
            for (effect, count) in &mut runtime.delayed {
                let kind = effect.additional_activate;
                if !(kind == event_type || (event_type == 2 && kind == 3)) { continue; }
                if *count >= if kind == 3 { 2 } else { 3 } { continue; }
                *count += 1;
                // Additional-activation effects last one tick beyond the running skill's
                // remaining time (skill-mechanics-dev-01; doc 13.177).
                applications.push((runtime.skill_id.clone(), runtime.perspective, runtime.rarity, *effect, -runtime.timer.get() + 1.0 / 15.0, runtime.detail_index));
            }
        }
        for (id, perspective, rarity, effect, duration, detail) in applications {
            let targets = self.apply_activation(&id, perspective, rarity, &[effect], false, field, true, Some(duration), true, detail);
            if perspective == Perspective::Self_ && id.parse::<u32>().is_ok() {
                self.additional_target_events.push((id, detail, self.accumulatetime.get(), targets));
            }
        }
    }

    /// v75: SkillTriggerIsActivateAnySkill.GetValueLh (0x35e1b00) on every check of the detail:
    /// v = GetActivateSkillCount - trues; result = v > last; last = v; trues += result.
    /// SkillTriggerCollection.Update (0x35e49e0) updates every trigger of the condition, so the
    /// state advances whenever the detail is checked (not activated, cooldown ready,
    /// precondition met), whatever the other conditions read.
    fn observe_anyskill(&mut self, i: usize) {
        let sk = &self.pending_skills[i];
        let checkable = (sk.prerequisite.is_none() || sk.prerequisite_satisfied)
            && self.skill_cooldown_ready(&sk.cooldown_key);
        let count: i32 = self.activate_count.iter().sum();
        let mut state = self.anyskill_observer.borrow_mut();
        let mut result = self.anyskill_result.borrow_mut();
        for &node in &sk.anyskill_nodes {
            if !checkable { result.insert(node, false); continue; }
            let entry = state.entry(node).or_insert((0, 0));
            let value = count - entry.1;
            let changed = value > entry.0;
            entry.0 = value;
            if changed { entry.1 += 1; }
            result.insert(node, changed);
        }
    }

    /// v74: apply recorded own-activation increments of running skills whose turn in the
    /// ascending-id pass has come (skill id <= `upto`), per ability, capped as before.
    fn apply_unapplied_additional(&mut self, upto: i64, field: Option<&RaceField>) {
        // Each running skill takes one turn per tick: ids in (floor, upto] take theirs now.
        let floor = self.additional_turn_floor;
        if upto <= floor { return; }
        self.additional_turn_floor = upto;
        if self.unapplied_running_skills == 0 && !self.retiring_additional_skills { return; }
        let mut applications = Vec::new();
        for runtime in &mut self.running_skills {
            let id = runtime.numeric_skill_id;
            if runtime.unapplied == 0 || id <= floor || id > upto { continue; }
            let n = std::mem::take(&mut runtime.unapplied);
            self.unapplied_running_skills -= 1;
            for (effect, count) in &mut runtime.delayed {
                let kind = effect.additional_activate;
                if !matches!(kind, 2 | 3) { continue; }
                let cap = if kind == 3 { 2 } else { 3 };
                let k = n.min(cap - (*count).min(cap));
                *count += k;
                for _ in 0..k {
                    applications.push((runtime.skill_id.clone(), runtime.perspective, runtime.rarity, *effect, -runtime.timer.get() + 1.0 / 15.0, runtime.detail_index));
                }
            }
        }
        for (id, perspective, rarity, effect, duration, detail) in applications {
            let targets = self.apply_activation(&id, perspective, rarity, &[effect], false, field, true, Some(duration), true, detail);
            if perspective == Perspective::Self_ && id.parse::<u32>().is_ok() {
                self.additional_target_events.push((id, detail, self.accumulatetime.get(), targets));
            }
        }
        if self.retire_finished_running_skills() {
            self.start_expired_skill_cooldowns();
        }
    }

    fn process_skill_activations(&mut self, field: Option<&RaceField>) {
        self.expire_skill_effects(field);
        self.activate_count_last_frame=0;
        self.process_skill_priority(None,field);
    }

    pub(crate) fn expire_skill_effects(&mut self, field: Option<&RaceField>) {
        if self.retail.is_none() {
            self.process_skill_history(field);
        }
        for i in (0..self.active_target_speed_skills.len()).rev() {
            if self.active_target_speed_skills[i].duration_timer.get() >= 0.0 {
                let s = self.active_target_speed_skills.remove(i);
                self.release_timer(&s.duration_timer);
                self.modifiers_target_speed.add(-s.modifier);
                self.retail_target_modifiers.remove(s.token);
                self.call_deactivate_hook(&s.skill_id, s.perspective);
            }
        }
        for i in (0..self.active_current_speed_skills.len()).rev() {
            if self.active_current_speed_skills[i].duration_timer.get() >= 0.0 {
                let s = self.active_current_speed_skills.remove(i);
                self.release_timer(&s.duration_timer);
                self.modifiers_current_speed.add(-s.modifier);
                if s.natural_deceleration {
                    self.modifiers_one_frame_accel += s.modifier;
                }
                self.call_deactivate_hook(&s.skill_id, s.perspective);
            }
        }
        for i in (0..self.active_accel_skills.len()).rev() {
            if self.active_accel_skills[i].duration_timer.get() >= 0.0 {
                let s = self.active_accel_skills.remove(i);
                self.release_timer(&s.duration_timer);
                self.modifiers_accel.add(-s.modifier);
                self.retail_accel_modifiers.remove(s.token);
                self.call_deactivate_hook(&s.skill_id, s.perspective);
            }
        }
        for i in (0..self.active_lane_speed_skills.len()).rev() {
            if self.active_lane_speed_skills[i].duration_timer.get() >= 0.0 {
                let s = self.active_lane_speed_skills.remove(i);
                self.release_timer(&s.duration_timer);
                self.call_deactivate_hook(&s.skill_id, s.perspective);
            }
        }

        for i in (0..self.active_aux_skills.len()).rev() {
            if self.active_aux_skills[i].duration_timer.get() >= 0.0 {
                let effect = self.active_aux_skills.remove(i);
                self.release_timer(&effect.duration_timer);
                self.call_deactivate_hook(&effect.skill_id, effect.perspective);
            }
        }
        if self.retail.is_some() {
            self.process_skill_history(field);
        }
    }

    fn refresh_skill_scan(&mut self) {
        // Pending records retain their priority/prerequisite for their lifetime.
        // All runtime edits remove entries; rebuild before the next skill pass.
        if self.skill_scan.pending_len == self.pending_skills.len() { return; }
        let index = &mut self.skill_scan;
        index.pending_len = self.pending_skills.len();
        index.prerequisites.clear();
        index.priority_zero.clear();
        index.priority_one.clear();
        index.windows.clear();
        for (i, skill) in self.pending_skills.iter().enumerate() {
            index.windows.push(SkillScanWindow {
                start: skill.trigger.start,
                end: skill.trigger.end,
                numeric_id: skill.numeric_skill_id.unwrap_or(i64::MIN),
                priority: skill.priority,
                retained: skill.detail_index == 0
                    && matches!(skill.rarity, SkillRarity::White | SkillRarity::Gold | SkillRarity::Evolution),
                observed: !skill.anyskill_nodes.is_empty(),
            });
            if skill.prerequisite.is_some() && !skill.prerequisite_satisfied {
                index.prerequisites.push(i);
            }
            match skill.priority {
                0 => index.priority_zero.push(i),
                1 => index.priority_one.push(i),
                _ => {}
            }
        }
    }

    pub(crate) fn process_skill_priority(&mut self, priority:Option<i32>,field:Option<&RaceField>) {
        self.refresh_skill_scan();
        // Priority zero must still publish additional activations/reservations
        // with an empty list. Priority one has no such end-of-pass work.
        if priority == Some(1) && self.skill_scan.priority_one.is_empty() { return; }
        // Watch prerequisites before their main trigger is armed. Skill records
        // belong to this solver, so each runner/alternative has its own latch.
        for at in 0..self.skill_scan.prerequisites.len() {
            let i = self.skill_scan.prerequisites[at];
            let sk = &self.pending_skills[i];
            if priority.is_some_and(|p|p!=sk.priority) {continue;}
            let satisfied = !sk.prerequisite_satisfied
                && sk.prerequisite.as_ref().is_some_and(|check| check(self, field));
            if satisfied { self.pending_skills[i].prerequisite_satisfied = true; }
        }
        let pending = &self.pending_skills;
        self.skill_scan.prerequisites.retain(|&i| !pending[i].prerequisite_satisfied);
        if self.armed_scan && priority.is_none()
            && !(self.retail.is_some() && self.pending_skills.iter().any(|sk| !sk.anyskill_nodes.is_empty())) {
            self.process_pending_armed(self.pos, field);
        } else {
            self.process_pending_full(self.pos, field, priority);
        }
        if self.retail.is_some() && (priority.is_none() || priority == Some(0)) {
            self.random_gold_reservations = self.select_random_gold(self.random_gold_request, false, &BTreeSet::new())
                .into_iter().map(|sk| sk.skill_id).collect();
            let excluded = self.random_gold_reservations.clone();
            let extra = self.select_random_gold(self.random_any_request, true, &excluded);
            self.random_gold_reservations.extend(extra.into_iter().map(|s|s.skill_id));
            self.random_gold_request = 0.0;
            self.random_any_request = 0.0;
        }
    }

    /// Position-ordered equivalent of the full scan below. Arms skills as
    /// `pos` reaches their trigger.start, then tests only the armed ones. A
    /// skill with trigger.start > pos is a guaranteed no-op in the full scan
    /// (trigger.end >= trigger.start, so neither branch can fire) and
    /// `pending_removal` is provably empty here (see `armed_scan`), so the
    /// skipped entries cannot matter. Armed skills are visited in
    /// descending sequence-number order, the same relative order the full
    /// scan visits them in, so activations within a frame keep their
    /// original order.
    fn process_pending_armed(&mut self, pos: f64, field: Option<&RaceField>) {
        while let Some(&(start, _seq)) = self.pending_queue.last() {
            if start > pos {
                break;
            }
            let (_, seq) = self.pending_queue.pop().unwrap();
            let at = self.armed.partition_point(|&s| s < seq);
            self.armed.insert(at, seq);
        }

        let mut activate_count_this_frame = 0;
        let mut i = self.armed.len();
        while i > 0 {
            i -= 1;
            let seq = self.armed[i];
            let j = self.pending_seq.binary_search(&seq).expect("armed seq must be present in pending_seq");
            let trigger_end = self.pending_skills[j].trigger.end;
            if pos >= trigger_end {
                self.armed.remove(i);
                self.pending_skills.remove(j);
                self.pending_seq.remove(j);
                continue;
            }
            let sk = &self.pending_skills[j];
            let fires = (sk.prerequisite.is_none() || sk.prerequisite_satisfied)
                && (field.is_some() || sk.effects.iter().all(|ef| ef.target == 1))
                && self.skill_cooldown_ready(&sk.cooldown_key)
                && sk.windows.iter().any(|r| r.start <= pos && pos < r.end)
                && (sk.extra_condition)(self, field);
            if fires {
                let sk = &self.pending_skills[j];
                let (sid, perspective, rarity, effects, green, cooldown, cooldown_key) =
                    (sk.skill_id.clone(), sk.perspective, sk.rarity, sk.effects.clone(), sk.has_green_tag, sk.cooldown, sk.cooldown_key.clone());
                let is_special = sid == "asitame" || sid == "staminasyoubu";
                self.activate_skill(&sid, perspective, rarity, &effects, green, self.pending_skills[j].detail_index, field);
                if cooldown <= 0.0 {
                    self.skill_cooldowns.insert(cooldown_key, f64::INFINITY);
                    self.armed.remove(i); self.pending_skills.remove(j); self.pending_seq.remove(j);
                } else {
                    if self.retail.is_some() && !cooldown_key.ends_with("#followup") {
                        self.waiting_skill_cooldowns.insert(cooldown_key,
                            (sid.clone(), self.pending_skills[j].detail_index, cooldown));
                    } else { self.start_skill_cooldown(cooldown_key,cooldown); }
                }
                if !is_special {
                    activate_count_this_frame += 1;
                }
            }
        }
        self.activate_count_last_frame = activate_count_this_frame;
    }

    /// Plain full-list scan, used whenever any pending skill carries an
    /// ACTIVATE_RANDOM_GOLD effect (so `pending_removal` can become
    /// non-empty -- see `_do_activate_random_gold`, which can re-activate
    /// another still-pending skill out of order).
    fn process_pending_full(&mut self, pos: f64, field: Option<&RaceField>, priority:Option<i32>) {
        let mut activate_count_this_frame = 0;
        if self.retail.is_some() && priority.map_or(true, |p| p == 0) {
            self.additional_turn_floor = i64::MIN;
        }
        let mut remaining = match priority {
            Some(0) => self.skill_scan.priority_zero.len(),
            Some(1) => self.skill_scan.priority_one.len(),
            _ => self.pending_skills.len(),
        };
        while remaining > 0 {
            remaining -= 1;
            // Removing the current record cannot move any lower index, which
            // is why the original descending order also permits this index.
            let i = match priority {
                Some(0) => self.skill_scan.priority_zero[remaining],
                Some(1) => self.skill_scan.priority_one[remaining],
                _ => remaining,
            };
            if self.retail.is_some() {
                if let Some(window) = self.skill_scan.windows.get(i) {
                    if priority.is_some_and(|p| p != window.priority) { continue; }
                    if !(pos >= window.start && pos < window.end)
                        && (window.retained || !(pos >= window.end))
                        && !window.observed && self.unapplied_running_skills == 0
                        && self.pending_removal.is_empty() && self.random_gold_reservations.is_empty()
                    {
                        // Even an inactive detail takes its turn in the ordered
                        // additional-effect pass. Later activations must see it.
                        self.additional_turn_floor = self.additional_turn_floor.max(window.numeric_id);
                        continue;
                    }
                }
            }
            if priority.is_some_and(|p|p!=self.pending_skills[i].priority) {continue;}
            if self.retail.is_some() {
                if let Some(id) = self.pending_skills[i].numeric_skill_id {
                    self.apply_unapplied_additional(id, field);
                }
            }
            if self.retail.is_some() && !self.pending_skills[i].anyskill_nodes.is_empty() {
                self.observe_anyskill(i);
            }
            let skill_id_matches_removal = self.pending_removal.contains(&self.pending_skills[i].cooldown_key);
            let reserved = self.pending_skills[i].detail_index == 0
                && self.random_gold_reservations.contains(&self.pending_skills[i].skill_id);
            let retain_for_random = self.retail.is_some() && self.pending_skills[i].detail_index == 0
                && matches!(self.pending_skills[i].rarity, SkillRarity::White | SkillRarity::Gold | SkillRarity::Evolution);
            if (pos >= self.pending_skills[i].trigger.end && !reserved && !retain_for_random) || skill_id_matches_removal {
                let removed = self.pending_skills.remove(i);
                self.pending_removal.remove(&removed.cooldown_key);
                continue;
            }
            let sk = &self.pending_skills[i];
            let fires = (reserved || (pos >= sk.trigger.start && pos < sk.trigger.end
                && (sk.prerequisite.is_none() || sk.prerequisite_satisfied)
                && sk.windows.iter().any(|r| r.start <= pos && pos < r.end)
                && (sk.extra_condition)(self, field)))
                && (field.is_some() || sk.effects.iter().all(|ef| ef.target == 1))
                && self.skill_cooldown_ready(&sk.cooldown_key);
            if fires {
                let (skill_id, perspective, rarity, effects, has_green_tag) = {
                    let sk = &self.pending_skills[i];
                    (sk.skill_id.clone(), sk.perspective, sk.rarity, sk.effects.clone(), sk.has_green_tag)
                };
                let cooldown = self.pending_skills[i].cooldown;
                let cooldown_key = self.pending_skills[i].cooldown_key.clone();
                self.activate_skill(&skill_id, perspective, rarity, &effects, has_green_tag, self.pending_skills[i].detail_index, field);
                if cooldown <= 0.0 { self.skill_cooldowns.insert(cooldown_key, f64::INFINITY); self.pending_skills.remove(i); }
                else {
                    if self.retail.is_some() && !cooldown_key.ends_with("#followup") {
                        self.waiting_skill_cooldowns.insert(cooldown_key,
                            (skill_id.clone(), self.pending_skills[i].detail_index, cooldown));
                    } else { self.start_skill_cooldown(cooldown_key,cooldown); }
                }
                if skill_id != "asitame" && skill_id != "staminasyoubu" {
                    activate_count_this_frame += 1;
                }
            }
        }
        if self.retail.is_some() && priority.map_or(true, |p| p == 0) {
            self.apply_unapplied_additional(i64::MAX, field);
        }
        self.activate_count_last_frame += activate_count_this_frame;
    }

    /// v7: ReleaseConservePower fires on the tick the last spurt starts (official kind-6
    /// events precede the next skill pass by exactly one tick in 191/192 corpus releases).

    pub(crate) fn conserve_release_duration(&self) -> f64 {
        if self.cp_clean.supported() {
            self.cp_clean.duration(self.course.distance, self.course.distance_type)
        } else {
            crate::race_solver_builder::asitame_release_duration(self.cp_acc, self.course.distance_type)
        }
    }

    pub(crate) fn conserve_release_reaches_threshold(&self) -> bool {
        // Only the experimental clean branch changes the legacy gate.
        self.retail.is_none() || !self.cp_clean.supported()
            || self.conserve_release_duration() >= 0.2_f32 as f64
    }

    pub(super) fn release_conserve_power_now(&mut self, field: Option<&RaceField>) {
        let pos = self.pos;
        let Some(i) = self.pending_skills.iter().position(|sk| sk.skill_id == "asitame") else { return };
        let sk = &self.pending_skills[i];
        let fires = pos >= sk.trigger.start && pos < sk.trigger.end
            && (sk.extra_condition)(self, field)
            && self.skill_cooldown_ready(&sk.cooldown_key);
        if !fires { return; }
        let (skill_id, perspective, rarity, effects, has_green_tag, detail, key) = {
            let sk = &self.pending_skills[i];
            (sk.skill_id.clone(), sk.perspective, sk.rarity, sk.effects.clone(), sk.has_green_tag, sk.detail_index, sk.cooldown_key.clone())
        };
        self.activate_skill(&skill_id, perspective, rarity, &effects, has_green_tag, detail, field);
        self.skill_cooldowns.insert(key, f64::INFINITY);
    }

    fn skill_cooldown_ready(&self,key:&str)->bool {
        if self.waiting_skill_cooldowns.contains_key(key) {return false;}
        if self.skill_cooldowns.get(key)==Some(&f64::INFINITY) {return false;}
        if let Some(timer)=self.retail.as_ref().and_then(|r|r.skill_cooldowns.get(key)) {return timer.get()>=0.;}
        self.accumulatetime.get()>=*self.skill_cooldowns.get(key).unwrap_or(&-1.)
    }

    fn start_expired_skill_cooldowns(&mut self) {
        let expired: Vec<_> = self.waiting_skill_cooldowns.iter().filter(|(_, (sid, detail, _))| {
            !self.running_skills.iter().any(|r| r.skill_id == *sid && r.detail_index == *detail)
                && (self.random_active_details.is_empty()
                    || !self.random_active_details.get(&(sid.clone(), *detail)).is_some_and(|t| t.get() < 0.0))
        }).map(|(key,(_,_,duration))|(key.clone(),*duration)).collect();
        for (key,duration) in expired {
            self.start_skill_cooldown(key.clone(),duration);
            self.waiting_skill_cooldowns.remove(&key);
        }
    }

    fn start_skill_cooldown(&mut self,key:String,duration:f64) {
        if self.retail.is_some() {
            let remaining=(duration as f32)*(self.course.distance as f32/1000.);
            if let Some(timer)=self.retail.as_ref().unwrap().skill_cooldowns.get(&key) {timer.set(-remaining as f64);}
            else {let timer=self.new_timer(-remaining as f64);self.retail.as_mut().unwrap().skill_cooldowns.insert(key,timer);}
        } else {self.skill_cooldowns.insert(key,self.accumulatetime.get()+duration*self.course.distance/1000.);}
    }

    fn activate_skill(&mut self, skill_id: &str, perspective: Perspective, rarity: SkillRarity, effects: &[SkillEffect], has_green_tag: bool, detail_index: usize, field: Option<&RaceField>) {
        let targets = self.apply_activation(skill_id, perspective, rarity, effects, has_green_tag, field, false, None, false, detail_index);
        if perspective == Perspective::Self_ && skill_id.parse::<i64>().is_ok() {
            self.skill_detail_events.push((skill_id.to_string(), detail_index, self.accumulatetime.get()));
            self.skill_target_events.push((skill_id.to_string(), detail_index, self.accumulatetime.get(), targets));
        }
    }

    pub(crate) fn is_debuff_immune(&self) -> bool {
        self.active_aux_skills.iter().any(|s| s.effect_type == SkillType::DebuffImmunity)
    }

    fn apply_activation(&mut self, skill_id: &str, perspective: Perspective, rarity: SkillRarity, effects: &[SkillEffect], has_green_tag: bool, field: Option<&RaceField>, follow_up: bool, duration_override: Option<f64>, reselect_targets: bool, detail_index: usize) -> Vec<Option<Vec<usize>>> {
        // ExtendEvolvedDuration must apply after other effects on the same
        // skill, so it doesn't extend its own siblings' durations.
        let mut effect_order: Vec<usize> = (0..effects.len()).collect();
        effect_order.sort_by_key(|&i| (effects[i].effect_type == SkillType::ExtendEvolvedDuration) as u8);
        let mut selected_targets = vec![None; effects.len()];

        // Freeze activation context before a sibling effect can recover HP.
        let remaining_hp = self.hp.hp_value();
        // External effects already carry the caster's frozen duration. Avoid
        // querying a currently borrowed caster while applying to its receiver.
        let leader_gap = if duration_override.is_none() {
            field.zip(self.field_index).map(|(f, i)| f.distance_to_leader(i, self.pos))
        } else {None};
        if !follow_up && perspective == Perspective::Self_ && skill_id.parse::<u32>().is_ok() { self.additional_skill_event(2, Some(skill_id), field); }
        let mut runtime_duration: f64 = 0.0;
        let mut delayed = Vec::new();
        for ability_index in effect_order {
            let ef = &effects[ability_index];
            // Selection precedes value calculation on every caster activation.
            // Reuse the result during delivery rather than drawing again.
            if (!follow_up || reselect_targets)
                && !matches!(ef.effect_type, SkillType::Noop | SkillType::Runaway) {
                if let Some(f) = field {
                    selected_targets[ability_index] = Some(f.skill_targets(self, ef, ef.modifier));
                }
            }
            let mut modifier = if self.retail.is_some() && ef.effect_type != SkillType::Noop && matches!(ef.value_usage,8|9) {
                let roll=self.rng_uniform(100.);
                ef.modifier * if roll <= 60. {0.} else if roll <= 90. {0.019999999552965164} else {0.03999999910593033}
            } else { ef.activation_modifier(&mut self.rng, self.activated_green_skills.len()) };
            if skill_id == "asitame" {
                // v8: ReleaseConservePower accel = sqrt((P-1200)*DeclCoef130)*AccelCoef0.001*style/distance coef,
                // reduced x0.98 after a position struggle and x0.8 after rushing during position keep.
                modifier = crate::race_solver_builder::asitame_release_modifier(
                    self.horse.raw_power, self.horse.strategy, self.course.distance_type);
                if self.cp_struggle { modifier *= 0.98; }
                if self.cp_kakari { modifier *= 0.8; }
            } else if skill_id == "staminasyoubu" {
                modifier = crate::race_solver_builder::stamina_syoubu_calc_approximate_modifier(
                    self.horse.raw_stamina, self.course.distance);
            }
            if ef.value_usage == 11 {
                let count = self.late_corner_overtakes;
                modifier *= if count < 2 { 1.0 } else if count < 3 { 1.100000023841858 } else if count < 4 { 1.2000000476837158 } else { 1.25 };
            }
            let coefficient=ef.duration_coefficient(remaining_hp, leader_gap, self.retail.is_some());
            let evolution=if rarity == SkillRarity::Evolution { self.modifiers_special_skill_duration_scaling } else { 1.0 };
            let scaled_duration = if self.retail.is_some() {
                (((ef.base_duration as f32)*(self.course.distance as f32/1000.))*(coefficient as f32)*(evolution as f32)) as f64
            } else {ef.base_duration * coefficient * (self.course.distance / 1000.0) * evolution};
            let scaled_duration = duration_override.unwrap_or(scaled_duration);
            // v8: release duration follows the accumulated conserve time.
            let scaled_duration = if skill_id == "asitame" && self.retail.is_some() {
                self.conserve_release_duration()
            } else { scaled_duration };
            runtime_duration = runtime_duration.max(scaled_duration);
            if !follow_up && perspective == Perspective::Self_ {
                let null = matches!(ef.effect_type, SkillType::Noop | SkillType::Runaway);
                let kind = if null {0} else {ef.effect_type as i32};
                let value = if null {0.0} else {modifier};
                if let Some((f, i)) = field.zip(self.field_index) {
                    f.record_ability_activation(i, kind, value);
                } else {
                    self.initial_ability_activations.push((kind, value));
                }
            }
            if ef.additional_activate != 0 && !follow_up { delayed.push((*ef, 0)); continue; }
            if (!follow_up || reselect_targets) && ef.target == 1 && ef.target_is_debuff && self.is_debuff_immune() { continue; }
            if ef.target != 1 && (!follow_up || reselect_targets) {
                if let Some(f) = field {
                    let targets = selected_targets[ability_index].take();
                    selected_targets[ability_index] = Some(f.queue_skill_effect(self, skill_id, rarity, *ef, modifier, scaled_duration, targets));
                    if scaled_duration > 0.0 {
                        let timer = self.new_skill_timer(scaled_duration);
                        self.active_aux_skills.push(ActiveSkill { token: 0, skill_id: skill_id.to_string(), perspective,
                            duration_timer: timer, modifier: 0.0, natural_deceleration: false, effect_type: SkillType::Noop });
                    }
                }
                continue;
            }
            match ef.effect_type {
                SkillType::DebuffImmunity => {
                    let timer = self.new_skill_timer(scaled_duration);
                    self.active_aux_skills.push(ActiveSkill { token: 0, skill_id: skill_id.to_string(), perspective,
                        duration_timer: timer, modifier: 1.0, natural_deceleration: false, effect_type: ef.effect_type });
                }
                SkillType::Vision | SkillType::SetTargetLane | SkillType::PreferOutsideLane | SkillType::PreferInsideLane => {
                    let timer = self.new_skill_timer(scaled_duration);
                    self.active_aux_skills.push(ActiveSkill { token: 0, skill_id: skill_id.to_string(), perspective,
                        duration_timer: timer, modifier, natural_deceleration: false, effect_type: ef.effect_type });
                }
                SkillType::Runaway => {
                    // Learned effect initializes style before construction;
                    // the client's runtime ability is SkillAbilityNull.
                }
                SkillType::Noop => {}
                SkillType::SpeedUp => self.add_stat_modifier(0, modifier),
                SkillType::StaminaUp => {
                    self.add_stat_modifier(1, modifier);
                    self.horse.raw_stamina = (self.horse.raw_stamina + modifier).max(1.0);
                }
                SkillType::PowerUp => {
                    self.add_stat_modifier(2, modifier);
                    self.horse.raw_power = (self.horse.raw_power + modifier).max(1.0);
                }
                SkillType::GutsUp => self.add_stat_modifier(3, modifier),
                SkillType::WisdomUp => self.add_stat_modifier(4, modifier),
                // Effect type 32 -- see race_solver.py's ALL_STATUS_UP branch
                // for how it was identified and why the modifier is on the
                // stat-point scale.
                SkillType::AllStatusUp => {
                    self.add_stat_modifier(0, modifier);
                    self.add_stat_modifier(1, modifier);
                    self.horse.raw_stamina = (self.horse.raw_stamina + modifier).max(1.0);
                    self.add_stat_modifier(2, modifier);
                    self.horse.raw_power = (self.horse.raw_power + modifier).max(1.0);
                    self.add_stat_modifier(3, modifier);
                    self.add_stat_modifier(4, modifier);
                }
                SkillType::MultiplyStartDelay => {
                    // Retail initialization already applies the successful
                    // gate effects. The runtime activation must not apply the
                    // multiplier again to the delay read by skill conditions.
                    if self.retail.is_none() { self.start_delay *= modifier; }
                },
                SkillType::ExtendKakari => {
                    if self.is_kakari {
                        if let Some(retail)=&mut self.retail {retail.rushing.set_forced_end(modifier as f32);}
                        // SetForceEndTimeBySkill replaces, rather than adds.
                        self.kakari_timer.set(-modifier.max(0.0));
                    }
                }
                SkillType::SetStartDelay => self.start_delay = modifier,
                SkillType::TargetSpeed => {
                    self.modifiers_target_speed.add(modifier);
                    let timer = self.new_skill_timer(scaled_duration);
                    let token = self.next_modifier_token();
                    self.retail_target_modifiers.register(token,modifier as f32);
                    self.active_target_speed_skills.push(ActiveSkill { token, skill_id: skill_id.to_string(), perspective, duration_timer: timer, modifier: modifier, natural_deceleration: false, effect_type: ef.effect_type });
                }
                SkillType::LaneMoveSpeedUp => {
                    let timer = self.new_skill_timer(scaled_duration);
                    self.active_lane_speed_skills.push(ActiveSkill { token: 0, skill_id: skill_id.to_string(), perspective, duration_timer: timer, modifier: modifier, natural_deceleration: false, effect_type: ef.effect_type });
                }
                SkillType::ModifyKakariChance => self.modifiers_kakari_chance += modifier / 100.0,
                SkillType::Accel => {
                    self.modifiers_accel.add(modifier);
                    let timer = self.new_skill_timer(scaled_duration);
                    let token = self.next_modifier_token();
                    if skill_id == "asitame" { self.cp_accel_token = Some(token); }
                    self.retail_accel_modifiers.register(token,modifier as f32);
                    self.active_accel_skills.push(ActiveSkill { token, skill_id: skill_id.to_string(), perspective, duration_timer: timer, modifier: modifier, natural_deceleration: false, effect_type: ef.effect_type });
                }
                SkillType::CurrentSpeed | SkillType::CurrentSpeedWithNaturalDeceleration => {
                    self.modifiers_current_speed.add(modifier);
                    let timer = self.new_skill_timer(scaled_duration);
                    let natural_deceleration = ef.effect_type == SkillType::CurrentSpeedWithNaturalDeceleration;
                    self.active_current_speed_skills.push(ActiveSkill { token: 0, skill_id: skill_id.to_string(), perspective, duration_timer: timer, modifier: modifier, natural_deceleration, effect_type: ef.effect_type });
                }
                SkillType::Recovery => {
                    if perspective == Perspective::Self_ && modifier > 0.0 {
                        self.activate_count_heal += 1;
                        if let Some((f, i)) = field.zip(self.field_index) { f.record_heal(i); }
                    }
                    if let Some(retail) = &mut self.retail {
                        // Modifier 12 is folded during the runner update,
                        // after all skills, not while evaluating conditions.
                        if let HpPolicy::Game(hp) = &self.hp {
                            retail.pending_hp += (hp.max_hp as f32) * (modifier as f32);
                        }
                    } else {
                        self.hp.recover(modifier);
                    }
                    self.stamina_keep_active = false; // doc: HP-recovery resets Stamina Keep
                    if self.retail.is_none() && modifier > 0.0 && self.phase >= 2 && !self.is_last_spurt {
                        self.last_spurt_transition = -1.0;
                        self.update_last_spurt_state();
                    }
                }
                SkillType::ActivateRandomSkill => self.do_activate_random_gold(modifier, field, true),
                SkillType::ActivateRandomGold => self.do_activate_random_gold(modifier, field, false),
                SkillType::ExtendEvolvedDuration => self.modifiers_special_skill_duration_scaling = modifier,
            }
        }

        if follow_up {
            if effects.iter().any(|ef| matches!(ef.effect_type, SkillType::SpeedUp | SkillType::StaminaUp | SkillType::PowerUp | SkillType::GutsUp | SkillType::WisdomUp | SkillType::AllStatusUp)) {
                self.refresh_stat_effects();
            }
            return selected_targets;
        }
        if runtime_duration > 0.0 {
            let key = (skill_id.to_string(), detail_index);
            if let Some(timer) = self.random_active_details.get(&key) {timer.set(-runtime_duration);}
            else {let timer = self.new_timer(-runtime_duration); self.random_active_details.insert(key,timer);}
        }
        let extend_on_overtake = effects.iter().any(|ef| ef.duration_usage == 4);
        if runtime_duration > 0.0 && (!delayed.is_empty() || extend_on_overtake) {
            let timer = self.new_timer(-runtime_duration);
            self.running_skills.push(RunningSkill { skill_id: skill_id.to_string(), numeric_skill_id: skill_id.parse().unwrap_or(i64::MAX), perspective, rarity, detail_index,
                timer, delayed, extend_on_overtake, extensions: 0, unapplied: 0 });
        }
        // Synthetic charge/stamina effects are not learned skill activations.
        if perspective == Perspective::Self_ && !matches!(skill_id, "asitame" | "staminasyoubu") {
            self.activate_count[self.phase as usize] += 1;
            self.activated_ids_this_tick.push(skill_id.parse::<i64>().unwrap_or(i64::MAX));
            if self.pos >= self.course.distance / 2.0 {
                self.activate_count_later_half += 1;
            }
        }
        self.used_skills.insert(skill_id.to_string());
        if perspective == Perspective::Self_ && has_green_tag {
            self.activated_green_skills.insert(skill_id.to_string());
        }
        self.call_activate_hook(skill_id, perspective);
        selected_targets
    }

    fn do_activate_random_gold(&mut self, ngolds: f64, field: Option<&RaceField>, include_white: bool) {
        // The initial trigger-only pass precedes PreUpdate(0), which expires
        // these one-time receiver requests before the selection phase.
        if self.retail_initializing { return; }
        if self.retail.is_some() {
            if include_white { self.random_any_request = ngolds; } else { self.random_gold_request = ngolds; }
            return;
        }
        for sk in self.select_random_gold(ngolds, include_white, &BTreeSet::new()) {
            self.activate_skill(&sk.skill_id, sk.perspective, sk.rarity, &sk.effects,
                sk.has_green_tag, sk.detail_index, field);
            self.pending_removal.insert(sk.cooldown_key.clone());
            self.skill_cooldowns.insert(sk.cooldown_key, f64::INFINITY);
        }
    }

    // Selection is separate from effect application/cooldown consumption.
    // Own the selected records so subsequent scans cannot invalidate indices.
    fn select_random_gold(&mut self, ngolds: f64, include_white: bool, excluded: &BTreeSet<String>) -> Vec<RandomSkillReservation> {
        if ngolds <= 0.0 { return Vec::new(); }
        let mut gold_indices: Vec<usize> = self
            .pending_skills
            .iter()
            .enumerate()
            .filter(|(_, sk)| sk.detail_index == 0 && sk.skill_id.parse::<u64>().is_ok()
                && crate::race_solver_builder::random_activation_is_general(&sk.skill_id)
                && !excluded.contains(&sk.skill_id)
                && (matches!(sk.rarity, SkillRarity::Gold | SkillRarity::Evolution) || (include_white && sk.rarity == SkillRarity::White)) && !self.pending_removal.contains(&sk.cooldown_key)
                && (if self.retail.is_some() {self.skill_cooldown_ready(&sk.cooldown_key)
                    && !self.random_active_details.iter().any(|((sid,_),timer)| sid == &sk.skill_id && timer.get() < 0.0)
                    && !self.running_skills.iter().any(|r|r.skill_id == sk.skill_id)}
                    else {!self.used_skills.contains(&sk.skill_id)})
                && sk.effects.iter().all(|ef| ef.base_duration >= 0.0 && ef.effect_type as i32 > SkillType::WisdomUp as i32
                && ef.effect_type != SkillType::AllStatusUp))
            .map(|(i, _)| i)
            .collect();
        let mut distinct: Vec<usize> = Vec::new();
        for i in gold_indices {
            if let Some(at) = distinct.iter().position(|&j| self.pending_skills[j].skill_id == self.pending_skills[i].skill_id) {
                distinct[at] = i;
            } else { distinct.push(i); }
        }
        gold_indices = distinct;
        gold_indices.sort_by_key(|&i| self.pending_skills[i].random_activation_order.unwrap_or(i));
        // Client 0x35bb540 draws one index and RemoveAt per reservation;
        // it does not shuffle every eligible skill first.
        let take = (ngolds as usize).min(gold_indices.len());
        let mut selected = Vec::with_capacity(take);
        for _ in 0..take {
            let j = if self.retail.is_some() {self.rng_uniform(gold_indices.len() as f64) as usize}
                else {self.gorosi_rng.uniform(gold_indices.len() as f64) as usize};
            selected.push(gold_indices.remove(j));
        }
        selected.into_iter().map(|idx| {
            let sk = &self.pending_skills[idx];
            RandomSkillReservation { skill_id: sk.skill_id.clone(), cooldown_key: sk.cooldown_key.clone(),
                perspective: sk.perspective, rarity: sk.rarity, effects: sk.effects.clone(),
                has_green_tag: sk.has_green_tag, detail_index: sk.detail_index }
        }).collect()
    }

    // -- competition mechanics (doc: Spot Struggle, Dueling, Compete Before
    // Spurt, Secure Lead, Stamina Keep) -- per the doc itself, the
    // least-confident section of the whole engine: the author explicitly
    // flags these as "inferred from parameter file" or "under
    // investigation" rather than confirmed reverse-engineering. Treat this
    // as the first place to look if race behavior ever looks wrong.

    fn update_competition_mechanics(&mut self, dt: f64, field: Option<&RaceField>) {
        let field = match field {
            Some(f) => f,
            None => return,
        };
        if self.experimental_later_stamina_mechanics { self.update_stamina_keep(dt); }
        self.update_spot_struggle(field);
        if self.experimental_later_stamina_mechanics {
            self.update_compete_before_spurt(dt, field);
            self.update_secure_lead(dt, field);
        }
        self.update_dueling(dt, field);
    }

    /// Doc: Stamina Keep -- conserve roughly 1.035-1.04x the HP needed to
    /// finish. The doc's own author flags this activation-chance formula as
    /// "highly speculative", explicitly contradicted by their own
    /// packet-capture testing. "HP needed to finish" is approximated here
    /// as remaining-course-fraction (a coarse proxy). While active,
    /// suppresses Compete Before Spurt / Secure Lead entry, per doc; reset
    /// when an HP-recovery skill fires.
    fn update_stamina_keep(&mut self, dt: f64) {
        if self.stamina_keep_active || self.course.distance <= 0.0 {
            return;
        }
        let remaining_frac = ((self.course.distance - self.pos) / self.course.distance).max(0.0);
        if self.hp.hp_ratio_remaining() >= remaining_frac * 1.04 {
            return;
        }
        let wisdom = self.horse.wisdom.max(1.0);
        let pct = 30.0 * (wisdom / 1000.0 + wisdom.powf(0.03));
        let per_tick_chance = 1.0 - (1.0 - pct.min(100.0) / 100.0).powf(dt / 2.0);
        if self.rng.random() < per_tick_chance {
            self.stamina_keep_active = true;
        }
    }

    /// Doc: Spot Struggle / Lead Competition. Only Nige/Oonige-strategy
    /// horses compete, and only within their own strategy -- note this is
    /// an *exact* strategy match (`s.horse.strategy == strategy`), not
    /// `strategy_matches` (which treats Nige/Oonige as equivalent
    /// elsewhere): a Nige horse's group here excludes Oonige horses.
    pub(super) fn latch_active_spot_cutoff(&mut self) {
        self.spot_struggle_cutoff_latched |= self.spot_struggle_active && self.spot_struggle_timer.get() < 0.0 && self.pos >= self.section_length * SPOT_STRUGGLE_END_SECTION;
    }

    fn update_spot_struggle(&mut self, field: &RaceField) {
        let strategy = self.horse.strategy;
        if strategy != Strategy::Nige && strategy != Strategy::Oonige {
            return;
        }
        let i = self.field_index.unwrap();
        if self.spot_struggle_started {
            // Natural expiration does not remove the partner from distance checks.
            // v71: a partner exactly 5 m away still holds the struggle (exit beyond 5 m), as
            // for the inclusive duel boundary (doc 13.152). Official survey-1943 (c10508): two
            // Front runners 5.000000000 m apart at 14.1192 s kept CompeteTop HP cost one more
            // tick (exact HP floor at ten samples; sim exited a tick early with >=).
            let distance_exit = (0..field.n).filter(|&j| j != i).all(|j| {
                let s = field.solvers[j].borrow();
                s.horse.strategy != strategy || !s.spot_struggle_started
                    || (s.pos - self.pos).abs() > 5.0 || (s.lane - self.lane).abs() >= 0.416
            });
            self.spot_struggle_distance_exited |= distance_exit;
            self.spot_struggle_cutoff_latched |= self.spot_struggle_active && self.spot_struggle_timer.get() < 0.0 && self.pos >= self.section_length * SPOT_STRUGGLE_END_SECTION;
            let group_cutoff = self.spot_struggle_cutoff_latched || (0..field.n).filter(|&j| j != i).any(|j| {
                let peer = field.solvers[j].borrow();
                peer.spot_struggle_started && peer.spot_struggle_cutoff_latched
            });
            if self.spot_struggle_active && (self.spot_struggle_distance_exited
                    || self.spot_struggle_timer.get() >= 0.0 || group_cutoff) {
                self.spot_struggle_active = false;
                self.modifiers_target_speed.add(-self.spot_struggle_modifier);
                self.retail_target_modifiers.remove(1);
            }
            return;
        }
        // Python's `max(group, key=...)` returns the *first* maximal
        // element on ties; `Iterator::max_by` would return the *last*, so
        // this scans manually with a strict `>` to match Python exactly.
        let mut group = Vec::new();
        let mut race_front = f64::NEG_INFINITY;
        let mut eligible_front_rank = usize::MAX;
        let mut front_idx: Option<usize> = None;
        let mut front_pos = f64::NEG_INFINITY;
        for j in 0..field.n {
            // `j == i` is `self`, already mutably borrowed for the duration
            // of this `step()` call -- read its own fields directly rather
            // than re-entering its `RefCell` via `field.solvers[j]`.
            let (j_strategy, j_pos, j_lane, started) = if j == i {
                (strategy, self.pos, self.lane, self.spot_struggle_started)
            } else {
                let s = field.solvers[j].borrow();
                (s.horse.strategy, s.pos, s.lane, s.spot_struggle_started)
            };
            // The distance gate follows the cached overall leader's live position.
            // A pass during this tick takes effect after the next rank update.
            if field.order_of(j) == 1 { race_front = j_pos; }
            if strategy == Strategy::Oonige || j_strategy != Strategy::Oonige {
                eligible_front_rank = eligible_front_rank.min(field.order_of(j));
            }
            if j_strategy == strategy {
                if started { return; }
                group.push((j, j_pos, j_lane));
                // v6: the group front is chosen by the cached (previous-update) field
                // order, not by live position; positions below remain live.
                if front_idx.map_or(true, |f| field.order_of(j) < field.order_of(f)) {
                    front_pos = j_pos;
                    front_idx = Some(j);
                }
            }
        }
        if group.len() < 2 || race_front < 150.0 {
            return;
        }
        let front_idx = front_idx.unwrap();
        // A group's front runner must lead the eligible field in cached order.
        // Ordinary front-runners ignore Runaways for this check, but must lead
        // every other style. Runaways must lead the entire field.
        if field.order_of(front_idx) != eligible_front_rank {
            return;
        }
        let front_lane = group.iter().find(|s| s.0 == front_idx).unwrap().2;
        // The cached group leader can already be beyond the entry cutoff.
        // Each invited partner must still be before it; another eligible
        // partner does not make a late runner eligible too.
        let participants: Vec<_> = group.into_iter().filter(|s|
            front_pos - s.1 <= 3.75 && (front_lane - s.2).abs() <= SPOT_STRUGGLE_LANE_GAP
                && (s.0 == front_idx || s.1 < self.section_length * SPOT_STRUGGLE_CHECK_END_SECTION)).collect();
        if participants.len() < 2 || !participants.iter().any(|s| s.1 < self.section_length * SPOT_STRUGGLE_CHECK_END_SECTION) {
            return;
        }
        for (j, _, _) in participants {
            if j == i { self.start_spot_struggle(); }
            else { field.solvers[j].borrow_mut().start_spot_struggle(); }
        }
    }

    fn start_spot_struggle(&mut self) {
        let guts = self.horse.guts.max(1.0);
        let strat_prof = STRATEGY_PROFICIENCY_MODIFIER_APPROX[self.horse.strategy_aptitude as usize];
        // v69: the Spot Struggle exponent is a float32 constant; retail uses the
        // same helper shape as the duel modifiers (doc 13.186).
        self.spot_struggle_modifier = if self.retail.is_some() {
            let argument = (guts as f32) * 500.0_f32;
            let power = (argument as f64).powf(0.6_f32 as f64) as f32;
            (power * 0.0001_f32) as f64
        } else {
            (500.0 * guts).powf(0.6) * 0.0001
        };
        let duration = (700.0 * guts).powf(0.5) * 0.012 * strat_prof;
        self.spot_struggle_active = true;
        self.spot_struggle_started = true;
        self.spot_struggle_timer.set(-duration);
        self.modifiers_target_speed.add(self.spot_struggle_modifier);
        self.retail_target_modifiers.register(1, self.spot_struggle_modifier as f32);
    }

    fn update_compete_before_spurt(&mut self, dt: f64, field: &RaceField) {
        if self.compete_spurt_active {
            if self.compete_spurt_timer.get() >= 0.0 {
                self.compete_spurt_active = false;
                self.compete_spurt_cooldown.set(-1.0);
                self.modifiers_target_speed.add(-self.compete_spurt_modifier);
                self.retail_target_modifiers.remove(2);
            }
            return;
        }
        let (section_lo, section_hi) = (self.section_length * 10.0, self.section_length * 15.0);
        if !(section_lo <= self.pos && self.pos < section_hi) || self.compete_spurt_cooldown.get() < 0.0 {
            return;
        }
        if self.stamina_keep_active {
            return;
        }
        let i = self.field_index.unwrap();
        let strategy = self.horse.strategy;
        let dist_threshold = compete_dist_threshold(strategy);
        let far_from_lead = field.distance_to_leader(i, self.pos) > dist_threshold;
        let near = field.near_count(i, self.pos, self.lane) > 0;
        if !(far_from_lead || near) {
            return;
        }
        if !self.roll_competition_wisdom(15.0, dt) {
            return;
        }
        let strat_coef = compete_strategy_coef(strategy);
        let mut solo_bonus = 1.0;
        if (strategy == Strategy::Nige || strategy == Strategy::Oonige) && field.is_solo_front_runner(i) {
            solo_bonus = if strategy == Strategy::Oonige { 2.0 } else { 1.1 };
        }
        let modifier = ((self.horse.power.max(0.0) / 1500.0).powf(0.5) * 2.0 + (self.horse.guts.max(0.0) / 3000.0).powf(0.2)) * 0.1 * strat_coef * solo_bonus;
        let stamina_strat_coef = compete_stamina_strategy_coef(strategy);
        let dist_coef = competition_distance_coefficient(self.course.distance);
        let near_factor = if near && !far_from_lead { 0.5 } else { 0.0 };
        let stamina_cost = 20.0 * (stamina_strat_coef * dist_coef + near_factor) * 2.0; // 2s window
        self.compete_spurt_active = true;
        self.compete_spurt_modifier = modifier;
        self.compete_spurt_timer.set(-2.0);
        self.modifiers_target_speed.add(modifier);
        self.retail_target_modifiers.register(2, modifier as f32);
        self.hp.spend(stamina_cost);
    }

    /// Doc: Secure Lead (section 11-15). The doc's real DesirableLead
    /// formula uses a full per-strategy-pair coefficient matrix that wasn't
    /// fully captured in this port's reference notes -- approximated below
    /// with a single flat coefficient instead of the real matrix.
    fn update_secure_lead(&mut self, dt: f64, field: &RaceField) {
        if self.secure_lead_active {
            if self.secure_lead_timer.get() >= 0.0 {
                self.secure_lead_active = false;
                self.secure_lead_cooldown.set(-1.0);
                self.modifiers_target_speed.add(-self.secure_lead_modifier);
                self.retail_target_modifiers.remove(3);
            }
            return;
        }
        let (section_lo, section_hi) = (self.section_length * 10.0, self.section_length * 15.0);
        if !(section_lo <= self.pos && self.pos < section_hi) || self.secure_lead_cooldown.get() < 0.0 {
            return;
        }
        if self.stamina_keep_active {
            return;
        }
        let i = self.field_index.unwrap();
        let gap_behind = match field.distance_to_behind(i, self.pos) {
            Some(g) => g,
            None => return,
        };
        let desirable_lead = SECURE_LEAD_COEF_APPROX + 0.0003 * (self.course.distance + 1000.0);
        if gap_behind >= desirable_lead {
            return;
        }
        if self.rng.random() >= 0.20 * (dt / 2.0) {
            return;
        }
        let strategy = self.horse.strategy;
        let strat_coef = secure_lead_strategy_coef(strategy);
        let mut solo_bonus = 1.0;
        if (strategy == Strategy::Nige || strategy == Strategy::Oonige) && field.is_solo_front_runner(i) {
            solo_bonus = if strategy == Strategy::Oonige { 7.0 } else { 2.0 };
        }
        let modifier = (self.horse.guts.max(0.0) / 2000.0).powf(0.5) * 0.3 * strat_coef * solo_bonus;
        let stamina_strat_coef = secure_lead_stamina_strategy_coef(strategy);
        let dist_coef = competition_distance_coefficient(self.course.distance);
        let stamina_cost = 20.0 * stamina_strat_coef * dist_coef * 2.0;
        self.secure_lead_active = true;
        self.secure_lead_modifier = modifier;
        self.secure_lead_timer.set(-2.0);
        self.modifiers_target_speed.add(modifier);
        self.retail_target_modifiers.register(3, modifier as f32);
        self.hp.spend(stamina_cost);
    }

    pub(crate) fn prepare_retail_duel_start(&mut self, dt: f64, field: &RaceField) {
        if self.retail.is_some() && !self.duel_active && self.compete_fight_count == 0 {
            self.update_dueling(dt, field);
        }
    }

    /// Compete Fight: proximity-only warmup; see docs/hakuraku-backend-review.md.
    fn update_dueling(&mut self, dt: f64, field: &RaceField) {
        // IsStraightLast (0x341bf80) also requires the final-corner flag.
        // Cornerless courses never set it, even with several straight events.
        if self.course.straights.is_empty() || self.course.corners.is_empty() {
            return;
        }
        let last_straight = self.course.straights.last().unwrap();
        let in_final_straight = last_straight.start <= self.pos && self.pos < last_straight.end;
        let i = self.field_index.unwrap();
        let retail_partner = self.retail.is_some();
        if self.duel_active {
            let nearby_participant = (0..field.n).filter(|&j| j != i).any(|j| {
                let other = field.solvers[j].borrow();
                // v13: a finished partner still counts in retail (it keeps moving
                // past the goal); only the 5 m gap ends the duel. Official replays
                // keep the duel speed until the goal in 79/79 engine-exact cases.
                other.compete_fight_count > 0 && other.duel_group == self.duel_group
                    && (retail_partner || other.pos < self.course.distance)
                    && (other.pos - self.pos).abs() <= 5.0
            });
            // Retail duels survive the finish line. A nearby entrant can still
            // join that group during its entry window; HP and separation end it.
            if (!retail_partner && !in_final_straight) || self.hp.hp_ratio_remaining() < 0.05 || !nearby_participant {
                self.duel_active = false;
                self.duel_continuous_timer = 0.0;
                self.duel_target_index = None;
                self.modifiers_target_speed.add(-self.duel_target_speed_modifier);
                self.modifiers_accel.add(-self.duel_accel_modifier);
                self.retail_target_modifiers.remove(4);
                self.retail_accel_modifiers.remove(4);
            }
            return;
        }
        // CompeteFight.TargetOrderPer=50, using percentage-order half-up rounding.
        let top_half = field.order_of(i) <= ((field.n + 1) / 2).max(1);
        // A finisher can complete its warmup while a nearby partner is still
        // on the final straight. The proximity cache retains a pair only while
        // at least one pre-movement position remains inside that straight.
        let eligible_straight = self.retail.as_ref().map_or(in_final_straight, |r|
            r.duel_preupdate_final_straight
                || (r.duel_preupdate_finished && r.final_corner && r.corner == 0));
        if !eligible_straight || self.compete_fight_count > 0 {
            self.duel_continuous_timer = 0.0;
            self.duel_target_index = None;
            if self.retail.is_none() { self.duel_proximity.clear(); }
            return;
        }
        let (pos, lane) = (self.pos, self.lane);
        let retail_cache = self.retail.is_some();
        let mut proximity = if retail_cache { std::mem::take(&mut self.duel_proximity) } else { BTreeMap::new() };
        let mut nearby = None;
        let mut eligible = None;
        let mut ready: Option<usize> = None;
        for j in 0..field.n {
            if j == i {
                continue;
            }
            let other = field.solvers[j].borrow();
            // Retail runners can invite a finished partner while their own
            // entry window is open. Proximity, speed, HP and duel history still
            // apply; finishing alone does not disqualify the selected partner.
            if !retail_cache && (other.pos >= self.course.distance
                    || (other.compete_fight_count > 0 && !other.duel_active)) {
                continue;
            }
            let dist_gap = other.pos - pos;
            let lane_gap = other.lane - lane;
            if if retail_cache { proximity.contains_key(&j) } else { dist_gap.abs() <= 3.0 && lane_gap.abs() <= DUEL_LANE_GAP } {
                if !retail_cache { proximity.insert(j, (self.duel_proximity.get(&j).copied().unwrap_or(0.0) + dt).min(2.0)); }
                nearby.get_or_insert(j);
                if self.hp.hp_ratio_remaining() >= 0.15 && other.hp.hp_ratio_remaining() >= 0.15
                    // v34: retail gates on the observed speed (current-speed
                    // effects included), as Hakuraku rust-v1.0.31 case t1 shows.
                    && (other.retail.as_ref().map_or(other.current_speed, |r| r.duel_preupdate_speed as f64)
                        - self.retail.as_ref().map_or(self.current_speed, |r| r.duel_preupdate_speed as f64)).abs()
                        <= 0.6000000238418579
                    && (top_half || field.order_of(j) <= ((field.n + 1) / 2).max(1)) {
                    eligible.get_or_insert(j);
                    if proximity[&j] >= 2.0
                        && ready.map_or(true, |best| proximity[&j] > proximity[&best]) {
                        ready = Some(j);
                    }
                }
            }
        }
        if retail_cache { nearby = proximity.keys().copied().find(|&j| j < field.n); }
        // Preserve ascending-index ties and longest-standing ready selection.
        self.duel_target_index = ready.or(eligible).or(nearby);
        self.duel_continuous_timer = self.duel_target_index.map_or(0.0, |j| proximity[&j]);
        self.duel_proximity = proximity;
        if ready.is_none() {
            return;
        }
        // Select the longest-standing eligible nearby runner before checking
        // its duel history. An expired partner blocks entry; do not fall back
        // to a newer partner merely because the selected runner cannot rejoin.
        if let Some(j) = self.duel_target_index.filter(|_| retail_cache) {
            let peer = field.solvers[j].borrow();
            if peer.compete_fight_count > 0 && !peer.duel_active { return; }
        }
        let j = self.duel_target_index.expect("duel activation requires a target");
        let mut peer = field.solvers[j].borrow_mut();
        let group = if retail_partner {
            peer.duel_group.or(self.duel_group).unwrap_or(i.min(j))
        } else {
            *peer.duel_group.get_or_insert(i)
        };
        self.duel_group.get_or_insert(group);
        peer.duel_group.get_or_insert(group);
        self.activate_retail_duel();
        // Commit both entrants together: an earlier runner may have selected
        // an expired partner, but can still be invited by this later runner.
        if retail_partner && !peer.duel_active && peer.compete_fight_count == 0 {
            peer.activate_retail_duel();
        }
    }

    fn activate_retail_duel(&mut self) {
        let guts = self.horse.guts.max(1.0);
        if let Some(retail) = &self.retail {
            let modifier = |coefficients: [f32; 3]| {
                let argument = (guts as f32) * coefficients[0];
                let power = (argument as f64).powf(coefficients[1] as f64) as f32;
                (power * coefficients[2]) as f64
            };
            self.duel_target_speed_modifier = modifier(retail.config.duel_speed_coefficients);
            self.duel_accel_modifier = modifier(retail.config.duel_accel_coefficients);
        } else {
            self.duel_target_speed_modifier = (200.0 * guts).powf(0.708) * 0.0001;
            self.duel_accel_modifier = (160.0 * guts).powf(0.59) * 0.0001;
        }
        self.duel_active = true;
        self.compete_fight_count += 1;
        self.modifiers_target_speed.add(self.duel_target_speed_modifier);
        self.modifiers_accel.add(self.duel_accel_modifier);
        self.retail_target_modifiers.register(4, self.duel_target_speed_modifier as f32);
        self.retail_accel_modifiers.register(4, self.duel_accel_modifier as f32);
    }

    /// Deactivate any skills that haven't finished their durations yet
    /// (call at the end of a simulation, in case a skill activated near the
    /// end and the race finished before its duration expired).
    /// Python's race_runner.py sets these as plain attributes on the
    /// already-constructed solver (`solver.on_skill_activate = ...`)
    /// rather than through the builder, so the orchestration layer needs
    /// to reach them post-construction too.
    pub fn set_on_skill_activate(&mut self, hook: SkillHook) {
        self.on_skill_activate = hook;
    }

    pub fn set_on_skill_deactivate(&mut self, hook: SkillHook) {
        self.on_skill_deactivate = hook;
    }

    pub fn cleanup(&mut self) {
        let pending: Vec<(String, Perspective)> = self
            .active_target_speed_skills
            .iter()
            .chain(self.active_current_speed_skills.iter())
            .chain(self.active_accel_skills.iter())
            .chain(self.active_lane_speed_skills.iter())
            .chain(self.active_aux_skills.iter())
            .map(|s| (s.skill_id.clone(), s.perspective))
            .collect();
        for (skill_id, perspective) in pending {
            self.call_deactivate_hook(&skill_id, perspective);
        }
        if self.is_downhill_mode {
            self.call_deactivate_hook("downhill", Perspective::Self_);
        }
    }
}


#[cfg(test)]
mod movement_tests {
    use super::*;
    use crate::activation_conditions::build_conditions;
    use crate::activation_sample_policy::SamplePolicy;
    use crate::course_data::{Corner, Straight};
    use crate::horse_types::Aptitude;
    use crate::hp_policy::GameHpState;
    use crate::race_parameters::{RaceParameters, RaceParametersWithSkillId};
    use std::sync::OnceLock;

    fn solver(pos: f64, lane: f64) -> RaceSolver {
        static COURSE: OnceLock<CourseData> = OnceLock::new();
        let course = COURSE.get_or_init(|| CourseData {
            race_track_id: 1, distance: 2400.0, distance_type: 3, surface: 1, turn: 1,
            course_set_status: vec![], corners: vec![Corner { start: 1600.0, length: 200.0 }],
            straights: vec![Straight { start: 1800.0, end: 2400.0, front_type: 1 }],
            slopes: vec![], max_lane: 1.3, first_move_lane_point: 1600.0, empty_in_move: true,
        });
        let horse = HorseParameters {
            speed: 1000.0, stamina: 1000.0, power: 1000.0, guts: 1000.0, wisdom: 1000.0,
            strategy: Strategy::Nige, distance_aptitude: Aptitude::A,
            surface_aptitude: Aptitude::A, strategy_aptitude: Aptitude::A, raw_stamina: 1000.0, raw_power: 1000.0,
        };
        let hp = HpPolicy::Game(GameHpState::new(course, 1, Rule30CARng::new(2, 0)));
        let mut s = RaceSolver::new(horse, course, Rule30CARng::new(1, 0), vec![], hp, None, None, None, 0, false);
        s.pos = pos;
        s.lane = lane;
        s.lane_target = lane;
        s.current_speed = 20.0;
        s
    }

    #[test]
    fn random_target_selection_precedes_random_ability_value() {
        for seed in 1..32 {
            let field = RaceField::new(vec![solver(100.0, 0.0), solver(99.0, 0.0), solver(98.0, 0.0)]);
            field.solvers[1].borrow_mut().is_kakari = true;
            field.solvers[2].borrow_mut().is_kakari = true;
            let effect = SkillEffect { effect_type: SkillType::TargetSpeed, base_duration: 10.0,
                modifier: -0.2, value_usage: 8, duration_usage: 1, additional_activate: 0,
                target: 20, target_value: 1, target_is_debuff: true };
            let mut oracle = solver(100.0, 0.0);
            oracle.field_index = Some(0);
            oracle.rng = Rule30CARng::new(seed, 0);
            let expected = field.skill_targets(&mut oracle, &effect, effect.modifier);
            effect.activation_modifier(&mut oracle.rng, 0);
            let next = oracle.rng.uniform(100.0);
            let mut owner = field.solvers[0].borrow_mut();
            owner.rng = Rule30CARng::new(seed, 0);
            let observed = owner.apply_activation("999901", Perspective::Self_, SkillRarity::White,
                &[effect], false, Some(&field), false, None, false, 0);
            assert_eq!(observed, vec![Some(expected)]);
            assert_eq!(owner.rng.uniform(100.0), next);
        }
    }

    #[test]
    fn cooldown_keeps_stateful_checks_and_prerequisite_observations() {
        let mut s = solver(100.0, 0.0);
        let table = Box::leak(Box::new(build_conditions()));
        let op = crate::condition_parser::parse(table, "is_activate_any_skill==1").unwrap();
        let params = RaceParameters::default();
        let extra = RaceParametersWithSkillId { params: &params, skill_id: "999903".into() };
        crate::activation_conditions::take_anyskill_nodes();
        let (_, check) = op.apply(&[Region::new(0.0, 2400.0)], s.course, &s.horse, &extra).unwrap();
        let nodes = crate::activation_conditions::take_anyskill_nodes();
        let node = nodes[0];
        s.pending_skills.push(PendingSkill {
            random_activation_order: None, detail_index: 0, priority: 0,
            skill_id: "999903".into(), numeric_skill_id: Some(999903), has_green_tag: false,
            perspective: Perspective::Self_, rarity: SkillRarity::White,
            trigger: Region::new(0.0, 2400.0), extra_condition: check.unwrap(),
            prerequisite: Some(Box::new(|s, _| s.phase == 0)), prerequisite_satisfied: false,
            cooldown: 1.0, cooldown_key: "999903".into(),
            windows: vec![Region::new(0.0, 2400.0)], effects: vec![], anyskill_nodes: nodes,
        });
        s.skill_cooldowns.insert("999903".into(), f64::INFINITY);
        s.activate_count[0] = 1;
        s.process_skill_priority(Some(0), None);
        assert!(s.pending_skills[0].prerequisite_satisfied);
        assert_eq!(s.skill_activation_observers.borrow().get(&node), Some(&(1, 1)));
        assert!(!s.used_skills.contains("999903"));
        // The prior observation must remain consumed after cooldown ends.
        s.skill_cooldowns.remove("999903");
        s.process_skill_priority(Some(0), None);
        assert!(!s.used_skills.contains("999903"));
        assert_eq!(s.skill_activation_observers.borrow().get(&node), Some(&(0, 1)));
    }

    fn enable_retail_for_skill_test(s: &mut RaceSolver) {
        let parameters = crate::json::parse(include_str!("../data/global_parameters.json")).unwrap();
        let config = RetailRuntimeConfig::new(
            parameters.get("parameters").unwrap(),
            Rc::new(RefCell::new(crate::retail_random::RetailRandom::new(1))), None, true,
        ).unwrap();
        let row = crate::retail_initialization::HorseInitialization {
            section_modifiers: vec![0.0; 24], ..Default::default()
        };
        s.enable_retail_runtime(config, &row, 1).unwrap();
    }

    #[test]
    fn sparse_skill_pass_matches_full_scan_observers_after_removals() {
        fn fixture() -> (RaceSolver, Rc<RefCell<Vec<(char, usize)>>>) {
            let mut s = solver(100.0, 0.0);
            enable_retail_for_skill_test(&mut s);
            let trace = Rc::new(RefCell::new(Vec::new()));
            for (i, priority) in [0, 1, 0, 1, 0, 7].into_iter().enumerate() {
                let before = trace.clone();
                let check = trace.clone();
                let id = (999910 + i).to_string();
                s.pending_skills.push(PendingSkill {
                    random_activation_order: None, detail_index: 0, priority,
                    numeric_skill_id: id.parse().ok(), skill_id: id.clone(),
                    has_green_tag: false, perspective: Perspective::Self_, rarity: SkillRarity::Unique,
                    trigger: Region::new(0.0, if i == 3 { 150.0 } else { 2400.0 }),
                    extra_condition: Box::new(move |_, _| {
                        check.borrow_mut().push(('c', i));
                        matches!(i, 1 | 4)
                    }),
                    prerequisite: Some(Box::new(move |s, _| {
                        before.borrow_mut().push(('p', i));
                        i != 4 || s.phase > 0
                    })),
                    prerequisite_satisfied: false, cooldown: 0.0, cooldown_key: id,
                    windows: vec![Region::new(0.0, 2400.0)], effects: vec![], anyskill_nodes: vec![100 + i],
                });
            }
            s.activate_count[0] = 1;
            (s, trace)
        }
        let (mut sparse, sparse_trace) = fixture();
        let (mut full, full_trace) = fixture();
        let passes = [
            (Some(1), 0, 100.0), (Some(0), 0, 100.0), (Some(0), 1, 100.0),
            (Some(1), 1, 200.0), (Some(1), 1, 200.0), (Some(0), 1, 200.0),
            (Some(7), 1, 200.0), (None, 1, 200.0),
        ];
        for (pass, (priority, phase, pos)) in passes.into_iter().enumerate() {
            sparse.phase = phase;
            full.phase = phase;
            sparse.pos = pos;
            full.pos = pos;
            sparse_trace.borrow_mut().clear();
            full_trace.borrow_mut().clear();
            // Include every entry to reproduce the former full traversal and
            // priority filtering while using the same activation implementation.
            let all: Vec<_> = (0..full.pending_skills.len()).collect();
            full.skill_scan = SkillScanIndex {
                pending_len: all.len(), prerequisites: all.clone(),
                priority_zero: all.clone(), priority_one: all,
                windows: Vec::new(),
            };
            full.process_skill_priority(priority, None);
            sparse.process_skill_priority(priority, None);
            assert_eq!(*sparse_trace.borrow(), *full_trace.borrow(), "pass {pass}");
            if pass == 0 {
                assert_eq!(*sparse_trace.borrow(), vec![('p', 1), ('p', 3), ('c', 3), ('c', 1)]);
            } else if pass == 1 {
                assert_eq!(*sparse_trace.borrow(), vec![('p', 0), ('p', 2), ('p', 4), ('c', 2), ('c', 0)]);
            } else if pass == 2 {
                assert_eq!(*sparse_trace.borrow(), vec![('p', 4), ('c', 4), ('c', 2), ('c', 0)]);
            }
            let pending = |s: &RaceSolver| s.pending_skills.iter()
                .map(|sk| (sk.skill_id.clone(), sk.prerequisite_satisfied)).collect::<Vec<_>>();
            assert_eq!(pending(&sparse), pending(&full));
            assert_eq!(*sparse.anyskill_observer.borrow(), *full.anyskill_observer.borrow());
            assert_eq!(*sparse.anyskill_result.borrow(), *full.anyskill_result.borrow());
            assert_eq!(sparse.activate_count, full.activate_count);
            assert_eq!(sparse.activate_count_last_frame, full.activate_count_last_frame);
            assert_eq!(sparse.skill_detail_events, full.skill_detail_events);
            assert_eq!(sparse.skill_cooldowns, full.skill_cooldowns);
            assert_eq!(sparse.used_skills, full.used_skills);
            assert_eq!((sparse.rng.lo, sparse.rng.hi), (full.rng.lo, full.rng.hi));
        }
    }

    #[test]
    fn inactive_skill_windows_match_full_visits_with_stateful_exceptions() {
        fn fixture() -> (RaceSolver, Rc<RefCell<Vec<usize>>>) {
            let mut s = solver(0., 0.);
            enable_retail_for_skill_test(&mut s);
            let trace = Rc::new(RefCell::new(Vec::new()));
            for (i, (start, end)) in [(100., 300.), (0., 50.), (200., 100.),
                (f64::NAN, 300.), (0., f64::NAN), (f64::NEG_INFINITY, f64::INFINITY),
                (0., 500.), (300., 500.)].into_iter().enumerate()
            {
                let check = trace.clone();
                let id = (999910 + i).to_string();
                s.pending_skills.push(PendingSkill {
                    random_activation_order: None, detail_index: usize::from(i == 2), priority: 1,
                    numeric_skill_id: id.parse().ok(), skill_id: id.clone(),
                    has_green_tag: false, perspective: Perspective::Self_,
                    rarity: if i % 2 == 0 { SkillRarity::White } else { SkillRarity::Unique },
                    trigger: Region::new(start, end), windows: vec![Region::new(start, end)],
                    extra_condition: Box::new(move |_, _| { check.borrow_mut().push(i); i % 3 == 0 }),
                    prerequisite: None, prerequisite_satisfied: false,
                    cooldown: 0., cooldown_key: id, effects: vec![],
                    anyskill_nodes: if matches!(i, 1 | 3) { vec![i] } else { vec![] },
                });
            }
            s.activate_count[0] = 2;
            (s, trace)
        }
        for exception in 0..4 {
            let (mut indexed, indexed_trace) = fixture();
            let (mut full, full_trace) = fixture();
            for s in [&mut indexed, &mut full] {
                if exception == 1 { s.pending_removal.insert("999917".into()); }
                if exception == 2 { s.random_gold_reservations.insert("999917".into()); }
                if exception == 3 {
                    let timer = s.new_timer(-10.);
                    s.running_skills.push(RunningSkill {
                        skill_id: "999912".into(), numeric_skill_id: 999912, detail_index: 0,
                        perspective: Perspective::Self_, rarity: SkillRarity::Unique, timer,
                        delayed: vec![(SkillEffect {
                            effect_type: SkillType::TargetSpeed, base_duration: 10., modifier: 0.2,
                            value_usage: 1, duration_usage: 1, additional_activate: 2,
                            target: 1, target_value: 0, target_is_debuff: false,
                        }, 0)], extend_on_overtake: false, extensions: 0, unapplied: 1,
                    });
                    s.unapplied_running_skills = 1;
                }
            }
            for pos in [f64::NAN, f64::NEG_INFINITY, 0., 100., 300., f64::INFINITY] {
                for s in [&mut indexed, &mut full] {
                    s.pos = pos;
                    s.additional_turn_floor = i64::MIN;
                    s.refresh_skill_scan();
                }
                full.skill_scan.windows.clear(); // Reference visits every detail.
                indexed.process_skill_priority(Some(1), None);
                full.process_skill_priority(Some(1), None);
                assert_eq!(*indexed_trace.borrow(), *full_trace.borrow());
                let ids = |s: &RaceSolver| s.pending_skills.iter().map(|sk| &sk.skill_id)
                    .cloned().collect::<Vec<_>>();
                assert_eq!(ids(&indexed), ids(&full));
                assert_eq!(indexed.additional_turn_floor, full.additional_turn_floor);
                assert_eq!(indexed.unapplied_running_skills, full.unapplied_running_skills);
                assert_eq!(indexed.retail_target_modifiers.apply(0.), full.retail_target_modifiers.apply(0.));
                assert_eq!(*indexed.anyskill_observer.borrow(), *full.anyskill_observer.borrow());
                assert_eq!(*indexed.anyskill_result.borrow(), *full.anyskill_result.borrow());
                assert_eq!(indexed.activate_count, full.activate_count);
                assert_eq!(indexed.skill_detail_events, full.skill_detail_events);
                assert_eq!(indexed.skill_cooldowns, full.skill_cooldowns);
                assert_eq!((indexed.rng.lo, indexed.rng.hi), (full.rng.lo, full.rng.hi));
            }
        }
    }

    #[test]
    fn empty_priority_zero_still_flushes_additional_activations_and_reservations() {
        let mut s = solver(100.0, 0.0);
        enable_retail_for_skill_test(&mut s);
        let effect = SkillEffect {
            effect_type: SkillType::TargetSpeed, base_duration: 10.0, modifier: 0.2,
            value_usage: 1, duration_usage: 1, additional_activate: 2,
            target: 1, target_value: 0, target_is_debuff: false,
        };
        let timer = s.new_timer(-10.0);
        s.running_skills.push(RunningSkill {
            skill_id: "999910".into(), numeric_skill_id: 999910, detail_index: 0,
            perspective: Perspective::Self_, rarity: SkillRarity::Unique, timer,
            delayed: vec![(effect, 0)], extend_on_overtake: false, extensions: 0, unapplied: 1,
        });
        s.unapplied_running_skills = 1;
        s.additional_turn_floor = 42;
        s.random_any_request = 1.0;
        s.random_gold_request = 1.0;
        s.random_gold_reservations.insert("999911".into());
        s.activate_count_last_frame = 3;
        s.process_skill_priority(Some(1), None);
        assert_eq!(s.additional_turn_floor, 42);
        assert_eq!(s.unapplied_running_skills, 1);
        assert_eq!(s.random_gold_reservations.len(), 1);
        assert_eq!((s.random_any_request, s.random_gold_request), (1.0, 1.0));
        s.process_skill_priority(Some(0), None);
        assert_eq!(s.additional_turn_floor, i64::MAX);
        assert_eq!(s.unapplied_running_skills, 0);
        assert_eq!(s.running_skills[0].delayed[0].1, 1);
        assert_eq!(s.additional_target_events.len(), 1);
        assert!(s.random_gold_reservations.is_empty());
        assert_eq!((s.random_any_request, s.random_gold_request), (0.0, 0.0));
        assert_eq!(s.activate_count_last_frame, 3);
    }

    #[test]
    fn leader_gap_duration_preserves_recorded_boundary_and_clamps() {
        let effect = SkillEffect {
            effect_type: SkillType::TargetSpeed, base_duration: 3.0, modifier: 0.05,
            value_usage: 1, duration_usage: 2, additional_activate: 0,
            target: 1, target_value: 0, target_is_debuff: false,
        };
        // Captured 2200 m activation: widening the gap division produces
        // 73758 units instead of the observed 73759 units.
        let coefficient = effect.duration_coefficient(None, Some(19.8475341796875), true);
        let duration = (3.0_f32 * (2200.0_f32 / 1000.0)) * coefficient as f32;
        assert_eq!((duration as f64 * 10000.0) as i32, 73759);
        assert_eq!(effect.duration_coefficient(None, Some(-1.0), true), 0.8_f32 as f64);
        assert_eq!(effect.duration_coefficient(None, Some(50.0), true), 1.6_f32 as f64);
        assert_eq!(effect.duration_coefficient(None, Some(100.0), true), 1.6_f32 as f64);
        assert_eq!(effect.duration_coefficient(None, None, true), 1.0);
    }

    #[test]
    fn final_skill_turn_accepts_earlier_activations_and_rejects_later_ones() {
        for (queued, incoming, count, expected) in [
            (0, false, 0, 0), (0, true, 0, 1), (1, false, 0, 1),
            (1, true, 0, 2), (1, true, 1, 1), (1, true, 2, 0),
        ] {
            let mut s = solver(100., 0.);
            enable_retail_for_skill_test(&mut s);
            let effect = SkillEffect {
                effect_type: SkillType::TargetSpeed, base_duration: 0.01, modifier: 0.02675,
                value_usage: 1, duration_usage: 1, additional_activate: 3,
                target: 1, target_value: 0, target_is_debuff: false,
            };
            // The receiver expired in PreUpdate, but the running detail has
            // not taken its final turn in the ascending-id skill pass yet.
            let timer = s.new_timer(0.05);
            s.running_skills.push(RunningSkill {
                skill_id: "110211".into(), numeric_skill_id: 110211, detail_index: 0,
                perspective: Perspective::Self_, rarity: SkillRarity::Unique, timer,
                delayed: vec![(effect, count)], extend_on_overtake: false,
                extensions: 0, unapplied: queued,
            });
            s.unapplied_running_skills = usize::from(queued > 0);
            s.waiting_skill_cooldowns.insert("110211".into(), ("110211".into(), 0, 30.));
            s.process_skill_history(None);
            s.start_expired_skill_cooldowns();
            assert_eq!(s.running_skills.len(), 1);
            assert!(s.active_target_speed_skills.is_empty());
            if incoming { s.additional_skill_event(2, None, None); }
            s.apply_unapplied_additional(110210, None);
            assert!(s.active_target_speed_skills.is_empty());
            s.apply_unapplied_additional(110211, None);
            assert!(s.running_skills.is_empty());
            assert_eq!(s.unapplied_running_skills, 0);
            assert_eq!(s.active_target_speed_skills.len(), expected);
            assert_eq!(s.additional_target_events.len(), expected);
            assert!(!s.waiting_skill_cooldowns.contains_key("110211"));
            assert!(s.retail.as_ref().unwrap().skill_cooldowns.contains_key("110211"));
            // Later activations cannot resurrect the completed detail.
            s.additional_skill_event(2, None, None);
            s.apply_unapplied_additional(i64::MAX, None);
            assert_eq!(s.active_target_speed_skills.len(), expected);
            assert_eq!(s.unapplied_running_skills, 0);
            if expected > 0 {
                let bonus = &s.active_target_speed_skills[0];
                assert_eq!(bonus.modifier, 0.02675);
                assert!(bonus.duration_timer.get() < 0.);
                assert!(bonus.duration_timer.get() > -0.0666);
                for bonus in &s.active_target_speed_skills { bonus.duration_timer.set(0.); }
                s.expire_skill_effects(None);
                assert!(s.active_target_speed_skills.is_empty());
            }
        }
    }

    #[test]
    fn final_straight_condition_only_checks_the_entry_tick() {
        for final_corner in [false, true] {
            let mut s = solver(1799.0, 0.0);
            enable_retail_for_skill_test(&mut s);
            s.retail.as_mut().unwrap().course_events_ready = true;
            s.retail.as_mut().unwrap().final_corner = final_corner;
            let table = Box::leak(Box::new(build_conditions()));
            let op = crate::condition_parser::parse(table, "is_last_straight_onetime==1").unwrap();
            let params = RaceParameters::default();
            let extra = RaceParametersWithSkillId { params: &params, skill_id: "900671".into() };
            let (regions, check) = op.apply(&[Region::new(0.0, 2400.0)], s.course, &s.horse, &extra).unwrap();
            let check = check.unwrap();
            s.prepare_retail_tick(0.0666, None, false);
            assert!(!check(&s, None));
            s.pos = 1800.0;
            s.prepare_retail_tick(0.0666, None, false);
            assert_eq!(check(&s, None), final_corner);
            // The runner remains inside the old 10 m window, but a condition
            // that becomes satisfied later cannot trigger a delayed entry.
            s.pos = 1801.0;
            assert!(regions.iter().any(|r| r.start <= s.pos && s.pos < r.end));
            s.prepare_retail_tick(0.0666, None, false);
            assert!(!check(&s, None));
        }
    }

    #[test]
    fn late_corner_overtakes_use_flags_from_before_movement() {
        for (position, phase, corner, expected) in [
            (1600.5, 1, 4, 0), (1799.5, 2, 4, 1),
            (1800.5, 2, 4, 1), (1800.5, 2, 0, 0),
        ] {
            let mut s = solver(position, 0.0);
            enable_retail_for_skill_test(&mut s);
            let retail = s.retail.as_mut().unwrap();
            retail.phase = phase;
            retail.corner = corner;
            s.record_overtakes(1);
            assert_eq!(s.late_corner_overtakes, expected);
            assert_eq!(s.pending_overtakes, 1);
        }
    }

    #[test]
    fn rank_continuity_reads_history_before_new_order_is_published() {
        let mut runners: Vec<_> = (0..10).map(|i| solver(100.0 - i as f64, 0.0)).collect();
        for s in &mut runners { enable_retail_for_skill_test(s); }
        let field = RaceField::new(runners);
        for s in &field.solvers { s.borrow().accumulatetime.set(6.0); }
        field.update();
        let table = Box::leak(Box::new(build_conditions()));
        let op = crate::condition_parser::parse(table, "order_rate_in20_continue==1").unwrap();
        let params = RaceParameters::default();
        let extra = RaceParametersWithSkillId { params: &params, skill_id: "900261".into() };
        let check = {
            let s = field.solvers[1].borrow();
            op.apply(&[Region::new(0.0, 2400.0)], s.course, &s.horse, &extra).unwrap().1.unwrap()
        };
        assert!(check(&field.solvers[1].borrow(), Some(&field)));
        field.solvers[1].borrow_mut().pos = 80.0;
        field.update();
        assert_eq!(field.order_of(1), 10);
        assert!(check(&field.solvers[1].borrow(), Some(&field)));
        field.update();
        assert!(!check(&field.solvers[1].borrow(), Some(&field)));
        field.solvers[1].borrow_mut().pos = 99.0;
        field.update();
        field.update();
        assert_eq!(field.order_of(1), 2);
        assert!(!check(&field.solvers[1].borrow(), Some(&field)));
    }

    fn force_lane(s: &mut RaceSolver, target: f64) {
        s.active_aux_skills.push(ActiveSkill {
            token: 0, skill_id: "test-lane".into(), perspective: Perspective::Self_,
            duration_timer: new_timer_value(-1000.0), modifier: target,
            natural_deceleration: false, effect_type: SkillType::SetTargetLane,
        });
    }

    #[test]
    fn null_lane_controller_keeps_target_and_ignores_fixed_lane_skill() {
        let mut s = solver(300.0, 0.3);
        let mut course = s.course.clone();
        course.empty_in_move = false;
        s.course = Box::leak(Box::new(course));
        s.set_starting_lane(0.7);
        force_lane(&mut s, 0.2);
        let field = RaceField::new(vec![s]);
        let mut s = field.solvers[0].borrow_mut();
        let rng = (s.rng.lo, s.rng.hi);
        s.update_lane(0.0666, Some(&field));
        assert_eq!(s.lane, 0.7);
        assert_eq!(s.lane_target, 0.7);
        // Null getter preserves the old target even if current lane differs.
        s.lane = 0.6;
        s.update_lane(0.0666, Some(&field));
        assert_eq!(s.lane_target, 0.7);
        assert!(s.lane > 0.6 && s.lane < 0.7);
        assert_eq!((s.rng.lo, s.rng.hi), rng);
    }

    #[test]
    fn extra_pacing_checks_all_ahead_and_exits_after_passing_later_style() {
        let mut later = solver(102.0, 0.2);
        later.horse.strategy = Strategy::Sasi;
        let field = RaceField::new(vec![solver(100.0, 0.0), solver(101.0, 0.1), later]);
        {
            let mut s = field.solvers[0].borrow_mut();
            let rng = (s.rng.lo, s.rng.hi);
            s.update_position_keep(0.0666, Some(&field));
            assert!(s.pos_keep_mode == Some(PosKeepMode::PaceUpEx));
            assert_eq!(s.pos_keep_speed_coef, 2.0);
            assert_eq!(s.pos_keep_effect_end, 200.0);
            assert_eq!((s.rng.lo, s.rng.hi), rng);
        }
        field.solvers[2].borrow_mut().pos = 99.0;
        field.update();
        let mut s = field.solvers[0].borrow_mut();
        s.update_position_keep(0.0666, Some(&field));
        assert!(s.pos_keep_mode.is_none());
        assert_eq!(s.pos_keep_speed_coef, 1.0);
        assert_eq!(s.pos_keep_cooldown, 1.0);
    }

    #[test]
    fn rushing_mode_precedes_extra_catchup_and_short_course_factor_is_bounded() {
        let mut later = solver(101.0, 0.2);
        later.horse.strategy = Strategy::Sasi;
        let field = RaceField::new(vec![solver(100.0, 0.0), later, solver(99.0, 0.1)]);
        let mut s = field.solvers[0].borrow_mut();
        s.is_kakari = true;
        s.rushing_mode = 4;
        s.update_position_keep(0.0666, Some(&field));
        assert!(s.pos_keep_mode == Some(PosKeepMode::SpeedUp));
        assert_eq!(s.pos_keep_speed_coef, 1.04);
        assert_eq!(position_keep_min_threshold(Strategy::Sasi, 800.0), 6.5);
        assert_eq!(position_keep_max_threshold(Strategy::Senkou, 800.0), 5.0);
    }

    #[test]
    fn cornerless_course_cannot_open_a_duel_on_its_last_straight() {
        let mut first = solver(2000.0, 0.0);
        let mut second = solver(1999.0, 0.1);
        let mut course = first.course.clone();
        course.corners.clear();
        let course = Box::leak(Box::new(course));
        first.course = course;
        second.course = course;
        let field = RaceField::new(vec![first, second]);
        for _ in 0..60 {
            field.solvers[0].borrow_mut().update_dueling(0.0666, &field);
            field.solvers[1].borrow_mut().update_dueling(0.0666, &field);
        }
        for runner in &field.solvers {
            let runner = runner.borrow();
            assert!(!runner.duel_active);
            assert_eq!(runner.compete_fight_count, 0);
        }
    }

    #[test]
    fn experimental_duel_groups_do_not_sustain_unrelated_participants() {
        let field = RaceField::new(vec![solver(2000.0, 0.0), solver(1999.0, 0.1),
            solver(2001.0, 0.6), solver(2000.0, 0.7)]);
        for i in [0, 2, 1, 3] {
            field.solvers[i].borrow_mut().update_dueling(2.0, &field);
            assert!(field.solvers[i].borrow().duel_active);
        }
        assert_eq!(field.solvers[0].borrow().duel_group, Some(0));
        assert_eq!(field.solvers[1].borrow().duel_group, Some(0));
        assert_eq!(field.solvers[2].borrow().duel_group, Some(2));
        assert_eq!(field.solvers[3].borrow().duel_group, Some(2));
        field.solvers[1].borrow_mut().pos = 1980.0;
        field.solvers[0].borrow_mut().update_dueling(0.0666, &field);
        assert!(!field.solvers[0].borrow().duel_active);
        assert!(field.solvers[2].borrow().duel_active);
    }

    #[test]
    fn duel_lane_and_distance_boundaries() {
        for lane in [0.249, 0.25, 0.251, 0.9, -0.249, -0.25] {
            for gap in [2.999, 3.0] {
                let field = RaceField::new(vec![solver(2000.0, 0.0), solver(2000.0 - gap, lane)]);
                let mut s = field.solvers[0].borrow_mut();
                s.update_dueling(2.0, &field);
                assert_eq!(s.duel_active, lane.abs() <= 0.25 && gap <= 3.0);
            }
        }
    }

    #[test]
    fn duel_warmup_same_target_speed_and_count() {
        let field = RaceField::new(vec![solver(2000.0, 0.0), solver(1999.0, 0.1), solver(1900.0, 0.1)]);
        field.solvers[0].borrow_mut().update_dueling(1.0, &field);
        field.solvers[1].borrow_mut().lane = 1.0;
        field.solvers[2].borrow_mut().pos = 1999.0;
        field.update();
        field.solvers[0].borrow_mut().update_dueling(1.0, &field);
        assert_eq!(field.solvers[0].borrow().duel_target_index, Some(2));
        assert_eq!(field.solvers[0].borrow().duel_continuous_timer, 1.0);
        assert!(!field.solvers[0].borrow().duel_active);
        field.solvers[2].borrow_mut().current_speed = 21.0;
        field.solvers[0].borrow_mut().update_dueling(1.0, &field);
        assert_eq!(field.solvers[0].borrow().duel_continuous_timer, 2.0);
        assert!(!field.solvers[0].borrow().duel_active);
        field.solvers[2].borrow_mut().current_speed = 20.0;
        field.solvers[0].borrow_mut().update_dueling(1.0, &field);
        assert_eq!(field.solvers[0].borrow().compete_fight_count, 1);
        field.solvers[0].borrow_mut().update_dueling(5.0, &field);
        assert_eq!(field.solvers[0].borrow().compete_fight_count, 1);
    }

    #[test]
    fn duel_warmup_checks_order_and_retains_eligible_rival() {
        let field = RaceField::new(vec![solver(2000.0, 0.0), solver(1999.0, 0.1), solver(1998.0, 0.1), solver(1900.0, 0.0)]);
        field.solvers[1].borrow_mut().current_speed = 21.0;
        field.solvers[0].borrow_mut().update_dueling(0.5, &field);
        assert_eq!(field.solvers[0].borrow().duel_target_index, Some(2));
        field.solvers[1].borrow_mut().current_speed = 20.0;
        field.solvers[0].borrow_mut().update_dueling(0.5, &field);
        assert_eq!(field.solvers[0].borrow().duel_target_index, Some(2));
        for (i, pos) in [(1, 2001.0), (2, 2002.0), (3, 2010.0)] {
            field.solvers[i].borrow_mut().pos = pos;
        }
        field.update();
        field.solvers[0].borrow_mut().update_dueling(1.0, &field);
        assert_eq!(field.order_of(0), 4);
        assert!(field.solvers[0].borrow().duel_active);
        assert_eq!(field.solvers[0].borrow().compete_fight_count, 1);
    }

    #[test]
    fn retail_duel_remains_active_past_the_finish_line() {
        for retail in [false, true] {
            let mut solvers = vec![solver(2401.0, 0.0), solver(2400.5, 0.1)];
            for s in &mut solvers {
                if retail { enable_retail_for_skill_test(s); }
                s.duel_group = Some(0);
                s.activate_retail_duel();
            }
            let field = RaceField::new(solvers);
            field.solvers[0].borrow_mut().update_dueling(0.0666, &field);
            assert_eq!(field.solvers[0].borrow().duel_active, retail);
            assert_eq!(field.solvers[0].borrow().compete_fight_count, 1);
        }
    }

    #[test]
    fn retail_duel_after_finish_still_ends_for_low_hp_or_separation() {
        for low_hp in [false, true] {
            let mut solvers = vec![solver(2405.0, 0.0), solver(2400.0, 0.1)];
            for s in &mut solvers {
                enable_retail_for_skill_test(s);
                s.duel_group = Some(0);
                s.activate_retail_duel();
            }
            let field = RaceField::new(solvers);
            field.solvers[0].borrow_mut().update_dueling(0.0666, &field);
            assert!(field.solvers[0].borrow().duel_active); // Exactly 5 m still holds.
            if low_hp {
                if let HpPolicy::Game(hp) = &mut field.solvers[0].borrow_mut().hp {
                    hp.hp = hp.max_hp * 0.04;
                }
            } else {
                field.solvers[0].borrow_mut().pos = 2405.001;
            }
            field.solvers[0].borrow_mut().update_dueling(0.0666, &field);
            assert!(!field.solvers[0].borrow().duel_active);
            assert_eq!(field.solvers[0].borrow().compete_fight_count, 1);
        }
    }

    #[test]
    fn retail_duel_can_invite_a_finished_partner_with_ready_proximity() {
        for retail in [false, true] {
            let mut solvers = vec![solver(2399.0, 0.0), solver(2401.0, 0.1)];
            if retail {
                for s in &mut solvers {
                    enable_retail_for_skill_test(s);
                }
            }
            let field = RaceField::new(solvers);
            if retail {
                let snapshot = [(2399.0, 0.0, true), (2401.0, 0.1, false)];
                for cell in &field.solvers {
                    cell.borrow_mut().collect_retail_duel_proximity(&snapshot, 0.0666);
                }
                field.solvers[0].borrow_mut().duel_proximity.insert(1, 2.0);
            }
            field.solvers[0].borrow_mut().update_dueling(2.0, &field);
            assert_eq!(field.solvers[0].borrow().duel_active, retail);
            assert_eq!(field.solvers[1].borrow().duel_active, retail);
            assert_eq!(field.solvers[0].borrow().compete_fight_count, if retail { 1 } else { 0 });
        }
    }

    #[test]
    fn finished_runner_is_not_a_duel_target() {
        let field = RaceField::new(vec![solver(2398.0, 0.0), solver(2400.0, 0.1), solver(2300.0, 0.0), solver(2290.0, 0.0)]);
        let mut s = field.solvers[0].borrow_mut();
        s.update_dueling(3.0, &field);
        assert_eq!(s.compete_fight_count, 0);
        assert_eq!(s.duel_continuous_timer, 0.0);
    }

    #[test]
    fn duel_proximity_survives_recovery_and_tracks_multiple_partners() {
        let field = RaceField::new(vec![solver(2000.0, 0.0), solver(1999.0, 0.1), solver(1998.0, 0.1)]);
        field.solvers[2].borrow_mut().current_speed = 22.0;
        if let HpPolicy::Game(hp) = &mut field.solvers[0].borrow_mut().hp { hp.hp = hp.max_hp * 0.14; }
        field.solvers[0].borrow_mut().update_dueling(1.9, &field);
        field.solvers[1].borrow_mut().lane = 1.0;
        field.solvers[2].borrow_mut().current_speed = 20.0;
        if let HpPolicy::Game(hp) = &mut field.solvers[0].borrow_mut().hp { hp.hp = hp.max_hp * 0.15; }
        field.solvers[0].borrow_mut().update_dueling(0.2, &field);
        assert!(field.solvers[0].borrow().duel_active);
        assert_eq!(field.solvers[0].borrow().duel_target_index, Some(2));
    }

    #[test]
    fn duel_partner_hp_and_former_participant_exit_rules() {
        let field = RaceField::new(vec![solver(2000.0, 0.0), solver(1999.0, 0.1)]);
        if let HpPolicy::Game(hp) = &mut field.solvers[1].borrow_mut().hp { hp.hp = hp.max_hp * 0.14; }
        field.solvers[0].borrow_mut().update_dueling(2.0, &field);
        assert!(!field.solvers[0].borrow().duel_active);
        if let HpPolicy::Game(hp) = &mut field.solvers[1].borrow_mut().hp { hp.hp = hp.max_hp * 0.15; }
        field.solvers[0].borrow_mut().update_dueling(0.1, &field);
        field.solvers[1].borrow_mut().update_dueling(2.0, &field);
        if let HpPolicy::Game(hp) = &mut field.solvers[1].borrow_mut().hp { hp.hp = hp.max_hp * 0.04; }
        field.solvers[1].borrow_mut().update_dueling(0.1, &field);
        field.solvers[0].borrow_mut().update_dueling(0.1, &field);
        assert!(field.solvers[0].borrow().duel_active);
        field.solvers[1].borrow_mut().pos = 1995.0;
        field.solvers[0].borrow_mut().update_dueling(0.1, &field);
        assert!(!field.solvers[0].borrow().duel_active);
        field.solvers[1].borrow_mut().pos = 1999.0;
        field.solvers[0].borrow_mut().update_dueling(3.0, &field);
        assert_eq!(field.solvers[0].borrow().compete_fight_count, 1);
        assert!(!field.solvers[0].borrow().duel_active);
    }

    #[test]
    fn spot_struggle_distance_gate_waits_for_cached_overall_leader() {
        let mut runaway = solver(148.56, 0.0);
        runaway.horse.strategy = Strategy::Oonige;
        let field = RaceField::new(vec![solver(148.62, 0.0), solver(148.0, 0.1), runaway]);
        field.solvers[0].borrow_mut().pos = 149.99;
        field.solvers[1].borrow_mut().pos = 149.4;
        field.solvers[2].borrow_mut().pos = 150.02;
        // The new live leader has crossed 150 m, but the published leader has not.
        field.solvers[0].borrow_mut().update_spot_struggle(&field);
        assert!(!field.solvers[0].borrow().spot_struggle_started);
        assert!(!field.solvers[1].borrow().spot_struggle_started);
        // Publishing the new leader opens the distance gate for the regular group.
        field.update();
        field.solvers[0].borrow_mut().update_spot_struggle(&field);
        assert!(field.solvers[0].borrow().spot_struggle_started);
        assert!(field.solvers[1].borrow().spot_struggle_started);
        assert!(!field.solvers[2].borrow().spot_struggle_started);
    }

    #[test]
    fn spot_struggle_distance_gate_reads_live_position_at_exact_boundary() {
        let field = RaceField::new(vec![solver(149.0, 0.0), solver(148.0, 0.1)]);
        field.solvers[0].borrow_mut().pos = 149.999;
        field.solvers[0].borrow_mut().update_spot_struggle(&field);
        assert!(!field.solvers[0].borrow().spot_struggle_started);
        field.solvers[0].borrow_mut().pos = 150.0;
        // A rank refresh is unnecessary when the leader's identity is unchanged.
        field.solvers[0].borrow_mut().update_spot_struggle(&field);
        assert!(field.solvers[0].borrow().spot_struggle_started);
        assert!(field.solvers[1].borrow().spot_struggle_started);
    }

    #[test]
    fn spot_struggle_waits_for_cached_lead_over_other_styles() {
        let mut leader = solver(201.0, 0.3);
        leader.horse.strategy = Strategy::Senkou;
        let field = RaceField::new(vec![solver(200.0, 0.0), solver(199.0, 0.1), leader]);
        field.solvers[0].borrow_mut().update_spot_struggle(&field);
        assert!(!field.solvers[0].borrow().spot_struggle_started);
        field.solvers[0].borrow_mut().pos = 202.0;
        field.solvers[1].borrow_mut().pos = 201.5;
        // Live positions have crossed; eligibility waits for rank publication.
        field.solvers[0].borrow_mut().update_spot_struggle(&field);
        assert!(!field.solvers[0].borrow().spot_struggle_started);
        field.update();
        field.solvers[0].borrow_mut().update_spot_struggle(&field);
        assert!(field.solvers[0].borrow().spot_struggle_started);
        assert!(field.solvers[1].borrow().spot_struggle_started);
        assert!(!field.solvers[2].borrow().spot_struggle_started);
    }

    #[test]
    fn spot_struggle_ignores_only_runaways_in_regular_front_runner_ranks() {
        for other_style_leads in [false, true] {
            let mut runaway = solver(220.0, 0.0);
            runaway.horse.strategy = Strategy::Oonige;
            let mut other = solver(if other_style_leads { 201.0 } else { 180.0 }, 0.3);
            other.horse.strategy = Strategy::Senkou;
            let field = RaceField::new(vec![runaway, solver(200.0, 0.0), solver(199.0, 0.1), other]);
            field.solvers[1].borrow_mut().update_spot_struggle(&field);
            assert_eq!(field.solvers[1].borrow().spot_struggle_started, !other_style_leads);
            assert_eq!(field.solvers[2].borrow().spot_struggle_started, !other_style_leads);
            assert!(!field.solvers[0].borrow().spot_struggle_started);
        }
    }

    #[test]
    fn spot_struggle_runaways_must_lead_the_entire_field() {
        let mut solvers = vec![solver(201.0, 0.3), solver(200.0, 0.0), solver(199.0, 0.1)];
        solvers[1].horse.strategy = Strategy::Oonige;
        solvers[2].horse.strategy = Strategy::Oonige;
        let field = RaceField::new(solvers);
        field.solvers[1].borrow_mut().update_spot_struggle(&field);
        assert!(!field.solvers[1].borrow().spot_struggle_started);
        field.solvers[0].borrow_mut().pos = 198.0;
        field.update();
        field.solvers[1].borrow_mut().update_spot_struggle(&field);
        assert!(field.solvers[1].borrow().spot_struggle_started);
        assert!(field.solvers[2].borrow().spot_struggle_started);
    }

    #[test]
    fn spot_struggle_group_start_and_expiration_are_once_per_style() {
        let field = RaceField::new(vec![solver(200.0, 0.0), solver(199.0, 0.1), solver(180.0, 0.1)]);
        field.solvers[0].borrow_mut().update_spot_struggle(&field);
        assert!(field.solvers[0].borrow().spot_struggle_active);
        assert!(field.solvers[1].borrow().spot_struggle_active);
        assert!(!field.solvers[2].borrow().spot_struggle_active);
        field.solvers[1].borrow().spot_struggle_timer.set(0.0);
        field.solvers[1].borrow_mut().update_spot_struggle(&field);
        field.solvers[0].borrow_mut().update_spot_struggle(&field);
        assert!(field.solvers[0].borrow().spot_struggle_active);
        // v71: a partner exactly 5 m away still holds the struggle; beyond 5 m it ends.
        field.solvers[1].borrow_mut().pos = 195.0;
        field.solvers[1].borrow_mut().update_spot_struggle(&field);
        field.solvers[0].borrow_mut().update_spot_struggle(&field);
        assert!(field.solvers[0].borrow().spot_struggle_active);
        field.solvers[1].borrow_mut().pos = 194.9;
        field.solvers[1].borrow_mut().update_spot_struggle(&field);
        field.solvers[0].borrow_mut().update_spot_struggle(&field);
        assert!(!field.solvers[0].borrow().spot_struggle_active);
        field.solvers[2].borrow_mut().pos = 199.0;
        field.solvers[2].borrow_mut().update_spot_struggle(&field);
        assert!(!field.solvers[2].borrow().spot_struggle_active);
    }

    #[test]
    fn sixth_can_duel_with_fifth_but_seventh_cannot_duel_with_sixth() {
        for rank in [6usize, 7] {
            let mut solvers = vec![solver(2000.0, 0.0)];
            solvers.extend((0..rank - 1).map(|i| solver(2001.0 + i as f64 * 10.0, 0.1)));
            solvers.extend((0..9 - rank).map(|i| solver(1900.0 - i as f64 * 10.0, 0.0)));
            let field = RaceField::new(solvers);
            assert_eq!(field.order_of(0), rank);
            let mut s = field.solvers[0].borrow_mut();
            s.update_dueling(2.0, &field);
            assert_eq!(s.duel_active, rank == 6);
        }
    }

    #[test]
    fn duel_condition_checks_accumulated_real_count_not_random_position() {
        let field = RaceField::new(vec![solver(2000.0, 0.0), solver(1999.0, 0.1)]);
        let conditions = build_conditions();
        let condition = &conditions["compete_fight_count"];
        assert!(matches!(condition.sample_policy, SamplePolicy::Immediate));
        let extra = RaceParametersWithSkillId { params: &RaceParameters::default(), skill_id: "duel-test".into() };
        let mut s = field.solvers[0].borrow_mut();
        let (regions, check) = (condition.filter_gt)(&[Region::new(0.0, 2400.0)], 0.0, s.course, &s.horse, &extra).unwrap();
        assert_eq!((regions[0].start, regions[0].end), (0.0, 2400.0));
        let check = check.unwrap();
        let (_, twice) = (condition.filter_gt)(&regions, 1.0, s.course, &s.horse, &extra).unwrap();
        assert!(!check(&s, Some(&field)));
        s.update_dueling(2.0, &field);
        assert!(check(&s, Some(&field)));
        assert!(!twice.unwrap()(&s, Some(&field)));
        if let HpPolicy::Game(hp) = &mut s.hp { hp.hp = hp.max_hp * 0.04; }
        s.update_dueling(1.0, &field);
        assert!(!s.duel_active);
        assert!(check(&s, Some(&field)));
    }

    #[test]
    fn spot_struggle_lane_and_entry_cutoff() {
        for lane in [0.1649, 0.1651, 0.9] {
            for pos in [599.0, 600.0] {
                let field = RaceField::new(vec![solver(pos, 0.0), solver(pos + 1.0, lane)]);
                let mut s = field.solvers[0].borrow_mut();
                s.update_spot_struggle(&field);
                assert_eq!(s.spot_struggle_active, lane < 0.165 && pos < 600.0);
            }
        }
        let field = RaceField::new(vec![solver(599.0, 0.0), solver(600.0, 0.1)]);
        let mut s = field.solvers[0].borrow_mut();
        s.update_spot_struggle(&field);
        assert!((s.spot_struggle_timer.get() + (700.0 * s.horse.guts).sqrt() * 0.012).abs() < 1e-10);
        s.pos = 601.0;
        s.update_spot_struggle(&field);
        assert!(s.spot_struggle_active);
        s.pos = 900.0;
        s.update_spot_struggle(&field);
        assert!(!s.spot_struggle_active);
        assert_eq!(s.modifiers_target_speed.acc + s.modifiers_target_speed.err, 0.0);
    }

    #[test]
    fn boxed_runner_cancels_stale_target() {
        let field = RaceField::new(vec![solver(500.0, 0.4), solver(500.5, 0.4), solver(500.0, 0.35), solver(500.0, 0.45)]);
        let mut s = field.solvers[0].borrow_mut();
        s.lane_target = 0.7;
        s.update_lane(0.1, Some(&field));
        assert_eq!(s.lane, 0.4);
        assert_eq!(s.lane_target, 0.4);
        assert!(!s.moved_lane_this_tick);
    }

    #[test]
    fn forced_lane_step_sweeps_side_envelope_in_both_directions() {
        for direction in [-1.0, 1.0] {
            let other_lane = 0.6 + direction * 0.25;
            let field = RaceField::new(vec![solver(2000.0, 0.6), solver(2000.0, other_lane)]);
            let mut s = field.solvers[0].borrow_mut();
            force_lane(&mut s, if direction < 0.0 { 0.0 } else { 1.3 });
            s.lane_change_speed = 0.6;
            s.update_lane(1.0, Some(&field));
            assert!((s.lane - (other_lane - direction * 2.0 * HORSE_LANE)).abs() < 1e-10);
            assert!(s.moved_lane_this_tick);
            s.update_lane(1.0, Some(&field));
            assert!(!s.moved_lane_this_tick);
        }
    }

    #[test]
    fn overlapping_runners_can_separate_and_finished_runners_do_not_block() {
        for (pos, other_pos, other_lane, target) in [(2000.0, 2000.0, 0.65, 0.2), (2000.0, 2002.0, 0.85, 1.3), (2399.5, 2400.0, 0.85, 1.3)] {
            let field = RaceField::new(vec![solver(pos, 0.6), solver(other_pos, other_lane)]);
            let mut s = field.solvers[0].borrow_mut();
            force_lane(&mut s, target);
            s.lane_change_speed = 0.6;
            s.update_lane(1.0, Some(&field));
            assert!(s.moved_lane_this_tick);
            assert!(if target < 0.6 { s.lane < 0.6 } else { s.lane > other_lane });
        }
    }

    #[test]
    fn outer_boundary_does_not_report_phantom_movement() {
        let field = RaceField::new(vec![solver(2000.0, 1.3)]);
        let mut s = field.solvers[0].borrow_mut();
        force_lane(&mut s, 2.0);
        s.lane_change_speed = 0.6;
        s.update_lane(1.0, Some(&field));
        assert_eq!(s.lane, 1.3);
        assert_eq!(s.lane_target, 1.3);
        assert!(!s.moved_lane_this_tick);
    }

    #[test]
    fn lateral_guard_compares_positions_from_the_same_tick() {
        for reverse in [false, true] {
            let mut s = solver(2000.0, 0.6);
            let mut rival = solver(1999.7, 0.72);
            s.extra_move_lane = Some(0.9);
            rival.extra_move_lane = Some(0.72);
            s.lane_change_speed = 0.6;
            s.start_delay = 0.0;
            rival.start_delay = 0.0;
            s.accel = 0.0;
            rival.accel = 0.0;
            let field = RaceField::new(if reverse { vec![rival, s] } else { vec![s, rival] });
            let (i, j) = if reverse { (1, 0) } else { (0, 1) };
            for _ in 0..15 {
                field.step_all(1.0 / 15.0, &[0, 1]);
                assert!(field.solvers[j].borrow().lane - field.solvers[i].borrow().lane >= 2.0 * HORSE_LANE - 1e-10);
            }
        }
    }

    #[test]
    fn partial_start_delay_preserves_effective_lateral_and_near_timer_dt() {
        let dt = 1.0 / 15.0;
        let mut s = solver(0.0, 0.6);
        let mut rival = solver(2.0, 0.6);
        s.start_delay = 0.04;
        rival.start_delay = 1.0;
        s.current_speed = 0.0;
        rival.current_speed = 0.0;
        force_lane(&mut s, 0.9);
        let field = RaceField::new(vec![s, rival]);
        field.step_all(dt, &[0, 1]);
        let s = field.solvers[0].borrow();
        let rival = field.solvers[1].borrow();
        let moving_dt = dt - 0.04;
        assert!((s.lane - 0.6 - 0.03 * moving_dt * moving_dt).abs() < 1e-10);
        assert!((s.infront_near_lane_timer.get() - moving_dt).abs() < 1e-10);
        assert!((s.accumulatetime.get() - dt).abs() < 1e-10);
        assert_eq!(rival.lane, 0.6);
        assert_eq!(rival.behind_near_lane_timer.get(), 0.0);
        assert!((rival.accumulatetime.get() - dt).abs() < 1e-10);
    }

    #[test]
    fn surrounded_uses_single_precision_cached_gaps_at_inclusive_limits() {
        for (own, limit, peer) in [(0.000003_f32, 0.08333_f32, 1_usize),
                                   (0.000003_f32, 0.08333_f32, 2),
                                   (0.000005_f32, 0.16666_f32, 3)] {
            let other = (own + limit) as f64;
            let own = own as f64;
            assert!(other - own > limit as f64);
            let field = RaceField::new(vec![solver(100.0, own), solver(102.0, own),
                                           solver(98.0, own), solver(100.0, own)]);
            field.solvers[peer].borrow_mut().lane = other;
            assert!(field.is_surrounded(0, 100.0, own));
            field.solvers[peer].borrow_mut().lane = (other as f32 + 0.000001) as f64;
            assert!(!field.is_surrounded(0, 100.0, own));
        }
    }

    #[test]
    fn special_following_timer_is_independent_and_resets_outside_its_corridor() {
        let field = RaceField::new(vec![solver(100.0, 0.0), solver(95.0, 0.15_f32 as f64)]);
        field.update();
        {
            let mut s = field.solvers[0].borrow_mut();
            s.update_near_lane_timers(1.5, Some(&field));
            assert_eq!(s.behind_near_lane_set1_timer.get(), 1.5);
            assert_eq!(s.behind_near_lane_timer.get(), 0.0);
        }
        field.solvers[1].borrow_mut().lane = 0.15001;
        {
            let mut s = field.solvers[0].borrow_mut();
            s.update_near_lane_timers(0.5, Some(&field));
            assert_eq!(s.behind_near_lane_set1_timer.get(), 0.0);
            s.behind_near_lane_set1_timer.set(2.0);
            s.update_near_lane_timers(0.5, None);
            assert_eq!(s.behind_near_lane_set1_timer.get(), 0.0);
        }
    }

    #[test]
    fn finished_runner_is_not_a_front_or_side_blocker() {
        let field = RaceField::new(vec![solver(2399.5, 0.6), solver(2400.0, 0.6)]);
        assert!(field.front_blocker(0, 2399.5, 0.6).is_none());
        field.solvers[1].borrow_mut().lane = 0.61;
        assert_eq!(field.side_room(0, 2399.5, 0.6), (false, false));
    }
}
