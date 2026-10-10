//! Port of race_runner.py -- reads one race spec as JSON on stdin, writes
//! `{seed, frames, results, skillEvents, dataSignature}` as JSON on stdout.
//! Persistent newline-delimited worker for race_simulator.py. Skill tables
//! are replaced from each request's installed data; standalone requests
//! without skillData use the bundled reference table.

mod activation_conditions;
mod activation_sample_policy;
mod condition_parser;
mod course_data;
mod conserve_power;
mod horse_types;
mod hp_policy;
mod json;
mod race_field;
mod race_parameters;
mod race_solver;
mod lane_selection;
mod race_solver_builder;
mod random_gen;
mod retail_random;
mod rushing;
mod retail_initialization;
mod retail_physics;
mod retail_states;
mod retail_sort;
mod retail_spurt;
mod retail_geometry;
mod retail_lane;
mod retail_position_keep;
mod overtaking;
mod region;
mod starting_grid;
mod stadium_score;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};
use std::rc::Rc;

use course_data::{Corner, CourseData, Slope, Straight};
use horse_types::{Aptitude, Strategy};
use json::Value;
use race_field::RaceField;
use race_solver_builder::{HorseSpec, RaceSolverBuilder, SkillAlternative, SkillDef, SkillEffectDef, SkillTable};

const DT: f64 = 1.0 / 15.0;
const MAX_RACE_SECONDS: f64 = 400.0;

const COURSE_DATA_JSON: &str = include_str!("../data/course_data.json");
const SKILL_DATA_JSON: &str = include_str!("../data/skill_data.json");

fn opt_array(v: &Value, key: &str) -> Vec<Value> {
    v.get(key).and_then(Value::as_array).cloned().unwrap_or_default()
}

fn load_course_table(text: &str) -> Result<BTreeMap<i32, CourseData>, String> {
    let root = json::parse(text)?;
    let obj = root.as_object().ok_or("course_data.json root must be an object")?;
    let mut out = BTreeMap::new();
    for (course_id_str, raw) in obj {
        let course_id: i32 = course_id_str.parse().map_err(|_| format!("bad course id key {course_id_str}"))?;
        let mut slopes: Vec<Slope> = opt_array(raw, "slopes")
            .iter()
            .map(|s| Ok(Slope { start: s.get_f64("start")?, length: s.get_f64("length")?, slope: s.get_f64("slope")? }))
            .collect::<Result<_, String>>()?;
        if !course_data::is_sorted_by_start_f(slopes.iter().map(|s| s.start)) {
            slopes.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        }
        let corners: Vec<Corner> = opt_array(raw, "corners")
            .iter()
            .map(|c| Ok(Corner { start: c.get_f64("start")?, length: c.get_f64("length")? }))
            .collect::<Result<_, String>>()?;
        let straights: Vec<Straight> = opt_array(raw, "straights")
            .iter()
            .map(|s| Ok(Straight { start: s.get_f64("start")?, end: s.get_f64("end")?, front_type: s.get_i64("frontType")? as i32 }))
            .collect::<Result<_, String>>()?;
        let course_set_status: Vec<i32> = opt_array(raw, "courseSetStatus").iter().filter_map(Value::as_i64).map(|x| x as i32).collect();
        let lane_max = raw.get("laneMax").and_then(Value::as_f64).unwrap_or(13000.0) / 10000.0;
        let first_move_lane_point = raw.get("firstMoveLanePoint").and_then(Value::as_f64)
            .unwrap_or_else(|| corners.first().map_or(0.0, |c|c.start.max(0.0)));
        out.insert(
            course_id,
            CourseData {
                race_track_id: raw.get_i64("raceTrackId")? as i32,
                distance: raw.get_f64("distance")?,
                distance_type: raw.get_i64("distanceType")? as i32,
                surface: raw.get_i64("surface")? as i32,
                turn: raw.get_i64("turn")? as i32,
                course_set_status,
                first_move_lane_point,
                empty_in_move: first_move_lane_point > 0.
                    && raw.get("emptyInMove").and_then(Value::as_bool).unwrap_or(true),
                corners,
                straights,
                slopes,
                max_lane: lane_max,
            },
        );
    }
    Ok(out)
}

