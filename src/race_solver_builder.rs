//! Port of race_solver_builder.py -- base->adjusted stat pipeline,
//! skill-data loading/wiring through the condition parser + sample-policy
//! layer.
//!
//! Scoped to exactly what `race_runner.py`'s actual usage exercises (the
//! only caller this port needs to reproduce): `nsamples` is always 1, and
//! `race_runner._new_builder` only ever calls `.seed/.course/.mood/.ground/
//! .weather/.season/.gate_index/.horse/.with_asiwotameru/
//! .with_stamina_syoubu/.num_umas/.wisdom_checks/.add_skill/.build`. Pacer support
//! (`.pacer`/`.use_default_pacer`), `.fork`,
//! `.with_activate_counts_as_random`, per-skill sample-policy overrides,
//! and the generator `redo`/`.send()` protocol are all unreachable from
//! that call path and are not implemented here; `build()` returns a plain
//! `Vec<RaceSolver>` of length `nsamples` instead of a generator.
//! Skill hooks are installed before construction so gate activations are logged.

use std::collections::BTreeMap;

use crate::activation_conditions::{self, CondResult, Condition};
use crate::condition_parser;
use crate::course_data::{self, phase_start, CourseData};
use crate::horse_types::{Aptitude, HorseParameters, Strategy};
use crate::hp_policy::{GameHpState, HpPolicy};
use crate::race_parameters::{parse_ground_condition, parse_season, parse_weather, parse_time, parse_grade, GroundCondition, RaceParameters, RaceParametersWithSkillId};
use crate::race_solver::{Perspective, RaceSolver, SkillEffect, SkillHook, SkillRarity, SkillType};
use crate::random_gen::Rule30CARng;
use crate::region::{rmap_one, Region};

// ---------------------------------------------------------------------------
// Skill data (skill_data.json), loaded once by main.rs via `init_skills`.

pub struct SkillEffectDef {
    pub effect_type_raw: i32,
    pub target: i32,
    pub target_value: i32,
    pub additional_activate: i32,
    pub modifier: f64,
    pub value_usage: i32,
    pub level_modifiers: Vec<f64>,
}

pub struct SkillAlternative {
    pub precondition: Option<String>,
    pub condition: String,
    pub effects: Vec<SkillEffectDef>,
    pub base_duration: f64,
    pub duration_usage: i32,
    pub cooldown: f64,
}

pub struct SkillDef {
    pub is_general_skill: bool,
    pub race_inactive: bool,
    pub rarity: i32,
    pub priority: i32,
    pub wisdom_check: bool,
    pub has_green_tag: bool,
    pub alternatives: Vec<SkillAlternative>,
}

pub type SkillTable = BTreeMap<String, SkillDef>;

static SKILLS: std::sync::OnceLock<std::sync::RwLock<SkillTable>> = std::sync::OnceLock::new();

thread_local! {
    static SCOPED_SKILLS: std::cell::RefCell<Option<std::rc::Rc<SkillTable>>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn with_skills<T>(table: std::rc::Rc<SkillTable>, run: impl FnOnce() -> T) -> T {
    struct Restore(Option<std::rc::Rc<SkillTable>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            SCOPED_SKILLS.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(SCOPED_SKILLS.with(|slot| slot.replace(Some(table))));
    run()
}

enum SkillTableView {
    Scoped(std::rc::Rc<SkillTable>),
    Global(std::sync::RwLockReadGuard<'static, SkillTable>),
}

#[cfg(test)]
mod scoped_table_tests {
    use super::*;
    #[test]
    fn nested_prepared_tables_restore_their_own_definitions() {
        let parse = |is_general: bool| crate::load_skill_value(&crate::json::parse(&format!(
            "{{\"123\":{{\"rarity\":1,\"wisdomCheck\":0,\"isGeneralSkill\":{is_general},\"alternatives\":[]}}}}"
        )).unwrap()).unwrap();
        let before = random_activation_is_general("123");
        with_skills(std::rc::Rc::new(parse(false)), || {
            assert!(!random_activation_is_general("123"));
            with_skills(std::rc::Rc::new(parse(true)), || assert!(random_activation_is_general("123")));
            assert!(!random_activation_is_general("123"));
        });
        assert_eq!(random_activation_is_general("123"), before);
    }
}

impl std::ops::Deref for SkillTableView {
    type Target = SkillTable;
    fn deref(&self) -> &SkillTable {
        match self { Self::Scoped(table) => table, Self::Global(table) => table }
    }
}

pub fn init_skills(table: SkillTable) {
    *SKILLS.get_or_init(|| std::sync::RwLock::new(BTreeMap::new())).write().unwrap() = table;
}

fn skills_table() -> SkillTableView {
    if let Some(table) = SCOPED_SKILLS.with(|slot| slot.borrow().clone()) {
        SkillTableView::Scoped(table)
    } else {
        SkillTableView::Global(SKILLS.get().expect("init_skills must run before build_skill_data").read().unwrap())
    }
}

pub(crate) fn random_activation_is_general(skill_id: &str) -> bool {
    // Old/custom definitions omit this flag. Preserve their legacy behavior;
    // installed projections explicitly carry the authoritative master flag.
    if let Some(table) = SCOPED_SKILLS.with(|slot| slot.borrow().clone()) {
        return table.get(skill_id).map_or(true, |definition| definition.is_general_skill);
    }
    SKILLS.get().and_then(|table| table.read().unwrap().get(skill_id)
        .map(|definition| definition.is_general_skill)).unwrap_or(true)
}

// ---------------------------------------------------------------------------
// Stat pipeline

const GROUND_SPEED_MODIFIER: [[f64; 5]; 3] = [[0.0; 5], [0.0, 0.0, 0.0, 0.0, -50.0], [0.0, 0.0, 0.0, 0.0, -50.0]];
const GROUND_POWER_MODIFIER: [[f64; 5]; 3] = [[0.0; 5], [0.0, 0.0, -50.0, -50.0, -50.0], [0.0, -100.0, -50.0, -100.0, -100.0]];
const STRATEGY_PROFICIENCY_MODIFIER: [f64; 8] = [1.1, 1.0, 0.85, 0.75, 0.6, 0.4, 0.2, 0.1];

const ASITAME_STRATEGY_DISTANCE_COEFFICIENT: [[f64; 6]; 5] = [
    [0.0; 6],
    [0.0, 1.0, 0.7, 0.75, 0.7, 1.0],
    [0.0, 1.0, 0.8, 0.7, 0.75, 1.0],
    [0.0, 1.0, 0.9, 0.875, 0.86, 1.0],
    [0.0, 1.0, 0.9, 1.0, 0.9, 1.0],
];
const ASITAME_BASE_MODIFIER: f64 = 0.00875;

/// v8: sqrt((power-1200)*130)*0.001*coef (client ReleaseConservePower parameters).
pub(crate) fn asitame_release_modifier(power: f64, strategy: Strategy, distance_type: i32) -> f64 {
    // v69: float operands and a float sqrt helper; the product with the float
    // coefficients is one double expression rounded once (doc 13.186).
    let coef = ASITAME_STRATEGY_DISTANCE_COEFFICIENT[distance_type as usize][strategy as usize];
    let arg = ((power as f32 - 1200.0_f32) * 130.0_f32).max(0.);
    let root = (arg as f64).sqrt() as f32;
    (root as f64 * 0.001_f32 as f64 * coef as f32 as f64) as f32 as f64
}
/// v8: duration = K * distance-type activity coef * accumulated conserve time (K fitted: 0.0185).
pub(crate) fn asitame_release_duration(acc: f64, distance_type: i32) -> f64 {
    const K: f64 = 0.0185;
    let dt = match distance_type { 1 => 0.45, 2 => 1.0, 3 => 0.875, _ => 0.8 };
    ((K * dt * acc) as f32) as f64
}
pub(crate) fn asitame_calc_approximate_modifier(power: f64, strategy: Strategy, distance_type: i32) -> f64 {
    ASITAME_BASE_MODIFIER * (power - 1200.0).sqrt() * ASITAME_STRATEGY_DISTANCE_COEFFICIENT[distance_type as usize][strategy as usize]
}

fn stamina_syoubu_distance_factor(distance: f64) -> f64 {
    if distance < 2101.0 {
        0.0
    } else if distance < 2201.0 {
        0.5
    } else if distance < 2401.0 {
        1.0
    } else if distance < 2601.0 {
        1.2
    } else {
        1.5
    }
}

pub(crate) fn stamina_syoubu_calc_approximate_modifier(stamina: f64, distance: f64) -> f64 {
    let random_factor = 1.0; // TODO (matches Python): unclear how the real random-factor scaling works
    (stamina - 1200.0).sqrt() * 0.0085 * stamina_syoubu_distance_factor(distance) * random_factor
}

#[cfg(test)]
mod overcap_tests {
    use super::*;

    #[test]
    fn odd_raw_stats_retain_half_points_before_mood_and_caps() {
        for (raw, expected) in [(1199.,1199.), (1200.,1200.), (1201.,1200.5),
                                (1259.,1229.5), (1283.,1241.5), (1284.,1242.)] {
            assert_eq!(adjust_overcap(raw), expected);
            assert_eq!(base_stat(raw, 1.04), expected * 1.04);
        }
        assert_eq!(base_stat(9999., 1.04), 2000.);
    }
}

fn adjust_overcap(stat: f64) -> f64 {
    if stat > 1200.0 {
        // Official opening positions retain the half-point for odd raw stats.
        1200.0 + (stat - 1200.0) / 2.0
    } else {
        stat
    }
}

/// Raw per-horse input, matching the dict shape `race_runner._new_builder`
/// passes to `.horse({...})`.
#[derive(Clone)]
pub struct HorseSpec {
    pub speed: f64,
    pub stamina: f64,
    pub power: f64,
    pub guts: f64,
    pub wisdom: f64,
    pub strategy: Strategy,
    pub distance_aptitude: Aptitude,
    pub surface_aptitude: Aptitude,
    pub strategy_aptitude: Aptitude,
}

/// RawStat -> BaseStat: halve anything past 1200 (adjust_overcap), apply
/// mood, then hard-clamp to [1, 2000] -- the wiki's documented final bound
/// on Base Stats (matches Python's `_base_stat`).
pub fn base_stat(raw: f64, motiv_coef: f64) -> f64 {
    (adjust_overcap(raw) * motiv_coef).max(1.0).min(2000.0)
}

pub fn build_base_stats(desc: &HorseSpec, mood: i32) -> HorseParameters {
    let motiv_coef = 1.0 + 0.02 * mood as f64;
    HorseParameters {
        speed: crate::retail_initialization::initial_base_wisdom(adjust_overcap(desc.speed), mood) as f64,
        stamina: crate::retail_initialization::initial_base_wisdom(adjust_overcap(desc.stamina), mood) as f64,
        power: crate::retail_initialization::initial_base_wisdom(adjust_overcap(desc.power), mood) as f64,
        guts: crate::retail_initialization::initial_base_wisdom(adjust_overcap(desc.guts), mood) as f64,
        wisdom: crate::retail_initialization::initial_base_wisdom(adjust_overcap(desc.wisdom), mood) as f64,
        strategy: desc.strategy,
        distance_aptitude: desc.distance_aptitude,
        surface_aptitude: desc.surface_aptitude,
        strategy_aptitude: desc.strategy_aptitude,
        raw_stamina: desc.stamina * motiv_coef,
        // Isolated CP hypothesis: mood bonus uses the overcap-adjusted base.
        // Raw stamina, effective stats, skills and release duration are separate.
        // v69: Conserve Power uses the float32 mood-adjusted base, like every
        // other mood-adjusted stat (doc 13.186).
        raw_power: crate::retail_initialization::initial_base_wisdom(adjust_overcap(desc.power), mood) as f64
            + (desc.power - 1200.0).max(0.0) * 0.5,
    }
}

pub fn build_adjusted_stats(base: &HorseParameters, course: &CourseData, ground: GroundCondition) -> HorseParameters {
    build_adjusted_stats_for_mode(base, course, ground, false)
}

pub fn build_adjusted_stats_for_mode(base: &HorseParameters, course: &CourseData, ground: GroundCondition, single_mode: bool) -> HorseParameters {
    let race_course_modifier = course_data::course_speed_modifier(course, base.speed, base.stamina, base.power, base.guts, base.wisdom);
    let ground = ground as usize;
    // Extracted Global.SingleModeAdd* values; added after base-stat adjustments.
    let bonus = if single_mode { 400.0 } else { 0.0 };
    HorseParameters {
        // Round after combining the course bonus and ground penalty. An
        // intermediate float store can change the adjusted stat when the
        // penalty crosses a float exponent boundary, affecting spurt speed.
        speed: (((base.speed as f32 as f64 * race_course_modifier as f32 as f64
            + GROUND_SPEED_MODIFIER[course.surface as usize][ground]) as f32)
            + bonus as f32).clamp(1.,2000.) as f64,
        stamina: (base.stamina + bonus).clamp(1.0,2000.0),
        power: (base.power + GROUND_POWER_MODIFIER[course.surface as usize][ground] + bonus).clamp(1.0,2000.0),
        guts: (base.guts + bonus).clamp(1.0,2000.0),
        wisdom: ((base.wisdom as f32 * ([11000.,10000.,8500.,7500.,6000.,4000.,2000.,1000.][base.strategy_aptitude as usize] * 0.0001_f32)) + bonus as f32).clamp(1.0,2000.0) as f64,
        strategy: base.strategy,
        distance_aptitude: base.distance_aptitude,
        surface_aptitude: base.surface_aptitude,
        strategy_aptitude: base.strategy_aptitude,
        raw_stamina: base.raw_stamina,
        raw_power: base.raw_power,
    }
}

// ---------------------------------------------------------------------------
// Skill target / effect building

const SKILL_TARGET_SELF: i32 = 1;
const SKILL_TARGET_ALL: i32 = 2;

fn is_target(self_persp: Perspective, target_type: i32) -> bool {
    target_type == SKILL_TARGET_ALL || self_persp == Perspective::Any || ((self_persp == Perspective::Self_) == (target_type == SKILL_TARGET_SELF))
}

fn valid_skill_type(t: i32) -> Option<SkillType> {
    match t {
        0 => Some(SkillType::Noop),
        1 => Some(SkillType::SpeedUp),
        2 => Some(SkillType::StaminaUp),
        3 => Some(SkillType::PowerUp),
        4 => Some(SkillType::GutsUp),
        5 => Some(SkillType::WisdomUp),
        6 => Some(SkillType::Runaway),
        8 => Some(SkillType::Vision),
        35 => Some(SkillType::SetTargetLane),
        9 => Some(SkillType::Recovery),
        10 => Some(SkillType::MultiplyStartDelay),
        11 => Some(SkillType::PreferOutsideLane),
        12 => Some(SkillType::PreferInsideLane),
        13 => Some(SkillType::ExtendKakari),
        14 => Some(SkillType::SetStartDelay),
        21 => Some(SkillType::CurrentSpeed),
        22 => Some(SkillType::CurrentSpeedWithNaturalDeceleration),
        27 => Some(SkillType::TargetSpeed),
        29 => Some(SkillType::ModifyKakariChance),
        31 => Some(SkillType::Accel),
        32 => Some(SkillType::AllStatusUp),
        28 => Some(SkillType::LaneMoveSpeedUp),
        36 => Some(SkillType::ActivateRandomSkill),
        37 => Some(SkillType::ActivateRandomGold),
        38 => Some(SkillType::DebuffImmunity),
        42 => Some(SkillType::ExtendEvolvedDuration),
        _ => None,
    }
}

// Doc's skill-level multiplier tables (levels 1-10), by effect category.
// Real Global mechanic, not present in the vendored TS engine (which never
// passes skill level at all -- everything fires at its flat stored value).
// Index 0 unused (levels are 1-indexed).
const LEVEL_MULTIPLIER_TARGET_SPEED: [f64; 11] = [1.00, 1.00, 1.01, 1.04, 1.07, 1.10, 1.13, 1.16, 1.19, 1.22, 1.25];
const LEVEL_MULTIPLIER_ACCEL: [f64; 11] = [1.00, 1.00, 1.02, 1.04, 1.06, 1.08, 1.10, 1.125, 1.15, 1.175, 1.20];
const LEVEL_MULTIPLIER_STAT: [f64; 11] = [1.00, 1.00, 1.01, 1.02, 1.03, 1.04, 1.05, 1.06, 1.07, 1.08, 1.10];
const LEVEL_MULTIPLIER_OTHER: [f64; 11] = [1.00, 1.00, 1.02, 1.04, 1.06, 1.08, 1.10, 1.12, 1.14, 1.16, 1.18];

fn level_multiplier(skill_type: SkillType, level: i32) -> f64 {
    let level = (level.max(1).min(10)) as usize;
    match skill_type {
        SkillType::TargetSpeed => LEVEL_MULTIPLIER_TARGET_SPEED[level],
        SkillType::Accel => LEVEL_MULTIPLIER_ACCEL[level],
        SkillType::SpeedUp | SkillType::StaminaUp | SkillType::PowerUp | SkillType::GutsUp | SkillType::WisdomUp
        | SkillType::AllStatusUp => LEVEL_MULTIPLIER_STAT[level],
        SkillType::Noop => 1.0,
        _ => LEVEL_MULTIPLIER_OTHER[level],
    }
}

fn retail_skill_value(raw: f64, multiplier: f64) -> f64 {
    // Master coefficient /10000 is the existing projected schema.
    let units = multiplier * 10000.0;
    let integral = units.round();
    let coefficient = if (units - integral).abs() < 1e-7 {
        integral as f32 * 0.0001_f32
    } else { multiplier as f32 };
    ((raw as f32 * 0.0001_f32) * coefficient) as f64
}

fn build_skill_effects(alt: &SkillAlternative, perspective: Perspective, level: i32, retail: bool) -> Vec<SkillEffect> {
    alt.effects
        .iter()
        .map(|ef| {
            let effective_type = match valid_skill_type(ef.effect_type_raw) {
                Some(t) if perspective == Perspective::Self_ || is_target(perspective, ef.target) => t,
                _ => SkillType::Noop,
            };
            let multiplier = ef.level_modifiers.get(level.max(1).min(10) as usize - 1)
                .copied().unwrap_or_else(|| level_multiplier(effective_type, level));
            let modifier = if retail { retail_skill_value(ef.modifier, multiplier) }
                else { ef.modifier / 10000.0 * multiplier };
            SkillEffect { effect_type: effective_type, base_duration: if retail {
                ((alt.base_duration as f32) * 0.0001_f32) as f64
            } else { alt.base_duration / 10000.0 }, duration_usage: alt.duration_usage,
                          modifier, value_usage: ef.value_usage, additional_activate: ef.additional_activate, target: ef.target, target_value: ef.target_value,
                          target_is_debuff: crate::race_field::is_skill_demerit(ef.effect_type_raw, ef.modifier / 10000.0) }
        })
        .collect()
}

fn skill_rarity_from_i32(v: i32) -> CondResult<SkillRarity> {
    match v {
        1 => Ok(SkillRarity::White),
        2 => Ok(SkillRarity::Gold),
        3 => Ok(SkillRarity::Unique),
        6 => Ok(SkillRarity::Evolution),
        other => Err(format!("unexpected skill rarity {other}")),
    }
}

pub struct SkillData {
    pub anyskill_nodes: Vec<usize>,
    pub detail_index: usize,
    pub skill_id: String,
    pub perspective: Perspective,
    pub rarity: SkillRarity,
    pub wisdom_check: bool,
    pub sample_policy: crate::activation_sample_policy::SamplePolicy,
    pub regions: Vec<Region>,
    pub extra_condition: activation_conditions::OptCond,
    pub prerequisite: activation_conditions::OptCond,
    pub cooldown: f64,
    pub cooldown_key: String,
    pub effects: Vec<SkillEffect>,
}

type SampledTriggerIndex<'a> = BTreeMap<(&'a str, usize, &'a str), Vec<&'a crate::retail_initialization::Trigger>>;

fn index_sampled_triggers(row: &crate::retail_initialization::HorseInitialization) -> SampledTriggerIndex<'_> {
    let mut index = SampledTriggerIndex::new();
    for trigger in &row.triggers {
        index.entry((trigger.skill_id.as_str(), trigger.alternative, trigger.field.as_str()))
            .or_default().push(trigger);
    }
    index
}

fn sampled_triggers_for(index: &SampledTriggerIndex<'_>, skill_id: &str, alternative: usize,
                        column: &str) -> Vec<crate::retail_initialization::Trigger> {
    index.get(&(skill_id, alternative, column)).into_iter().flatten()
        .map(|trigger| (**trigger).clone()).collect()
}

#[allow(clippy::too_many_arguments)]
fn build_skill_data(
    horse: &HorseParameters,
    race_params: &RaceParameters,
    course: &'static CourseData,
    whole_course: &[Region],
    conditions: &'static BTreeMap<String, Condition>,
    skill_id: &str,
    perspective: Perspective,
    level: i32,
    ignore_null_effects: bool,
    retail: Option<&crate::retail_initialization::HorseInitialization>,
    sampled: Option<&SampledTriggerIndex<'_>>,
) -> CondResult<Vec<SkillData>> {
    debug_assert_eq!(retail.is_some(), sampled.is_some());
    let skills = skills_table();
    let def = skills.get(skill_id).ok_or_else(|| format!("bad skill ID {skill_id}"))?;
    if def.alternatives.is_empty() {
        if def.race_inactive { return Ok(Vec::new()); }
        return Err(format!("skill {skill_id} has no installed activation alternatives"));
    }
    if retail.is_some() {
        // An ineligible branch must not hide an unsupported reference mechanic.
        for (index, alternative) in def.alternatives.iter().enumerate() {
            for (column, expression) in [("condition", alternative.condition.as_str()),
                ("precondition", alternative.precondition.as_deref().unwrap_or(""))] {
                if expression.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .any(|token| token == "is_exist_skill_id") {
                    return Err(format!("skill {skill_id} detail {index}: unsupported {column} token is_exist_skill_id"));
                }
            }
            for effect in &alternative.effects {
                if valid_skill_type(effect.effect_type_raw).is_none() {
                    return Err(format!("skill {skill_id} detail {index}: unsupported effect type {}", effect.effect_type_raw));
                }
            }
        }
    }
    let extra = RaceParametersWithSkillId { params: race_params, skill_id: skill_id.to_string() };

    let mut triggers: Vec<SkillData> = Vec::new();
    for (alt_index,alt) in def.alternatives.iter().enumerate() {
        let parse = |expression:&str,column:&str| condition_parser::parse_with_retail_runtime(
            conditions,expression,sampled.map(|index| sampled_triggers_for(index, skill_id, alt_index, column)),
            retail.is_some_and(|row|row.runtime_phase));
        let mut full: Vec<Region> = whole_course.to_vec();
        let mut prerequisite: activation_conditions::OptCond = None;
        if let Some(precondition) = alt.precondition.as_deref() {
            if !precondition.is_empty() {
                let pre = parse(precondition,"precondition")?;
                let (pre_regions, pre_check) = pre.apply(whole_course, course, horse, &extra)?;
                if pre_regions.is_empty() {
                    continue;
                }
                let bounds = Region::new(pre_regions[0].start, whole_course.last().unwrap().end);
                full = rmap_one(&full, |r| r.intersect(&bounds));
                prerequisite = Some(Box::new(move |s, field| {
                    pre_regions.iter().any(|r| r.start <= s.pos && s.pos < r.end)
                        && pre_check.as_ref().is_none_or(|check| check(s, field))
                }));
            }
        }

        let op = parse(&alt.condition,"condition")?;
        crate::activation_conditions::take_anyskill_nodes();
        let (mut regions, mut extra_condition) = op.apply(&full, course, horse, &extra)?;
        let anyskill_nodes = crate::activation_conditions::take_anyskill_nodes();
        if regions.is_empty() {
            continue;
        }
        if retail.is_none() && matches!(op.sample_policy(), crate::activation_sample_policy::SamplePolicy::Immediate) && regions.len() > 1 {
            // Keep later immediate windows alive, while excluding their gaps.
            let start = regions.iter().map(|r| r.start).fold(f64::INFINITY, f64::min);
            let end = regions.iter().map(|r| r.end).fold(f64::NEG_INFINITY, f64::max);
            let windows = regions;
            let check = extra_condition;
            extra_condition = Some(Box::new(move |s, field| {
                windows.iter().any(|r| r.start <= s.pos && s.pos < r.end)
                    && check.as_ref().is_none_or(|f| f(s, field))
            }));
            regions = vec![Region::new(start, end)];
        }
        let effects = build_skill_effects(alt, perspective, level, retail.is_some());
        if !effects.is_empty() || ignore_null_effects {
            let rarity_raw = def.rarity;
            let rarity = if (3..=5).contains(&rarity_raw) { 3 } else { rarity_raw };
            triggers.push(SkillData {
                anyskill_nodes,
                detail_index: alt_index,
                skill_id: skill_id.to_string(),
                perspective,
                rarity: skill_rarity_from_i32(rarity)?,
                wisdom_check: def.wisdom_check,
                sample_policy: op.sample_policy().clone(),
                regions,
                extra_condition,
                prerequisite,
                cooldown: alt.cooldown / 10000.0,
                cooldown_key: format!("{}{}", skill_id, if alt.condition.contains("is_activate_other_skill_detail") { "#followup" } else { "" }),
                effects,
            });
        }
    }
    if triggers.iter().any(|t| t.detail_index == 0) {
        triggers.reverse(); // backward runtime scan gives the first alternative priority
        return Ok(triggers);
    }
    // SkillDetail.IsReservedRandomActivate permits only detail zero. Preserve
    // its effects even when another detail has a satisfiable normal trigger.
    let effects = build_skill_effects(&def.alternatives[0], perspective, level, retail.is_some());
    if effects.is_empty() && !ignore_null_effects {
        triggers.reverse();
        return Ok(triggers);
    }
    let rarity_raw = def.rarity;
    let rarity = if (3..=5).contains(&rarity_raw) { 3 } else { rarity_raw };
    let after_end = vec![Region::new(9999.0, 9999.0)];
    triggers.insert(0, SkillData {
        anyskill_nodes: Vec::new(),
        detail_index: 0,
        skill_id: skill_id.to_string(),
        perspective,
        rarity: skill_rarity_from_i32(rarity)?,
        wisdom_check: def.wisdom_check,
        sample_policy: crate::activation_sample_policy::SamplePolicy::Immediate,
        regions: after_end,
        extra_condition: Some(Box::new(|_s, _f| false)),
        prerequisite: None,
        cooldown: 0.0, cooldown_key: skill_id.to_string(),
        effects,
    });
    triggers.reverse();
    Ok(triggers)
}

// ---------------------------------------------------------------------------
// Builder

type ExtraSkillHook = Box<dyn Fn(&mut Vec<SkillData>, &HorseParameters, &CourseData)>;

fn wisdom_seed(horse_seed: u32, skill_id: u32) -> u32 {
    let mut seed = horse_seed ^ skill_id ^ 0x57495344;
    seed = (seed ^ (seed >> 16)).wrapping_mul(0x85ebca6b);
    seed = (seed ^ (seed >> 13)).wrapping_mul(0xc2b2ae35);
    seed ^ (seed >> 16)
}

pub fn race_seed_hi(horse_seed: u32) -> u32 {
    // Local seed expansion, matching race_runner._race_seed_hi. Preserve the
    // low word for wisdom checks and consume the same number of random draws.
    let hi = wisdom_seed(horse_seed, 0x52414345);
    if hi == 0 { 1 } else { hi }
}

pub struct RaceSolverBuilder {
    nsamples: usize,
    course: Option<&'static CourseData>,
    race_params: RaceParameters,
    horse_desc: Option<HorseSpec>,
    single_mode: bool,
    gate_index: i32,
    rng: Rule30CARng,
    wisdom_seed: u32,
    wisdom_checks: bool,
    conditions: &'static BTreeMap<String, Condition>,
    skills: Vec<(String, Perspective, i32)>,
    extra_skill_hooks: Vec<ExtraSkillHook>,
    pub race_context: crate::race_solver::RaceContext,
    pub retail_initialization: Option<crate::retail_initialization::HorseInitialization>,
    pub on_skill_activate: Option<SkillHook>,
    pub on_skill_deactivate: Option<SkillHook>,
}

impl RaceSolverBuilder {
    pub fn new(nsamples: usize, conditions: &'static BTreeMap<String, Condition>, seed_lo: u32, seed_hi: u32) -> Self {
        RaceSolverBuilder {
            nsamples,
            course: None,
            race_params: RaceParameters::default(),
            horse_desc: None,
            single_mode: false,
            gate_index: 0,
            rng: Rule30CARng::new(seed_lo, seed_hi),
            wisdom_seed: seed_lo,
            wisdom_checks: false,
            conditions,
            skills: Vec::new(),
            extra_skill_hooks: Vec::new(),
            race_context: Default::default(),
            retail_initialization: None,
            on_skill_activate: None,
            on_skill_deactivate: None,
        }
    }

    pub fn seed(&mut self, lo: u32, hi: u32) -> &mut Self {
        self.rng = Rule30CARng::new(lo, hi);
        self.wisdom_seed = lo;
        self
    }

    pub fn wisdom_checks(&mut self, enabled: bool) -> &mut Self {
        self.wisdom_checks = enabled;
        self
    }

    pub fn num_umas(&mut self, n: i32) -> &mut Self {
        self.race_params.num_umas = Some(n);
        self
    }

    pub fn roster_conditions(&mut self, same_style_count: i32, favorite_matches: Option<bool>,
                             skill_counts: std::rc::Rc<BTreeMap<String, i32>>, popularity: i32,
                             chara_ids: std::rc::Rc<std::collections::BTreeSet<i64>>, other_style_counts: [i32; 4]) -> &mut Self {
        self.race_params.same_style_count = Some(same_style_count);
        self.race_params.other_style_counts = Some(other_style_counts);
        self.race_params.favorite_matches = favorite_matches;
        self.race_params.skill_counts = Some(skill_counts);
        self.race_params.chara_ids = Some(chara_ids);
        self.race_params.popularity = popularity;
        self
    }

    pub fn course(&mut self, course_id: i32) -> &mut Self {
        self.course = Some(course_data::get_course(course_id));
        self
    }

    pub fn mood(&mut self, mood: i32) -> &mut Self {
        self.race_params.mood = mood;
        self
    }

    pub fn ground(&mut self, ground: &str) -> CondResult<&mut Self> {
        self.race_params.ground_condition = parse_ground_condition(ground)?;
        Ok(self)
    }

    pub fn weather(&mut self, weather: &str) -> CondResult<&mut Self> {
        self.race_params.weather = parse_weather(weather)?;
        Ok(self)
    }

    pub fn season(&mut self, season: &str) -> CondResult<&mut Self> {
        self.race_params.season = parse_season(season)?;
        Ok(self)
    }

    pub fn time(&mut self, time: &str) -> CondResult<&mut Self> {
        self.race_params.time = parse_time(time)?;
        Ok(self)
    }

    pub fn grade(&mut self, grade: &str) -> CondResult<&mut Self> {
        self.race_params.grade = parse_grade(grade)?;
        Ok(self)
    }

    pub fn dirt_grade(&mut self, value: Option<bool>) -> &mut Self {
        self.race_params.is_dirtgrade = value;
        self
    }

    pub fn horse(&mut self, desc: HorseSpec) -> &mut Self {
        self.horse_desc = Some(desc);
        self
    }

    pub fn single_mode(&mut self, enabled: bool) -> &mut Self {
        self.single_mode = enabled;
        self
    }

    pub fn gate_index(&mut self, index: i32) -> &mut Self {
        self.gate_index = index;
        self
    }

    pub fn add_skill(&mut self, skill_id: &str, level: i32) -> &mut Self {
        self.skills.push((skill_id.to_string(), Perspective::Self_, level));
        self
    }

    /// Must be called after `.horse()` and `.mood()`.
    pub fn with_asiwotameru(&mut self) -> &mut Self {
        let hook: ExtraSkillHook = Box::new(move |skilldata, _horse, course| {
                let spurt_start = vec![Region::new(phase_start(course.distance, 2), course.distance)];
                skilldata.push(SkillData {
                    anyskill_nodes: Vec::new(),
                    detail_index: 0,
                    skill_id: "asitame".to_string(),
                    perspective: Perspective::Self_,
                    rarity: SkillRarity::White,
                    wisdom_check: false,
                    sample_policy: crate::activation_sample_policy::SamplePolicy::Immediate,
                    regions: spurt_start,
                    prerequisite: None,
        cooldown: 0.0, cooldown_key: "asitame".to_string(),
                    extra_condition: Some(Box::new(|s, _f| s.is_last_spurt && s.horse.raw_power > 1200.0 && s.conserve_release_reaches_threshold())),
                    effects: vec![SkillEffect {
                        effect_type: SkillType::Accel,
                        base_duration: 3.0 / (course.distance / 1000.0),
                        modifier: 0.0, // Read active stats when the effect activates.
                        value_usage: 1,
                        duration_usage: 1,
                        additional_activate: 0, target: 1, target_value: 0, target_is_debuff: false,
                    }],
                });
        });
        self.extra_skill_hooks.push(hook);
        self
    }

    pub fn with_stamina_syoubu(&mut self) -> &mut Self {
        let hook: ExtraSkillHook = Box::new(move |skilldata, _horse, course| {
            if course.distance > 2100.0 {
                let spurt_start = vec![Region::new(phase_start(course.distance, 2), course.distance)];
                skilldata.push(SkillData {
                    anyskill_nodes: Vec::new(),
                    detail_index: 0,
                    skill_id: "staminasyoubu".to_string(),
                    perspective: Perspective::Self_,
                    rarity: SkillRarity::White,
                    wisdom_check: false,
                    sample_policy: crate::activation_sample_policy::SamplePolicy::Immediate,
                    regions: spurt_start,
                    prerequisite: None,
        cooldown: 0.0, cooldown_key: "staminasyoubu".to_string(),
                    extra_condition: Some(Box::new(|s, _f| s.horse.raw_stamina > 1200.0 && s.current_speed >= s.last_spurt_speed)),
                    effects: vec![SkillEffect {
                        effect_type: SkillType::TargetSpeed,
                        base_duration: 9999.0,
                        modifier: 0.0, // Read active stats when the effect activates.
                        value_usage: 1,
                        duration_usage: 1,
                        additional_activate: 0, target: 1, target_value: 0, target_is_debuff: false,
                    }],
                });
            }
        });
        self.extra_skill_hooks.push(hook);
        self
    }

    /// Everything `build()` does before it starts constructing solvers:
    /// stat pipeline, skill-data loading, sample-policy trigger placement.
    /// Split out so `validate_skills()` can run exactly this much without
    /// paying for a throwaway `RaceSolver`.
    #[allow(clippy::type_complexity)]
    fn prepare(&mut self) -> CondResult<(HorseParameters, Vec<SkillData>, Vec<Vec<Region>>, Rule30CARng)> {
        let horse_desc = self.horse_desc.as_ref().expect("horse() must be called before build()");
        let mut horse = build_base_stats(horse_desc, self.race_params.mood);
        // HorseData.InitRunningStyleEx scans learned effects before triggers
        // and wisdom lotteries; type 6 is SkillAbilityNull at runtime.
        if horse.strategy == Strategy::Nige && self.skills.iter().any(|(sid, perspective, _)| {
            *perspective == Perspective::Self_ && skills_table().get(sid).is_some_and(|skill|
                skill.alternatives.iter().any(|alt| alt.effects.iter().any(|effect| effect.effect_type_raw == 6)))
        }) {
            horse.strategy = Strategy::Oonige;
        }
        let solver_rng = Rule30CARng::new(self.rng.int32(), 0);
        let _pacer_rng = Rule30CARng::new(self.rng.int32(), 0); // drawn for parity with Python's rng advancement even though no pacer is ever configured on this path

        let course = self.course.expect("course() must be called before build()");
        let whole_course = vec![Region::new(0.0, course.distance)];

        let mut trigger_horse = horse.clone();
        if self.retail_initialization.as_ref().is_some_and(|row|row.runtime_phase) {
            use crate::retail_initialization::initial_base_wisdom;
            // v69: skill-condition base stats halve the excess over 1200 before mood,
            // like every other base stat (doc 13.189).
            let base = |raw| initial_base_wisdom(adjust_overcap(raw), self.race_params.mood) as i32 as f64;
            trigger_horse.speed = base(horse_desc.speed);
            trigger_horse.stamina = base(horse_desc.stamina);
            trigger_horse.power = base(horse_desc.power);
            trigger_horse.guts = base(horse_desc.guts);
            trigger_horse.wisdom = base(horse_desc.wisdom);
        }
        // These samples already belong to this seed and runner. Index their
        // identity once, keeping each expression's original node order.
        let sampled = self.retail_initialization.as_ref().map(index_sampled_triggers);
        let mut skilldata: Vec<SkillData> = Vec::new();
        for (skill_id, perspective, level) in &self.skills {
            skilldata.extend(build_skill_data(&trigger_horse, &self.race_params, course, &whole_course, self.conditions, skill_id, *perspective, *level, false, self.retail_initialization.as_ref(), sampled.as_ref())?);
        }
        // Usage 13 uses displayed raw stats, not adjusted stats or green buffs.
        let raw_max = [horse_desc.speed, horse_desc.stamina, horse_desc.power, horse_desc.guts, horse_desc.wisdom]
            .into_iter().fold(f64::NEG_INFINITY, f64::max);
        let coef = if raw_max < 600.0 { 0.800000011920929 } else if raw_max < 800.0 { 0.8999999761581421 }
            else if raw_max < 1000.0 { 1.0 } else if raw_max < 1100.0 { 1.100000023841858 } else { 1.2000000476837158 };
        for sd in &mut skilldata {
            for effect in &mut sd.effects {
                if effect.value_usage == 13 { effect.modifier *= coef; }
                else if matches!(effect.value_usage, 3..=7 | 10 | 12) {
                    let (value, bounds): (f64, [f64; 4]) = match effect.value_usage {
                        3..=7 => (self.race_context.team_stats.unwrap_or([horse.speed, horse.stamina, horse.power, horse.guts, horse.wisdom])[(effect.value_usage-3) as usize], [1200.0,1800.0,2600.0,3600.0]),
                        10 => (self.race_context.wins, [6.0,14.0,18.0,25.0]),
                        _ => (self.race_context.fans, [20000.0,50000.0,100000.0,160000.0]),
                    };
                    let tier = bounds.iter().filter(|&&threshold| value >= threshold).count();
                    let mut factor = [0.800000011920929,0.8999999761581421,1.0,1.100000023841858,1.2000000476837158][tier];
                    if self.retail_initialization.as_ref().is_some_and(|row|row.runtime_phase)
                        && ((matches!(effect.value_usage,3..=7) && value>=99999.)
                            || (effect.value_usage==12 && value>=99999999.)) { factor=1.; }
                    effect.modifier *= factor;
                }
            }
        }
        if self.retail_initialization.as_ref().is_some_and(|row|row.runtime_phase) {
            for sd in &mut skilldata {
                for effect in &mut sd.effects {
                    if matches!(effect.value_usage, 3..=7 | 10 | 12 | 13) {
                        effect.modifier = effect.modifier as f32 as f64;
                    }
                }
            }
        }
        for hook in &self.extra_skill_hooks {
            hook(&mut skilldata, &horse, course);
        }

        let mut triggers: Vec<Vec<Region>> = Vec::with_capacity(skilldata.len());
        let mut sampled: BTreeMap<String, Vec<Region>> = BTreeMap::new();
        for sd in &skilldata {
            // Shared-stream initialization already sampled these exact windows.
            // Only the conventional sampler needs duplicate-key RNG reuse.
            if self.retail_initialization.is_some() {
                triggers.push(sd.regions.clone());
                continue;
            }
            let key = format!("{}:{:?}:{:?}", sd.cooldown_key, sd.sample_policy, sd.regions);
            let windows = sampled.entry(key).or_insert_with(|| {
                if matches!(sd.sample_policy, crate::activation_sample_policy::SamplePolicy::AllCornerRandom) {
                    crate::activation_sample_policy::place_all_corner_triggers(&sd.regions, &mut self.rng)
                } else { sd.sample_policy.sample(&sd.regions, self.nsamples, &mut self.rng) }
            });
            triggers.push(windows.clone());
        }

        // must come after skill activations are decided: conditions like
        // base_power depend on BASE stats
        horse = build_adjusted_stats_for_mode(&horse, course, self.race_params.ground_condition, self.single_mode);

        Ok((horse, skilldata, triggers, solver_rng))
    }

    /// Raises whatever `prepare()` would raise for this builder's skill
    /// set, without constructing a `RaceSolver` -- used to probe whether a
    /// single skill can be built at all (see main.rs's per-skill fallback,
    /// mirroring `race_runner.is_usable`).
    pub fn validate_skills(&mut self) -> CondResult<()> {
        self.prepare()?;
        Ok(())
    }

    /// Returns one solver per sample. `nsamples` is always 1 on the path
    /// this port implements (see module doc) -- enforced here rather than
    /// silently mis-behaving, since `Box<dyn Fn>` extra_conditions aren't
    /// `Clone` and Python's generator otherwise shares the same closure
    /// object by reference across samples, which this port has no
    /// equivalent for. Unlike Python's generator, there is no `redo`/
    /// `.send()` protocol either -- also unreachable from race_runner.py's
    /// actual usage.
    pub fn build(&mut self) -> CondResult<Vec<RaceSolver>> {
        if self.nsamples != 1 {
            return Err("RaceSolverBuilder::build only supports nsamples == 1 on this port".to_string());
        }
        let (horse, skilldata, triggers, solver_rng) = self.prepare()?;
        let course = self.course.unwrap();
        let ground = self.race_params.ground_condition;

        let mut skills = Vec::with_capacity(skilldata.len());
        let base = build_base_stats(self.horse_desc.as_ref().unwrap(), self.race_params.mood);
        let chance = (1.0 - 90.0 / base.wisdom).max(0.2);
        let mut wisdom_rngs: BTreeMap<String, Rule30CARng> = BTreeMap::new();
        let mut wisdom_picks: BTreeMap<String, bool> = BTreeMap::new();
        for (sd, windows) in skilldata.into_iter().zip(triggers) {
            let mut normal_enabled = true;
            if let Some(enabled)=self.retail_initialization.as_ref().and_then(|row|row.skill_enabled.get(&sd.skill_id)) {
                normal_enabled = *enabled;
                // A reservation overrides the failed ordinary wisdom lottery.
                if !enabled && !(sd.detail_index == 0 && matches!(sd.rarity, SkillRarity::White | SkillRarity::Gold | SkillRarity::Evolution)) {continue;}
            } else if self.wisdom_checks && sd.wisdom_check {
                let sid = sd.skill_id.parse::<u32>().unwrap_or(0);
                let seed = wisdom_seed(self.wisdom_seed, sid);
                let hi = wisdom_seed(self.wisdom_seed ^ 0x9e3779b9, sid);
                let rng = wisdom_rngs.entry(sd.skill_id.clone()).or_insert_with(|| Rule30CARng::new(seed, hi));
                if !*wisdom_picks.entry(sd.cooldown_key.clone()).or_insert_with(|| rng.random() < chance) { continue; }
            }
            let trig = Region::new(windows[0].start, windows.last().unwrap().end);
            skills.push(crate::race_solver::PendingSkill {
                anyskill_nodes: sd.anyskill_nodes,
                random_activation_order: None,
                detail_index: sd.detail_index,
                priority: skills_table().get(&sd.skill_id).map_or(0,|s|s.priority),
                has_green_tag: skills_table().get(&sd.skill_id).is_some_and(|s| s.has_green_tag),
                cooldown_key: if sd.cooldown_key.is_empty() { sd.skill_id.clone() } else { sd.cooldown_key },
                numeric_skill_id: sd.skill_id.parse().ok(),
                skill_id: sd.skill_id,
                perspective: sd.perspective,
                rarity: sd.rarity,
                trigger: trig,
                cooldown: sd.cooldown,
                windows,
                // `None` here plays the role Python's `k_true` sentinel
                // played: a condition whose static filter alone determines
                // whether it holds, with no further per-tick check needed.
                extra_condition: if normal_enabled {
                    sd.extra_condition.unwrap_or_else(|| Box::new(|_s, _f| true))
                } else { Box::new(|_s, _f| false) },
                effects: sd.effects,
                prerequisite: sd.prerequisite,
                prerequisite_satisfied: false,
            });
        }

        // Backward runtime scan: reverse wire skill groups, retain the
        // alternatives' existing order. Match the client's two priority passes.
        let mut first_index = BTreeMap::new();
        for (index, skill) in skills.iter_mut().enumerate() {
            first_index.entry(skill.skill_id.clone()).or_insert(index);
            skill.random_activation_order = Some(first_index[&skill.skill_id]);
        }
        let table = skills_table();
        skills.sort_by_key(|skill| (
            table.get(&skill.skill_id).is_some_and(|def| def.priority == 1),
            std::cmp::Reverse(first_index[&skill.skill_id])));
        drop(table);
        let mut solver_rng = solver_rng;
        let hp = HpPolicy::Game(GameHpState::new(course, ground as i32, Rule30CARng::new(solver_rng.int32(), 0)));
        let mut solver = RaceSolver::new(horse, course, solver_rng, skills, hp, None,
            self.on_skill_activate.take(), self.on_skill_deactivate.take(), self.gate_index,
            self.retail_initialization.as_ref().is_some_and(|row| row.runtime_phase));
        solver.race_context = self.race_context.clone();
        Ok(vec![solver])
    }
}

#[cfg(test)]
mod sampled_trigger_index_tests {
    use super::*;
    use crate::retail_initialization::{HorseInitialization, Trigger, Windows};

    #[test]
    fn indexed_samples_preserve_expression_order_and_parser_errors() {
        let mut row = HorseInitialization::default();
        for (i, (skill, alternative, column, name, argument)) in [
            ("a", 0, "condition", "phase_random", 0),
            ("b", 0, "condition", "phase_random", 1),
            ("a", 0, "precondition", "phase_random", 2),
            ("a", 1, "condition", "phase_random", 3),
            ("a", 0, "condition", "corner_random", 1),
            ("a", 0, "unrelated", "phase_random", 4),
        ].into_iter().enumerate() {
            row.triggers.push(Trigger {
                skill_id: skill.into(), alternative, field: column.into(), name: name.into(), argument,
                windows: Windows::Distances(vec![(i as f32, i as f32 + 10.0)]), draw_start: i, draw_end: i + 1,
            });
        }
        let index = index_sampled_triggers(&row);
        let table = Box::leak(Box::new(activation_conditions::build_conditions()));
        for _ in 0..2 {
            for (skill, alternative, column, expression) in [
                ("a", 0, "condition", "phase_random==0&corner_random==1"),
                ("a", 0, "condition", "phase_random==0"),
                ("a", 0, "condition", "corner_random==1&phase_random==0"),
                ("a", 0, "precondition", "phase_random==2"),
                ("a", 1, "condition", "phase_random==3"),
                ("b", 0, "condition", "phase_random==1"),
                ("a", 7, "condition", "phase_random==0"),
                ("missing", 0, "condition", "phase_random==0"),
            ] {
                let filtered: Vec<_> = row.triggers.iter().filter(|trigger|
                    trigger.skill_id == skill && trigger.alternative == alternative && trigger.field == column).cloned().collect();
                let indexed = sampled_triggers_for(&index, skill, alternative, column);
                assert_eq!(format!("{indexed:?}"), format!("{filtered:?}"));
                let expected = condition_parser::parse_with_retail_runtime(table, expression, Some(filtered), true).err();
                let actual = condition_parser::parse_with_retail_runtime(table, expression, Some(indexed), true).err();
                assert_eq!(actual, expected, "{skill}/{alternative}/{column}: {expression}");
            }
        }
    }
}

#[cfg(test)]
mod skill_value_rounding_tests {
    use super::retail_skill_value;
    #[test]
    fn master_and_level_conversion_round_separately() {
        assert_eq!(retail_skill_value(1500.,1.), 0.14999999105930328);
        assert_eq!(retail_skill_value(1500.,1.13), ((1500_f32*0.0001_f32)*(11300_f32*0.0001_f32)) as f64);
        assert_eq!(retail_skill_value(-1500.,1.13), -retail_skill_value(1500.,1.13));
        assert_eq!(retail_skill_value(1500.,1.123456), ((1500_f32*0.0001_f32)*1.123456_f32) as f64);
    }
}