fn load_skill_table(text: &str) -> Result<SkillTable, String> {
    let root = json::parse(text)?;
    let obj = root.as_object().ok_or("skill_data.json root must be an object")?;
    let mut out = BTreeMap::new();
    for (skill_id, raw) in obj {
        let alternatives: Vec<SkillAlternative> = raw
            .get_array("alternatives")?
            .iter()
            .map(|alt| {
                let precondition = alt.get("precondition").and_then(Value::as_str).filter(|s| !s.is_empty()).map(|s| s.to_string());
                let condition = alt.get_str("condition")?.to_string();
                let base_duration = alt.get_f64("baseDuration")?;
                let effects: Vec<SkillEffectDef> = alt
                    .get_array("effects")?
                    .iter()
                    .map(|ef| {
                        Ok(SkillEffectDef { effect_type_raw: ef.get_i64("type")? as i32, target: ef.get_i64("target")? as i32, modifier: ef.get_f64("modifier")?,
                            additional_activate: ef.get("additionalActivate").and_then(Value::as_f64).unwrap_or(0.0) as i32,
                            target_value: ef.get("targetValue").and_then(Value::as_f64).unwrap_or(0.0) as i32,
                            value_usage: ef.get("valueUsage").and_then(Value::as_f64).unwrap_or(1.0) as i32,
                            level_modifiers: opt_array(ef, "levelModifiers").iter().filter_map(Value::as_f64).collect() })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(SkillAlternative { precondition, condition, effects, base_duration,
                    cooldown: alt.get("cooldown").and_then(Value::as_f64).unwrap_or(0.0),
                    duration_usage: alt.get("durationUsage").and_then(Value::as_f64).unwrap_or(1.0) as i32 })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let rarity = raw.get_i64("rarity")? as i32;
        let wisdom_check = raw.get("wisdomCheck").and_then(Value::as_f64).map(|n| n != 0.0).unwrap_or(false);
        let has_green_tag = opt_array(raw, "tags").iter().filter_map(Value::as_f64)
            .any(|tag| (601.0..=615.0).contains(&tag));
        let priority = raw.get("priority").and_then(Value::as_f64).unwrap_or(0.0) as i32;
        let race_inactive = raw.get("raceInactive").and_then(Value::as_bool).unwrap_or(false);
        let is_general_skill = raw.get("isGeneralSkill").and_then(Value::as_bool).unwrap_or(true);
        out.insert(skill_id.clone(), SkillDef { rarity, priority, wisdom_check, alternatives, has_green_tag, race_inactive, is_general_skill });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Race spec (stdin JSON) parsing

fn skill_id_and_level(v: &Value) -> Result<(String, i32, bool), String> {
    if let Some(obj) = v.as_object() {
        let id = if let Some(s) = obj.get("skillId").and_then(Value::as_str) {
            s.to_string()
        } else if let Some(n) = obj.get("skillId").and_then(Value::as_f64) {
            (n as i64).to_string()
        } else {
            return Err("skill entry missing skillId".to_string());
        };
        let level = obj.get("level").and_then(Value::as_f64).map(|n| n as i32).filter(|&n| n != 0).unwrap_or(1);
        let is_unique = obj.get("isUnique").and_then(Value::as_bool).unwrap_or(false);
        Ok((id, level, is_unique))
    } else if let Some(s) = v.as_str() {
        Ok((s.to_string(), 1, false))
    } else if let Some(n) = v.as_f64() {
        Ok(((n as i64).to_string(), 1, false))
    } else {
        Err("invalid skill entry".to_string())
    }
}

struct HorseInputSpec {
    speed: f64,
    stamina: f64,
    power: f64,
    guts: f64,
    wisdom: f64,
    strategy_str: String,
    distance_aptitude_str: String,
    surface_aptitude_str: String,
    strategy_aptitude_str: String,
    mood: i32,
    gate_index: i32,
    popularity: i32,
    skills: Vec<(String, i32, bool)>,
    team_id: i64, chara_id: i64, fans: f64, wins: f64,
}

struct RaceSpec {
    trace_conserve_power: bool,
    experimental_later_stamina_mechanics: bool,
    seed: i64,
    player_horse_index: usize,
    course_id: i32,
    ground: String,
    weather: String,
    season: String,
    time: String,
    grade: String,
    is_dirt_grade: Option<bool>,
    horses: Vec<HorseInputSpec>,
    wisdom_checks: bool,
    experimental_grid: bool,
    single_mode: bool,
}

fn experimental_grid_enabled(v: &Value, horse_count: usize) -> Result<bool, String> {
    let Some(grid) = v.get("experimentalGrid") else { return Ok(false); };
    let valid = grid.as_object().is_some_and(|o| o.len() == 2)
        && grid.get("columns").and_then(Value::as_f64) == Some(20.0)
        && grid.get("rowSpacing").and_then(Value::as_f64) == Some(3.0)
        && (21..=120).contains(&horse_count);
    if !valid {
        return Err("experimentalGrid requires 21 through 120 horses, columns=20 and rowSpacing=3.0.".to_string());
    }
    Ok(true)
}

fn starting_coordinates(rank: usize, experimental_grid: bool) -> (f64, f64) {
    let column = if experimental_grid { rank % 20 } else { rank };
    let lane = (column as f64 + if column >= 9 { 0.6 } else { 0.0 }) / 18.0;
    let pos = if experimental_grid { -3.0 * (rank / 20) as f64 } else { 0.0 };
    (pos, lane)
}

fn parse_race_spec(v: &Value) -> Result<RaceSpec, String> {
    let seed = v.get_i64("seed")?;
    let course_id = v.get_i64("courseId")? as i32;
    let ground = v.get_str("groundCondition")?.to_string();
    let weather = v.get_str("weather")?.to_string();
    let season = v.get_str("season")?.to_string();
    let time = match v.get("time") { None => "midday", Some(_) => v.get_str("time")? }.to_string();
    let grade = match v.get("grade") { None => "g1", Some(_) => v.get_str("grade")? }.to_string();
    let is_dirt_grade = match v.get("isDirtGrade") {
        None | Some(Value::Null) => None,
        Some(value) => Some(value.as_bool().ok_or("isDirtGrade must be a boolean when supplied")?),
    };
    let horses_raw = v.get_array("horses")?;
    let experimental_grid = experimental_grid_enabled(v, horses_raw.len())?;
    let player_index = match v.get("playerHorseIndex") {
        None => 0.0,
        Some(value) => value.as_f64().ok_or("playerHorseIndex must be an integer within the horse array.")?,
    };
    if !player_index.is_finite() || player_index.fract() != 0.0
        || player_index < 0.0 || player_index >= horses_raw.len() as f64 {
        return Err("playerHorseIndex must be an integer within the horse array.".to_string());
    }
    let player_horse_index = player_index as usize;
    let mut horses = Vec::with_capacity(horses_raw.len());
    for (i, h) in horses_raw.iter().enumerate() {
        if v.get("experimentalRetailRuntime").and_then(Value::as_bool) == Some(true)
            && h.get("teamRank").map_or(false, |rank| rank.as_f64() != Some(0.0)) {
            return Err(format!("runner {i}: experimental runtime requires teamRank=0; nonzero team-rank bonus integration is unvalidated"));
        }
        let mood = h.get("mood").and_then(Value::as_f64).map(|n| n as i32).unwrap_or(0);
        let gate_index = h.get("gateIndex").and_then(Value::as_f64).map(|n| n as i32).unwrap_or(i as i32);
        let skills_raw = opt_array(h, "skills");
        let mut skills = Vec::with_capacity(skills_raw.len());
        for entry in &skills_raw {
            skills.push(skill_id_and_level(entry)?);
        }
        horses.push(HorseInputSpec {
            speed: h.get_f64("speed")?,
            stamina: h.get_f64("stamina")?,
            power: h.get_f64("power")?,
            guts: h.get_f64("guts")?,
            wisdom: h.get_f64("wisdom")?,
            strategy_str: h.get_str("strategy")?.to_string(),
            distance_aptitude_str: h.get_str("distanceAptitude")?.to_string(),
            surface_aptitude_str: h.get_str("surfaceAptitude")?.to_string(),
            strategy_aptitude_str: h.get_str("strategyAptitude")?.to_string(),
            mood,
            gate_index,
            popularity: h.get("popularity").and_then(Value::as_i64).unwrap_or(0) as i32,
            skills,
            team_id: h.get("teamId").and_then(Value::as_f64).unwrap_or(0.0) as i64,
            chara_id: h.get("charaId").and_then(Value::as_f64).unwrap_or(0.0) as i64,
            fans: h.get("fans").and_then(Value::as_f64).unwrap_or(0.0).max(0.0),
            wins: h.get("careerWins").and_then(Value::as_f64).unwrap_or(0.0).max(0.0),
        });
    }
    let wisdom_checks = v.get("wisdomChecks").and_then(Value::as_bool).unwrap_or(true);
    let single_mode = v.get("singleMode").and_then(Value::as_bool).unwrap_or(false);
    let experimental_later_stamina_mechanics = match v.get("experimentalLaterStaminaMechanics") {
        None => false,
        Some(value) => value.as_bool().ok_or("experimentalLaterStaminaMechanics must be a boolean.")?,
    };
    let trace_conserve_power = match v.get("traceConservePower") {
        None => false,
        Some(value) => value.as_bool().ok_or("traceConservePower must be a boolean")?,
    };
    Ok(RaceSpec { trace_conserve_power, seed, player_horse_index, course_id, ground, weather, season, time, grade, is_dirt_grade, horses, wisdom_checks, experimental_grid, single_mode, experimental_later_stamina_mechanics })
}

/// `_STRATEGY_IDS.get(spec["horses"][i]["strategy"], 2)` in Python: an
/// *exact*, case-sensitive string match against the raw input (not the
/// parsed `Strategy` enum, which is case-insensitive and accepts "SASHI").
fn strategy_id_for_output(raw: &str) -> f64 {
    match raw {
        "Nige" => 1.0,
        "Senkou" => 2.0,
        "Sasi" => 3.0,
        "Oikomi" => 4.0,
        "Oonige" => 5.0,
        _ => 2.0,
    }
}

// ---------------------------------------------------------------------------
// Builder construction, mirroring race_runner._new_builder

fn new_builder(
    conditions: &'static BTreeMap<String, activation_conditions::Condition>,
    seed: i64,
    i: usize,
    spec: &RaceSpec,
    h: &HorseInputSpec,
    retail_runtime: bool,
) -> Result<RaceSolverBuilder, String> {
    let seed_lo = (seed + i as i64 * 7919) as u32;
    let mut b = RaceSolverBuilder::new(1, conditions, seed_lo, race_solver_builder::race_seed_hi(seed_lo));
    b.course(spec.course_id);
    let mood = h.mood.max(-2).min(2);
    b.mood(mood);
    b.ground(&spec.ground)?;
    b.weather(&spec.weather)?;
    b.season(&spec.season)?;
    b.time(&spec.time)?;
    b.grade(&spec.grade)?;
    b.dirt_grade(spec.is_dirt_grade);
    b.gate_index(h.gate_index);
    b.num_umas(spec.horses.len() as i32);
    b.wisdom_checks(spec.wisdom_checks);
    b.single_mode(spec.single_mode);
    let desc = HorseSpec {
        speed: h.speed,
        stamina: h.stamina,
        power: h.power,
        guts: h.guts,
        wisdom: h.wisdom,
        strategy: Strategy::parse_str(&h.strategy_str)?,
        distance_aptitude: Aptitude::parse_str(&h.distance_aptitude_str, "distance")?,
        surface_aptitude: Aptitude::parse_str(&h.surface_aptitude_str, "surface")?,
        strategy_aptitude: Aptitude::parse_str(&h.strategy_aptitude_str, "strategy")?,
    };
    let mut same_style_count = 0;
    let mut other_style_counts = [0; 4];
    let favorites: Vec<_> = spec.horses.iter().filter(|other| other.popularity == 1).collect();
    let favorite_matches = if favorites.len() == 1 {
        Some(horse_types::strategy_matches(desc.strategy, Strategy::parse_str(&favorites[0].strategy_str)?))
    } else { None };
    let mut skill_counts = BTreeMap::new();
    for (j, other) in spec.horses.iter().enumerate() {
        if j != i {
            let style = Strategy::parse_str(&other.strategy_str)?.base_running_style();
            other_style_counts[(style - 1) as usize] += 1;
        }
        if horse_types::strategy_matches(desc.strategy, Strategy::parse_str(&other.strategy_str)?) {
            same_style_count += 1;
        }
        let ids: std::collections::BTreeSet<_> = other.skills.iter().map(|(id, _, _)| id).collect();
        for id in ids { *skill_counts.entry(id.clone()).or_insert(0) += 1; }
    }
    b.roster_conditions(same_style_count, favorite_matches, skill_counts, h.popularity,
                        spec.horses.iter().map(|other| other.chara_id).collect(), other_style_counts);
    let members: Vec<&HorseInputSpec> = if h.team_id != 0 { spec.horses.iter().filter(|other| other.team_id == h.team_id).collect() } else { vec![h] };
    let mut totals = [0.0; 5];
    for other in members {
        for (i, raw) in [other.speed, other.stamina, other.power, other.guts, other.wisdom].iter().enumerate() {
            if retail_runtime {
                // v66: value-usage 3-7 tiers read overcap-halved base stats (practice-campaigns 0251/0255, skill 210052).
                let base = if *raw > 1200.0 { 1200.0 + (*raw - 1200.0) * 0.5 } else { *raw };
                totals[i] = (totals[i] as f32 + retail_initialization::initial_base_wisdom(base, other.mood)) as f64;
            } else {
                totals[i] += race_solver_builder::base_stat(*raw, 1.0 + 0.02 * other.mood.clamp(-2,2) as f64);
            }
        }
    }
    b.race_context = race_solver::RaceContext { team_id: h.team_id, chara_id: h.chara_id, fans: h.fans, wins: h.wins, team_stats: Some(totals) };
    b.horse(desc);
    // Fully Charged is observed in Global; later stamina systems remain opt-in.
    b.with_asiwotameru();
    if spec.experimental_later_stamina_mechanics { b.with_stamina_syoubu(); }
    Ok(b)
}

fn is_usable(
    cache: &mut BTreeMap<String, bool>,
    conditions: &'static BTreeMap<String, activation_conditions::Condition>,
    spec: &RaceSpec,
    h: &HorseInputSpec,
    i: usize,
    skill_id: &str,
    level: i32,
) -> bool {
    if let Some(&ok) = cache.get(skill_id) {
        return ok;
    }
    let ok = (|| -> Result<(), String> {
        let mut probe = new_builder(conditions, spec.seed, i, spec, h, false)?;
        probe.add_skill(skill_id, level);
        probe.validate_skills()
    })()
    .is_ok();
    cache.insert(skill_id.to_string(), ok);
    ok
}

fn run_race(spec: &RaceSpec, conditions: &'static BTreeMap<String, activation_conditions::Condition>,
            retail: Option<&retail_initialization::Initialization>, retail_sections:bool,
            retail_runtime:bool, world_geometry:Option<Rc<retail_geometry::CourseGeometry>>,
            startup_pass:bool) -> Result<Value, String> {
    let course = course_data::get_course(spec.course_id);
    let dt=if retail_runtime {0.0666_f32 as f64} else {DT};

    let skill_events: Rc<RefCell<Vec<Value>>> = Rc::new(RefCell::new(Vec::new()));
    let conserve_power_events: Rc<RefCell<Vec<Value>>> = Rc::new(RefCell::new(Vec::new()));
    let kakari_start: Rc<RefCell<BTreeMap<usize, f64>>> = Rc::new(RefCell::new(BTreeMap::new()));
    // (horse_index, skill_id) -> (activation time, isUnique, modeled duration), until the
    // deactivate hook finalizes the record retaining intended duration;
    // quantized expiry ticks are not the replay duration.
    let pending_sequence = Rc::new(Cell::new(0usize));
    let pending_skills: Rc<RefCell<BTreeMap<(usize, i64), (f64, bool, f64, usize)>>> = Rc::new(RefCell::new(BTreeMap::new()));
    // Whether a skill id can actually be built for this race. Some skills'
    // conditions reference tokens this engine doesn't implement and those
    // fail from inside build() (not add_skill()), so a single bad skill
    // would otherwise abort the whole horse's build. Memoized across the
    // roster (most horses share most of their skills).
    let mut skill_usable: BTreeMap<String, bool> = BTreeMap::new();

    // Gate IDs still drive post-number skills. Lateral starts use dense gate
    // rank, matching captured partial-field lineups and each track's gate layout.
    let mut gate_order: Vec<usize> = (0..spec.horses.len()).collect();
    gate_order.sort_by_key(|&i| spec.horses[i].gate_index);
    let mut starts = vec![(0.0, 0.0); spec.horses.len()];
    for (rank, &i) in gate_order.iter().enumerate() {
        starts[i] = if spec.experimental_grid { starting_coordinates(rank, true) }
            else if retail_runtime {(0.,starting_grid::retail_starting_lane(rank,course.race_track_id,spec.horses.len()))}
            else { (0.0, starting_grid::starting_lane(rank, course.race_track_id, spec.horses.len())) };
        if retail_runtime {starts[i]=(starts[i].0 as f32 as f64,starts[i].1 as f32 as f64);}
    }
    let mut solvers = Vec::with_capacity(spec.horses.len());
    for (i, h) in spec.horses.iter().enumerate() {
        let mut builder = new_builder(conditions, spec.seed, i, spec, h, retail_runtime)?;
        builder.retail_initialization=retail.map(|init|init.horses[i].clone());
        if let Some(row)=&mut builder.retail_initialization {row.runtime_phase=retail_runtime;}
        for (skill_id, level, _is_unique) in &h.skills {
            builder.add_skill(skill_id, *level);
        }
        // skillId (as sent on the wire, e.g. "100011") -> whether the input
        // spec flagged it isUnique. Consulted by the activate hook below;
        // matches race_runner.py's per-horse `unique_ids` set.
        let unique_ids: BTreeSet<String> = h.skills.iter()
            .filter(|(_, _, is_unique)| *is_unique)
            .map(|(id, _, _)| id.clone())
            .collect();

        let horse_index = i;
        let events = skill_events.clone();
        let conserve_events = conserve_power_events.clone();
        let kstart = kakari_start.clone();
        let pending = pending_skills.clone();
        let sequence = pending_sequence.clone();
        builder.on_skill_activate = Some(Box::new(move |s, skill_id, perspective| {
            if perspective == race_solver::Perspective::Other { return; }
            if skill_id == "asitame" {
                let duration = s.conserve_release_duration();
                s.cp_clean.record_release(s.accumulatetime.get(), s.pos, duration, s.cp_acc, s.horse.raw_power);
                conserve_events.borrow_mut().push(json::obj(vec![
                    ("horseIndex", json::num(horse_index as f64)),
                    ("t", json::num(s.accumulatetime.get())),
                ]));
                return;
            }
            if skill_id == "kakari" {
                // Rushing (kakari) is not a real skill id -- record the
                // start time now; the deactivate hook finalizes the event
                // with the REAL elapsed duration once it ends (kakari
                // duration can be extended mid-race by an opponent's
                // EXTEND_KAKARI skill, so this must be measured at
                // deactivation, not read from a stored duration here).
                kstart.borrow_mut().insert(horse_index, s.accumulatetime.get());
                return;
            }
            let sid: i64 = match skill_id.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            if s.remaining_skill_duration(skill_id, perspective) <= 0.0 {
                events.borrow_mut().push(json::obj(vec![
                    ("horseIndex", json::num(horse_index as f64)),
                    ("skillId", json::num(sid as f64)),
                    ("t", json::num(s.accumulatetime.get())),
                    ("duration", json::num(0.0)),
                    ("isUnique", json::num(if unique_ids.contains(skill_id) { 1.0 } else { 0.0 })),
                ]));
                return;
            }
            // Timed activations end when their last tracked effect ends.
            // Preserve Python dictionary insertion order for unfinished effects.
            let ordinal = pending.borrow().get(&(horse_index, sid)).map(|x| x.3)
                .unwrap_or_else(|| { let n = sequence.get(); sequence.set(n + 1); n });
            pending.borrow_mut().insert(
                (horse_index, sid),
                (s.accumulatetime.get(), unique_ids.contains(skill_id), s.remaining_skill_duration(skill_id, perspective), ordinal),
            );
        }));

        let events2 = skill_events.clone();
        let kstart2 = kakari_start.clone();
        let pending2 = pending_skills.clone();
        builder.on_skill_deactivate = Some(Box::new(move |s, skill_id, perspective| {
            if perspective == race_solver::Perspective::Other { return; }
            if skill_id == "kakari" {
                let start_t = match kstart2.borrow_mut().remove(&horse_index) {
                    Some(t) => t,
                    None => return,
                };
                events2.borrow_mut().push(json::obj(vec![
                    ("horseIndex", json::num(horse_index as f64)),
                    ("skillId", Value::String("kakari".to_string())),
                    ("t", json::num(start_t)),
                    ("duration", json::num(s.accumulatetime.get() - start_t)),
                    ("isUnique", json::num(0.0)),
                ]));
                return;
            }
            let sid: i64 = match skill_id.parse() {
                Ok(v) => v,
                Err(_) => return,
            };
            if s.remaining_skill_duration(skill_id, perspective) > 0.0 {
                return;
            }
            let (start_t, is_unique, modeled_duration, _) = match pending2.borrow_mut().remove(&(horse_index, sid)) {
                Some(v) => v,
                None => return,
            };
            events2.borrow_mut().push(json::obj(vec![
                ("horseIndex", json::num(horse_index as f64)),
                ("skillId", json::num(sid as f64)),
                ("t", json::num(start_t)),
                // Intended replay duration, rather than the rounded-up expiry tick.
                ("duration", json::num(modeled_duration)),
                ("isUnique", json::num(if is_unique { 1.0 } else { 0.0 })),
            ]));
        }));

        let mut solver = match builder.build() {
            Ok(mut v) => v.pop().unwrap(),
            Err(error) => {
                if retail.is_some() {return Err(error);}
                let mut retry = new_builder(conditions, spec.seed, i, spec, h, retail_runtime)?;
                for (skill_id, level, _is_unique) in &h.skills {
                    if !is_usable(&mut skill_usable, conditions, spec, h, i, skill_id, *level) {
                        continue;
                    }
                    retry.add_skill(skill_id, *level);
                }
                retry.on_skill_activate = builder.on_skill_activate.take();
                retry.on_skill_deactivate = builder.on_skill_deactivate.take();
                retry.build()?.pop().unwrap()
            }
        };

        if let Some(init)=retail {solver.start_delay=init.horses[i].start_delay as f64;}
        // The null lane controller keeps this initial target throughout a
        // straight race, so both lane and target must use the actual gate.
        solver.set_starting_lane(starts[i].1);
        if spec.experimental_grid {
            solver.pos = starts[i].0;
        }

        if spec.trace_conserve_power { solver.cp_clean.enable_trace(); }
        solver.experimental_later_stamina_mechanics = spec.experimental_later_stamina_mechanics;
        solvers.push(solver);
    }

    if retail_sections || retail_runtime {
        let mut init=retail.ok_or("Retail sections require initialization")?.clone();
        let courses=json::parse(COURSE_DATA_JSON)?;
        let parameters=json::parse(include_str!("../data/global_parameters.json"))?;
        let horses:Vec<_>=solvers.iter().map(|s|(s.horse.wisdom as f32,(s.modifiers_kakari_chance*100.0) as f32)).collect();
        retail_initialization::initialize_sections_and_rushing(&mut init,&horses,
            courses.get(&spec.course_id.to_string()).ok_or("Missing retail course")?,
            parameters.get("parameters").ok_or("Missing retail parameters")?)?;
        let runtime_config=if retail_runtime {
            let raw=courses.get(&spec.course_id.to_string()).ok_or("Missing retail course")?;
            let event_geometry = match &world_geometry {
                Some(g) => g.clone(),
                None => Rc::new(retail_geometry::CourseGeometry::bundled(spec.course_id, course.distance as f32)?),
            };
            let mut config = race_solver::RetailRuntimeConfig::new(parameters.get("parameters").unwrap(),
                Rc::new(RefCell::new(init.rng.clone())),world_geometry,
                course.first_move_lane_point>0. && raw.get("emptyInMove").and_then(Value::as_bool).unwrap_or(true))?;
            Rc::get_mut(&mut config).unwrap().event_geometry = Some(event_geometry);
            Some(config)
        } else {None};
        for (solver,row) in solvers.iter_mut().zip(init.horses.iter()) {
            solver.section_modifier=row.section_modifiers.iter().map(|&v|v as f64).collect();
            solver.section_modifier.push(0.0);
            solver.kakari_start=match row.rushing_section {
                Some(section)=>((section-1) as f32*(course.distance as f32/24.0)) as f64,
                None=>course.distance+9999.0,
            };
            if let Some(config)=&runtime_config {
                solver.enable_retail_runtime(config.clone(),row,race_parameters::parse_ground_condition(&spec.ground)? as i32)?;
            }
        }
    }

    // Gives every solver live awareness of the rest of the field (order/
    // rank, gaps to the horse ahead/behind) -- not part of the original TS
    // engine, which has no notion of other horses at all.
    // Snapshot adjusted initial stat ranks before runtime skill expiry/changes.
    // Stable ties preserve roster order, matching the exported rank fields.
    let parameter_orders: Vec<Vec<usize>> = (0..2).map(|stat| {
        let mut order: Vec<usize> = (0..solvers.len()).collect();
        let value = |i: usize| {
            let s = &solvers[i];
            retail_physics::bounded_stat(if stat == 0 { s.horse.guts as f32 } else { s.horse.wisdom as f32 })
        };
        order.sort_by(|&a, &b| value(b).total_cmp(&value(a)));
        let mut ranks = vec![0; order.len()];
        for (rank, i) in order.into_iter().enumerate() { ranks[i] = rank; }
        ranks
    }).collect();
    let mut field = RaceField::new(solvers);
    field.player_horse_index = spec.player_horse_index;
    // Research-only UpdateFirstHorse candidate, disabled unless requested.
    if startup_pass {
        field.step_all(0., &(0..field.n).collect::<Vec<_>>());
    }

    let n = field.n;
    let mut rushing_scores: Vec<_> = (0..n).map(|_| stadium_score::RushingScore::new()).collect();
    let mut positioning_scores: Vec<_> = (0..n).map(|_| stadium_score::PositioningScore::new()).collect();
    let score_styles: Vec<_> = field.solvers.iter().map(|s| s.borrow().horse.strategy.base_running_style()).collect();
    let mut finished: Vec<Option<f64>> = vec![None; n];
    let mut finished_step: Vec<Option<f64>> = vec![None; n];
    let mut frames: Vec<Value> = Vec::new();
    let mut competition_events: Vec<Value> = Vec::new();
    let mut duel_counts: Vec<i32> = field.solvers.iter().map(|s| s.borrow().compete_fight_count).collect();
    let mut spot_started: Vec<bool> = field.solvers.iter().map(|s| s.borrow().spot_struggle_started).collect();

    // Selected-player spurt telemetry, independent of replay kind 6, which
    // is ReleaseConservePower and does not identify the player.
    let mut player_last_spurt_t: Option<f64> = None;

    // Serialize the actual initialized retail state before advancing the first tick.
    if retail_runtime {
        let initial: Vec<Value> = (0..n).map(|i| {
            let s = field.solvers[i].borrow();
            let blocker = s.retail.as_ref().and_then(|r| r.front_blocker)
                .map_or(-1, |(j, _)| j as i32);
            json::obj(vec![
                ("pos", json::num(s.pos)),
                ("lane", json::num(s.lane)),
                ("speed", json::num(s.retail.as_ref().map_or(s.current_speed, |r| r.observed_speed as f64))),
                ("hp", json::num(s.hp.hp_value().map(|v| v.max(0.0)).unwrap_or(0.0))),
                ("temptationMode", json::num(s.rushing_mode as f64)),
                ("blockFrontHorseIndex", json::num(blocker as f64)),
            ])
        }).collect();
        frames.push(json::obj(vec![("t", json::num(0.)), ("horses", json::arr(initial))]));
    }

    let mut t = 0.0f64;

    while finished.iter().any(|f| f.is_none()) && t < MAX_RACE_SECONDS {
        let active: Vec<usize> = (0..n).filter(|&i| finished[i].is_none()).collect();
        let prev_pos: BTreeMap<usize, f64> = active.iter().map(|&i| (i, field.solvers[i].borrow().pos)).collect();
        let visited = if retail_runtime { (0..n).collect::<Vec<_>>() } else { active.clone() };
        // Score observers see the cached ranks before CalcHorseOrder, and
        // the newly committed phase from each horse's Update.
        let ranks: Vec<_> = (0..n).map(|i| field.order_of(i)).collect();
        let leaders = stadium_score::style_leaders(&score_styles, &ranks);
        field.step_all(dt, &visited);
        for &i in &visited {
            let s = field.solvers[i].borrow();
            rushing_scores[i].update(s.is_kakari, dt);
            positioning_scores[i].update(s.retail.as_ref().map_or(s.phase, |r| r.phase as i32), Some(leaders[i]));
        }
        // Type 5 is CompeteFight. Emit only real count increments, never every
        // active tick or a synthetic event when a horse crosses the finish.
        for &i in &active {
            if field.solvers[i].borrow().spot_struggle_started && !spot_started[i] {
                competition_events.push(json::obj(vec![
                    ("t", json::num(t + dt)), ("type", json::num(4.0)),
                    ("horseIndex", json::num(i as f64)),
                ]));
                spot_started[i] = true;
            }
        }
        // Hypothesis companion: a runner may start a duel on its goal-crossing
        // snapshot, after its finish is recorded; emit real count increments
        // for every visited runner.
        for &i in &visited {
            let count = field.solvers[i].borrow().compete_fight_count;
            for _ in duel_counts[i]..count {
                competition_events.push(json::obj(vec![
                    ("t", json::num(t + dt)), ("type", json::num(5.0)),
                    ("horseIndex", json::num(i as f64)),
                ]));
            }
            duel_counts[i] = count;
        }
        if player_last_spurt_t.is_none() && n > 0 && field.solvers[spec.player_horse_index].borrow().is_last_spurt {
            player_last_spurt_t = Some(t + dt);
        }
        for &i in &active {
            let pos = field.solvers[i].borrow().pos;
            if if retail_runtime {pos > course.distance} else {pos >= course.distance} {
                if retail_runtime {
                    let end = t as f32 + dt as f32;
                    finished_step[i] = Some(end as f64);
                    finished[i] = Some(retail_physics::finish_time(end, dt as f32,
                        prev_pos[&i] as f32, pos as f32, course.distance as f32) as f64);
                } else {
                    let span = pos - prev_pos[&i];
                    let frac = if span > 0.0 { (course.distance - prev_pos[&i]) / span } else { 0.0 };
                    finished[i] = Some(t + dt * frac);
                    finished_step[i] = Some(t + dt);
                }
            }
        }
        t = if retail_runtime {(t as f32+dt as f32) as f64} else {t+dt};

        // Record every tick unconditionally -- real captured finish spreads
        // can exceed a flat "winner + grace seconds" cutoff (an 18-horse
        // field with a wide stat spread can spread 10+ seconds), and the
        // outer while loop already stops on its own once every horse has
        // finished (or MAX_RACE_SECONDS, a generous safety cap). Truncating
        // here left late finishers' frames stuck mid-track while their
        // results still reported the later time, producing a warp/speed-
        // burst right at the line (matches race_runner.py's fix).
        let horses: Vec<Value> = (0..n)
            .map(|i| {
                let s = field.solvers[i].borrow();
                let hp_val = s.hp.hp_value().map(|v| v.max(0.0)).unwrap_or(0.0);
                let blocker = if retail_runtime || finished[i].is_none() {
                    if let Some(r)=&s.retail {r.front_blocker.map_or(-1,|(j,_)|j as i32)}
                    else {field.front_blocker(i, s.pos, s.lane).map(|(j, _)| j as i32).unwrap_or(-1)}
                } else { -1 };
                json::obj(vec![("pos", json::num(s.pos)), ("speed", json::num(s.retail.as_ref().map_or(s.current_speed,|r|r.observed_speed as f64))),
                    ("hp", json::num(hp_val)), ("lane", json::num(s.lane)),
                    ("temptationMode", json::num(if retail_runtime || finished[i].is_none() { s.rushing_mode as f64 } else { 0.0 })),
                    ("blockFrontHorseIndex", json::num(blocker as f64))])
            })
            .collect();
        frames.push(json::obj(vec![("t", json::num(t)), ("horses", json::arr(horses))]));
    }

    for i in 0..n {
        if finished[i].is_none() {
            finished[i] = Some(t);
            finished_step[i] = Some(t);
        }
    }

    // If the player never entered her own last-spurt phase before the race
    // ended (MAX_RACE_SECONDS or an HP-exhaustion edge case) -- fall back to
    // the finish time to preserve this telemetry field's numeric contract.
    if player_last_spurt_t.is_none() && n > 0 {
        player_last_spurt_t = Some(finished_step[spec.player_horse_index].unwrap_or(t));
    }

    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| finished[a].unwrap().partial_cmp(&finished[b].unwrap()).unwrap());
    let mut order_index = vec![0usize; n];
    for (rank, &horse_i) in order.iter().enumerate() {
        order_index[horse_i] = rank;
    }

    let mut results = Vec::with_capacity(n);
    for i in 0..n {
        let s = field.solvers[i].borrow();
        let last_spurt = if s.last_spurt_transition > 0.0 { s.last_spurt_transition } else { -1.0 };
        results.push(json::obj(vec![
            ("finishOrder", json::num(order_index[i] as f64)),
            ("finishTime", json::num(finished_step[i].unwrap())),
            ("finishTimeRaw", json::num(finished[i].unwrap())),
            ("gutsOrder", json::num(if retail_runtime { parameter_orders[0][i] as f64 } else { 0. })),
            ("wizOrder", json::num(if retail_runtime { parameter_orders[1][i] as f64 } else { 0. })),
            ("hpEmptyOnRace", s.retail.as_ref().map_or(Value::Null, |r| Value::Bool(r.hp_empty_on_race))),
            ("startDelayTime", json::num(retail.map_or(s.start_delay, |init| init.horses[i].start_delay as f64))),
            ("lastSpurtStartDistance", json::num(last_spurt)),
            ("actualLastSpurtStartTime", s.actual_last_spurt_start_time.map(json::num).unwrap_or(Value::Null)),
            ("actualLastSpurtStartDistance", s.actual_last_spurt_start_distance.map(json::num).unwrap_or(Value::Null)),
            ("runningStyle", json::num(s.horse.strategy.base_running_style() as f64)),
            ("rushingStartCount", json::num(s.temptation_count as f64)),
            ("rushingPenaltyCount", json::num(rushing_scores[i].count as f64)),
            ("positioningPhases", Value::Array(positioning_scores[i].phases.iter().map(|&p| json::num(p as f64)).collect())),
        ]));
    }

    // Captures retain effect durations past the runner's finish. Use the
    // modeled duration when its solver stopped before observing expiry.
    let pending_final = pending_skills.borrow();
    let mut pending_ordered: Vec<_> = pending_final.iter().collect();
    pending_ordered.sort_by_key(|(_, value)| value.3);
    for ((horse_index, sid), (start_t, is_unique, modeled_duration, _)) in pending_ordered {
        skill_events.borrow_mut().push(json::obj(vec![
            ("horseIndex", json::num(*horse_index as f64)),
            ("skillId", json::num(*sid as f64)),
            ("t", json::num(*start_t)),
            ("duration", json::num(*modeled_duration)),
            ("isUnique", json::num(if *is_unique { 1.0 } else { 0.0 })),
        ]));
    }

    let skill_events_out = skill_events.borrow().clone();
    let mut result = json::obj(vec![
        ("seed", json::num(spec.seed as f64)),
        ("scoreTelemetryVersion", json::num(3.0)),
        ("runningStyleVersion", json::num(2.0)),
        ("raceInitializationVersion", json::num(1.0)),
        ("positionKeepVersion", json::num(1.0)),
        ("skillWindowVersion", json::num(1.0)),
        ("postNumberVersion", json::num(1.0)),
        ("skillOrderVersion", json::num(1.0)),
        ("lateralPhysicsVersion", json::num(2.0)),
        ("overtakeTimersVersion", json::num(2.0)),
        ("hpStateRatesVersion", json::num(1.0)),
        ("adjustedStatsVersion", json::num(1.0)),
        ("targetSpeedVersion", json::num(1.0)),
        ("laneSkillConditionsVersion", json::num(4.0)),
        ("courseDataVersion", json::num(3.0)),
        ("statAdjustmentVersion", json::num(1.0)),
        ("rosterPassiveVersion", json::num(2.0)),
        ("spurtConditionVersion", json::num(1.0)),
        ("elapsedTimeConditionVersion", json::num(1.0)),
        ("skillDurationVersion", json::num(1.0)),
        ("conservePowerEventVersion", json::num(2.0)),
        ("thresholdStatsVersion", json::num(1.0)),
        ("staminaRulesVersion", json::num(1.0)),
        ("startingGridVersion", json::num(1.0)),
        ("fieldTriggerVersion", json::num(6.0)),
        ("rushingModeVersion", json::num(1.0)),
        ("skillObservationVersion", json::num(1.0)),
        ("randomActivationVersion", json::num(6.0)),
        ("raceContextVersion", json::num(2.0)),
        ("skillReplayDurationVersion", json::num(1.0)),
        ("pacingModeVersion", json::num(2.0)),
        ("currentSpeedReceiverVersion", json::num(1.0)),
        ("experimentalLaterStaminaMechanics", Value::Bool(spec.experimental_later_stamina_mechanics)),
        ("movementVersion", json::num(1.0)),
        ("laneSelectionVersion", json::num(4.0)),
        ("competitionMechanicsVersion", json::num(1.0)),
        ("skillEventVersion", json::num(1.0)),
        ("skillValueVersion", json::num(4.0)),
        ("rosterConditionVersion", json::num(2.0)),
        ("frames", json::arr(frames)),
        ("results", json::arr(results)),
        ("skillEvents", json::arr(skill_events_out)),
        ("skillDetailObservationVersion", json::num(1.0)),
        ("skillDetailEvents", json::arr(field.solvers.iter().enumerate().flat_map(|(i, solver)| {
            solver.borrow().skill_detail_events.iter().map(|(sid, detail, t)| json::obj(vec![
                ("horseIndex", json::num(i as f64)), ("skillId", json::num(sid.parse::<i64>().unwrap() as f64)),
                ("detailIndex", json::num(*detail as f64)), ("t", json::num(*t))
            ])).collect::<Vec<_>>()
        }).collect())),
        ("skillTargetObservationVersion", json::num(1.0)),
        ("skillTargetEvents", json::arr(field.solvers.iter().enumerate().flat_map(|(i, solver)| {
            solver.borrow().skill_target_events.iter().map(|(sid, detail, t, targets)| json::obj(vec![
                ("horseIndex", json::num(i as f64)), ("skillId", json::num(sid.parse::<i64>().unwrap() as f64)),
                ("detailIndex", json::num(*detail as f64)), ("t", json::num(*t)),
                ("abilityTargets", json::arr(targets.iter().map(|selection| match selection {
                    Some(indices) => json::arr(indices.iter().map(|index| json::num(*index as f64)).collect()),
                    None => json::Value::Null,
                }).collect()))
            ])).collect::<Vec<_>>()
        }).collect())),
        ("additionalTargetObservationVersion", json::num(1.0)),
        ("additionalTargetEvents", json::arr(field.solvers.iter().enumerate().flat_map(|(i, solver)| {
            solver.borrow().additional_target_events.iter().map(|(sid, detail, t, targets)| json::obj(vec![
                ("horseIndex", json::num(i as f64)), ("skillId", json::num(sid.parse::<i64>().unwrap() as f64)),
                ("detailIndex", json::num(*detail as f64)), ("t", json::num(*t)),
                ("abilityIndexScope", json::Value::String("runtime_subset".to_string())),
                ("abilityTargets", json::arr(targets.iter().map(|selection| match selection {
                    Some(indices) => json::arr(indices.iter().map(|index| json::num(*index as f64)).collect()),
                    None => json::Value::Null,
                }).collect()))
            ])).collect::<Vec<_>>()
        }).collect())),
        ("competitionEvents", json::arr(competition_events)),
        ("conservePowerEvents", json::arr(conserve_power_events.borrow().clone())),
        ("playerLastSpurtTime", json::num(player_last_spurt_t.unwrap_or(t))),
        ("playerHorseIndex", json::num(spec.player_horse_index as f64)),
    ]);
    if spec.trace_conserve_power {
        if let Value::Object(map) = &mut result {
            map.insert("conservePowerDiagnosticVersion".into(), json::num(1.));
            map.insert("conservePowerDiagnostics".into(), json::arr(
                field.solvers.iter().enumerate().map(|(i, solver)| {
                    let s = solver.borrow();
                    s.cp_clean.diagnostics(i, s.cp_acc)
                }).collect()));
        }
    }
    if spec.single_mode {
        if let Value::Object(map) = &mut result {
            map.insert("singleModeVersion".to_string(), json::num(1.0));
        }
    }
    if retail_runtime {
        if let Value::Object(map) = &mut result {
            map.insert("postFinishMovementVersion".to_string(), json::num(1.0));
        }
    }
    if spec.experimental_grid {
        if let Value::Object(map) = &mut result {
            map.insert("experimentalGridVersion".to_string(), json::num(1.0));
        }
    }
    Ok(result)
}

// Frame cadence -- MUST stay identical to race_simulator.py's
// _DENSE_LEAD_FRAMES / _SPARSE_STRIDE / _downsample_frames. See the comment
// block above those constants for why real scenarios are dense through the
// start dash and sparse afterwards.
const DENSE_LEAD_FRAMES: usize = 17;
const SPARSE_STRIDE: usize = 16;

/// Thin `frames` down to exactly the set build_race_scenario would have kept.
///
/// The sim produces ~1,170 frames (1.5 MB of JSON) and Python keeps 89 of
/// them, so 92% of that blob was serialized here, pushed through a pipe and
/// parsed on the other side purely to be discarded. Doing the cut here is
/// worth ~36 ms per race on the Python side alone.
///
/// Python prepends a synthetic t=0 "gate" frame when the first simulated
/// frame starts after t=0, and it downsamples AFTER that prepend -- so the
/// index math below works in that post-prepend space and maps back, even
/// when a legacy path omits the gate frame. Retail now emits its actual
/// initialized frame. `gateFrameNeeded`
/// tells Python whether to add it; `framesDownsampled` tells Python the list
/// is already thinned and must not be thinned again.
fn downsample_result(result: Value) -> Value {
    let mut map = match result {
        Value::Object(m) => m,
        other => return other,
    };
    let frames = match map.remove("frames") {
        Some(Value::Array(f)) => f,
        Some(other) => {
            map.insert("frames".to_string(), other);
            return Value::Object(map);
        }
        None => return Value::Object(map),
    };

    let gate_needed = frames
        .first()
        .and_then(|f| f.get("t"))
        .and_then(Value::as_f64)
        .map_or(false, |t| t > 0.0);
    let off = if gate_needed { 1 } else { 0 };
    let eff_len = frames.len() + off;

    let mut keep = vec![false; eff_len];
    if eff_len <= 2 {
        for k in keep.iter_mut() {
            *k = true;
        }
    } else {
        for k in keep.iter_mut().take(DENSE_LEAD_FRAMES.min(eff_len)) {
            *k = true;
        }
        let mut i = 0;
        while i < eff_len {
            keep[i] = true;
            i += SPARSE_STRIDE;
        }
        keep[eff_len - 1] = true;
    }

    // build_race_scenario needs each horse's position/speed at the last frame
    // at or before its OWN finishTimeRaw, and it needs that at full
    // resolution -- re-deriving it from thinned frames would move the finish
    // sample to a nearby kept tick and change the encoded blob. Compute it
    // here, while the full list still exists, and send it alongside.
    let mut finish_frames: Vec<Value> = Vec::new();
    if let Some(results) = map.get("results").and_then(Value::as_array) {
        for (hi, r) in results.iter().enumerate() {
            let finish_t = r
                .get("finishTimeRaw")
                .and_then(Value::as_f64)
                .unwrap_or(f64::INFINITY);
            let mut best: Option<&Value> = None;
            for f in frames.iter() {
                let t = f.get("t").and_then(Value::as_f64).unwrap_or(0.0);
                if t <= finish_t {
                    best = Some(f);
                } else {
                    break;
                }
            }
            let entry = match best
                .and_then(|f| f.get("horses"))
                .and_then(Value::as_array)
                .and_then(|hs| hs.get(hi))
            {
                Some(h) => json::obj(vec![
                    ("pos", h.get("pos").cloned().unwrap_or(Value::Null)),
                    ("speed", h.get("speed").cloned().unwrap_or(Value::Null)),
                    (
                        "t",
                        best.and_then(|f| f.get("t")).cloned().unwrap_or(Value::Null),
                    ),
                ]),
                None => Value::Null,
            };
            finish_frames.push(entry);
        }
    }

    let raw_frame_count = frames.len();
    let kept: Vec<Value> = frames
        .into_iter()
        .enumerate()
        .filter(|(i, _)| keep[i + off])
        .map(|(_, f)| f)
        .collect();

    map.insert("frames".to_string(), Value::Array(kept));
    map.insert("finishFrames".to_string(), Value::Array(finish_frames));
    map.insert("rawFrameCount".to_string(), json::num(raw_frame_count as f64));
    map.insert("framesDownsampled".to_string(), Value::Bool(true));
    map.insert("gateFrameNeeded".to_string(), Value::Bool(gate_needed));
    Value::Object(map)
}

/// v23: in the game the race's horse index IS the gate order (every archived
/// official race has frame_order == horseIndex + 1; tools/compare_horseact.py
/// rejects anything else), so every retail tie-break and per-horse stream keyed
/// on the horse index -- the start-of-race pacemaker picked from an all-zero
/// order (race_field.rs RaceField::new -> update -> leading_eligible_in), the
/// stable rank sort, retail_initialization's per-horse random draws, the
/// stepping order -- is keyed on the gate. Server requests list the player's
/// runners first with random gates, which moved those tie-breaks onto list
/// position (index 0 won ~99% of identical packs). With the retail runtime the
/// field is therefore run in gate order and every per-horse output is mapped
/// back to request order. Requests already in gate order (all official
/// replays) are untouched, bit for bit.
/// Returns the (possibly reordered) request and perm[internal] = request index.
fn gate_order(mut spec: Value) -> Result<(Value, Option<Vec<usize>>), String> {
    if spec.get("experimentalRetailRuntime").and_then(Value::as_bool) != Some(true) {
        return Ok((spec, None));
    }
    let horses = match spec.get("horses").and_then(Value::as_array) {
        Some(h) => h.clone(),
        None => return Ok((spec, None)),
    };
    let key = |i: usize| horses[i].get("gateIndex").and_then(Value::as_f64).unwrap_or(i as f64);
    let mut perm: Vec<usize> = (0..horses.len()).collect();
    perm.sort_by(|&a, &b| key(a).partial_cmp(&key(b)).unwrap_or(std::cmp::Ordering::Equal));
    if perm.iter().enumerate().all(|(i, &p)| i == p) {
        return Ok((spec, None));
    }
    let mut inverse = vec![0usize; perm.len()];
    for (internal, &request) in perm.iter().enumerate() { inverse[request] = internal; }
    if let Value::Object(map) = &mut spec {
        map.insert("horses".to_string(), Value::Array(perm.iter().map(|&i| horses[i].clone()).collect()));
        if let Some(player) = map.get("playerHorseIndex").and_then(Value::as_f64) {
            if player >= 0.0 && (player as usize) < inverse.len() && player.fract() == 0.0 {
                map.insert("playerHorseIndex".to_string(), json::num(inverse[player as usize] as f64));
            }
        }
    }
    Ok((spec, Some(perm)))
}

/// Map a gate-order result back to request order: per-horse arrays are
/// reordered and every horse-index field is translated.
fn restore_request_order(result: &mut Value, perm: &[usize]) {
    let map_index = |v: &mut Value| {
        if let Value::Number(n) = v {
            if *n >= 0.0 && n.fract() == 0.0 && (*n as usize) < perm.len() {
                *n = perm[*n as usize] as f64;
            }
        }
    };
    let reorder = |items: &mut Vec<Value>| {
        if items.len() != perm.len() { return; }
        let mut out = vec![Value::Null; perm.len()];
        for (internal, item) in std::mem::take(items).into_iter().enumerate() { out[perm[internal]] = item; }
        *items = out;
    };
    let map = match result { Value::Object(m) => m, _ => return };
    if let Some(v) = map.get_mut("playerHorseIndex") { map_index(v); }
    for key in ["results", "finishFrames", "conservePowerDiagnostics"] {
        if let Some(Value::Array(items)) = map.get_mut(key) { reorder(items); }
    }
    if let Some(Value::Array(frames)) = map.get_mut("frames") {
        for frame in frames.iter_mut() {
            if let Value::Object(f) = frame {
                if let Some(Value::Array(horses)) = f.get_mut("horses") {
                    for h in horses.iter_mut() {
                        if let Value::Object(h) = h {
                            if let Some(v) = h.get_mut("blockFrontHorseIndex") { map_index(v); }
                        }
                    }
                    reorder(horses);
                }
            }
        }
    }
    for key in ["skillEvents", "skillDetailEvents", "skillTargetEvents", "additionalTargetEvents",
                "competitionEvents", "conservePowerEvents", "conservePowerDiagnostics"] {
        if let Some(Value::Array(events)) = map.get_mut(key) {
            for e in events.iter_mut() {
                if let Value::Object(e) = e {
                    if let Some(v) = e.get_mut("horseIndex") { map_index(v); }
                    if let Some(Value::Array(targets)) = e.get_mut("abilityTargets") {
                        for t in targets.iter_mut() {
                            if let Value::Array(ix) = t { for v in ix.iter_mut() { map_index(v); } }
                        }
                    }
                }
            }
        }
    }
}

fn run() -> Result<(), String> {
    let courses = load_course_table(COURSE_DATA_JSON)?;
    course_data::init_courses(courses);
    let skills = load_skill_table(SKILL_DATA_JSON)?;
    race_solver_builder::init_skills(skills);

    let conditions: &'static BTreeMap<String, activation_conditions::Condition> = Box::leak(Box::new(activation_conditions::build_conditions()));

    // Persistent-worker loop: one JSON spec per line in, one JSON result per
    // line out, flushed each iteration.
    //
    // The two tables loaded above are ~600 KB of JSON and cost ~21 ms to parse
    // -- that used to be paid on EVERY race, because each race was a fresh
    // process. Paying it once per process instead is the entire point. The
    // per-request work below is stateless (the RNG is seeded from the spec),
    // so races cannot leak into each other across iterations.
    //
    // A caller that writes one spec and closes stdin still behaves exactly as
    // before, so this stays compatible with the old one-shot invocation.
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            break; // EOF -- parent closed the pipe
        }
        if line.trim().is_empty() {
            continue;
        }
        // A bad spec must not take the worker down with it: report the error
        // as a JSON reply and keep serving. The caller treats an "error" key
        // as a failed run and falls back to the Python engine for that race.
        let outcome = (|| -> Result<Value, String> {
            let spec_json = json::parse(&line)?;
            // v23: the retail runtime runs the field in gate order (see gate_order).
            let (spec_json, gate_perm) = gate_order(spec_json)?;
            // Replace the table on every request: the persistent worker must
            // never retain one client's installed skill definitions for another.
            let skills = match spec_json.get("skillData") {
                Some(data) => load_skill_table(&json::to_string(data))?,
                None => load_skill_table(SKILL_DATA_JSON)?,
            };
            race_solver_builder::init_skills(skills);
            let spec = parse_race_spec(&spec_json)?;
            let retail_runtime=spec_json.get("experimentalRetailRuntime").and_then(Value::as_bool)==Some(true);
            if spec.trace_conserve_power && !retail_runtime {
                return Err("traceConservePower requires experimentalRetailRuntime".into());
            }
            let startup_pass = match spec_json.get("experimentalRetailStartup") {
                None => false,
                Some(value) => value.as_bool().ok_or("experimentalRetailStartup must be a boolean")?,
            };
            if startup_pass && !retail_runtime {
                return Err("experimentalRetailStartup requires the experimental runtime".into());
            }
            let world_geometry=if spec_json.get("experimentalRetailGeometry").and_then(Value::as_bool)==Some(true) {
                if !retail_runtime || spec.experimental_grid {
                    return Err("World geometry requires the experimental runtime and standard starting grid".into());
                }
                Some(Rc::new(retail_geometry::CourseGeometry::bundled(spec.course_id,
                    course_data::get_course(spec.course_id).distance as f32)?))
            } else {None};
            let retail_sections=spec_json.get("experimentalRetailSections").and_then(Value::as_bool)==Some(true);
            let retail=if spec_json.get("experimentalRetailInitialization").and_then(Value::as_bool)==Some(true) || retail_sections || retail_runtime {
                let input_horses:Vec<_>=spec.horses.iter().map(|horse| json::obj(vec![
                    ("skills",Value::Array(horse.skills.iter().map(|(sid,level,_)|json::obj(vec![
                        ("skill_id",json::num(sid.parse::<f64>().unwrap_or(0.0))),
                        ("level",json::num(*level as f64))])).collect())),
                    // v9: the skill wisdom lottery sees Wit after the >1200 halving, then mood.
                    ("base_wisdom",json::num(retail_initialization::initial_base_wisdom(
                        if horse.wisdom > 1200.0 { 1200.0 + (horse.wisdom - 1200.0) * 0.5 } else { horse.wisdom },
                        horse.mood as i32) as f64)),
                    // v69: retail initialization sees the base style; Runaway (Oonige) keeps the Front
                    // Runner force-in coefficient and its random draw (doc 13.190).
                    ("running_style",json::num(Strategy::parse_str(&horse.strategy_str).unwrap().base_running_style() as f64))
                ])).collect();
                let raw_courses=json::parse(COURSE_DATA_JSON)?;
                let raw_skills=spec_json.get("skillData").cloned().unwrap_or(json::parse(SKILL_DATA_JSON)?);
                let parameters=json::parse(include_str!("../data/global_parameters.json"))?;
                let seed=i32::try_from(spec.seed).map_err(|_|"Retail seed must be a signed 32-bit integer")?;
                Some(retail_initialization::initialize(seed,&input_horses,
                    raw_courses.get(&spec.course_id.to_string()).ok_or("Missing retail course")?,&raw_skills,
                    parameters.get("parameters").ok_or("Missing retail parameters")?)?)
            } else {None};
            let raw_result=run_race(&spec, conditions,retail.as_ref(),retail_sections,retail_runtime,world_geometry,startup_pass)?;
            let mut result = if spec_json.get("retainAllFrames").and_then(Value::as_bool)==Some(true) {
                raw_result
            } else {downsample_result(raw_result)};
            if let Value::Object(map)=&mut result {
                map.insert("experimentalRetailInitialization".into(),Value::Bool(retail.is_some()));
            }
            if let (Value::Object(map), Some(signature)) = (&mut result, spec_json.get("dataSignature")) {
                map.insert("dataSignature".to_string(), signature.clone());
            }
            if let Some(perm) = &gate_perm {
                restore_request_order(&mut result, perm);
            }
            Ok(result)
        })();
        let reply = match outcome {
            Ok(v) => v,
            Err(e) => json::obj(vec![("error", Value::String(e))]),
        };
        writeln!(out, "{}", json::to_string(&reply)).map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("race runner error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod experimental_grid_tests {
    use super::*;

    #[test]
    fn grid_requires_exact_contract_and_hundred_horses() {
        let valid = json::parse(r#"{"experimentalGrid":{"columns":20,"rowSpacing":3.0}}"#).unwrap();
        assert!(experimental_grid_enabled(&valid, 100).unwrap());
        for count in [21, 24, 68, 99, 100, 105, 120] {
            assert!(experimental_grid_enabled(&valid, count).unwrap());
        }
        for count in [0, 18, 20, 121] {
            assert!(experimental_grid_enabled(&valid, count).is_err());
        }
        for raw in ["null", "false", "[]", "{}",
                    r#"{"columns":10,"rowSpacing":3}"#,
                    r#"{"columns":20,"rowSpacing":2.5}"#,
                    r#"{"columns":"20","rowSpacing":3}"#,
                    r#"{"columns":20,"rowSpacing":true}"#,
                    r#"{"columns":20,"rowSpacing":3,"extra":1}"#] {
            let spec = json::parse(&format!(r#"{{"experimentalGrid":{raw}}}"#)).unwrap();
            assert!(experimental_grid_enabled(&spec, 100).is_err(), "{raw}");
        }
        assert!(!experimental_grid_enabled(&json::parse("{}").unwrap(), 100).unwrap());
    }

    #[test]
    fn grid_has_five_separate_rows_and_twenty_ordinary_stalls() {
        for rank in 0..100 {
            let (pos, lane) = starting_coordinates(rank, true);
            assert_eq!(pos, -3.0 * (rank / 20) as f64);
            assert_eq!(lane, starting_coordinates(rank % 20, false).1);
            assert!((-12.0..=0.0).contains(&pos));
            assert!((0.0..1.1).contains(&lane));
            for other in 0..rank {
                let (other_pos, other_lane) = starting_coordinates(other, true);
                assert!((pos - other_pos).abs() >= 3.0 || (lane - other_lane).abs() > 0.05);
            }
        }
        // Ordinary 24-runner lineups must not wrap into the experimental grid.
        assert_eq!(starting_coordinates(23, false), (0.0, 23.6 / 18.0));
    }
}
