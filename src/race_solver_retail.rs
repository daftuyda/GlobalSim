//! Experimental solver integration. The worker flag remains closed until the
//! field passes and skill runtime are wired and differentially validated.
use super::*;
use crate::json::Value;
use crate::retail_geometry::CourseGeometry;
use crate::retail_initialization::HorseInitialization;
use crate::retail_lane::{
    self, MoveContext, MoveParameters, MoveState, TargetContext, TargetState,
};
use crate::retail_physics::{self, Acceleration, Parameters as Physics, TargetSpeed};
use crate::retail_position_keep as pace;
use crate::retail_random::RetailRandom;
use crate::retail_spurt::SpurtState;
use crate::retail_states::{DownhillParameters, DownhillState, RushingParameters, RushingState, SlopeState};

pub(crate) struct RetailRuntimeConfig {
    pub physics: Rc<Physics>,
    pub pace: pace::Parameters,
    pub lane: retail_lane::TargetParameters,
    pub movement: MoveParameters,
    pub rushing: RushingParameters,
    pub downhill: DownhillParameters,
    pub random: Rc<RefCell<RetailRandom>>,
    pub geometry: Option<Rc<CourseGeometry>>,
    pub event_geometry: Option<Rc<CourseGeometry>>,
    pub empty_in_move: bool,
    pub duel_distance: f32,
    pub duel_lane: f32,
    pub duel_speed_coefficients: [f32; 3],
    pub duel_accel_coefficients: [f32; 3],
    pub spot_speed_coefficients: [f32; 3],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course_data::{Corner, Slope, Straight};
    use crate::horse_types::Aptitude;
    fn course() -> &'static CourseData {
        Box::leak(Box::new(CourseData {
            race_track_id: 1,
            distance: 2400.,
            distance_type: 3,
            surface: 1,
            turn: 1,
            course_set_status: vec![],
            corners: vec![Corner {
                start: 1600.,
                length: 200.,
            }],
            straights: vec![Straight {
                start: 1800.,
                end: 2400.,
                front_type: 1,
            }],
            slopes: vec![
                Slope {
                    start: 400.,
                    length: 350.,
                    slope: -20000.,
                },
                Slope {
                    start: 1200.,
                    length: 100.,
                    slope: 15000.,
                },
            ],
            max_lane: 1.3_f32 as f64,
            first_move_lane_point: 375.,
            empty_in_move: true,
        }))
    }
    fn config(seed: i32) -> Rc<RetailRuntimeConfig> {
        let parameters = crate::json::parse(include_str!("../data/global_parameters.json"))
        .unwrap();
        RetailRuntimeConfig::new(
            parameters.get("parameters").unwrap(),
            Rc::new(RefCell::new(RetailRandom::new(seed))),
            None,
            true,
        )
        .unwrap()
    }
    fn solver(
        trace: &Value,
        course: &'static CourseData,
        config: Rc<RetailRuntimeConfig>,
    ) -> RaceSolver {
        let style = trace.get_i64("style").unwrap() as i32;
        let stats = trace.get_array("stats").unwrap();
        let n = |i: usize| stats[i].as_f64().unwrap();
        let horse = HorseParameters {
            speed: n(0),
            stamina: n(1),
            power: n(2),
            guts: n(3),
            wisdom: n(4),
            strategy: Strategy::from_i32(style).unwrap(),
            distance_aptitude: Aptitude::A,
            surface_aptitude: Aptitude::A,
            strategy_aptitude: Aptitude::A,
            raw_stamina: n(1),
            raw_power: n(2),
        };
        let ground = trace.get_i64("ground").unwrap() as i32;
        let hp = HpPolicy::Game(crate::hp_policy::GameHpState::new(
            course,
            ground,
            Rule30CARng::new(2, 0),
        ));
        let skills = trace
            .get_array("skills")
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let trigger = Region {
                    start: x.get_f64("start").unwrap(),
                    end: 2400.,
                };
                let id = (990000 + i).to_string();
                PendingSkill {
                    anyskill_nodes: Vec::new(),
                    random_activation_order: None,
                    detail_index: 0,
                    priority: x.get_i64("priority").unwrap_or(0) as i32,
                    skill_id: id.clone(),
                    numeric_skill_id: id.parse().ok(),
                    has_green_tag: false,
                    perspective: Perspective::Self_,
                    rarity: if x.get_i64("rarity").unwrap_or(1)==2 {SkillRarity::Gold} else {SkillRarity::White},
                    trigger: trigger.clone(),
                    extra_condition: Box::new(|_, _| true),
                    prerequisite: None,
                    prerequisite_satisfied: false,
                    cooldown: x.get_f64("cooldown").unwrap(),
                    cooldown_key: id,
                    windows: vec![trigger],
                    effects: vec![SkillEffect {
                        effect_type: match x.get_i64("type").unwrap() {
                            27 => SkillType::TargetSpeed,
                            21 => SkillType::CurrentSpeed,
                            22 => SkillType::CurrentSpeedWithNaturalDeceleration,
                            31 => SkillType::Accel,
                            9 => SkillType::Recovery,
                            28 => SkillType::LaneMoveSpeedUp,
                            35 => SkillType::SetTargetLane,
                            37 => SkillType::ActivateRandomGold,
                            _ => panic!("unknown probe"),
                        },
                        base_duration: x.get_f64("duration").unwrap(),
                        modifier: x.get_f64("modifier").unwrap(),
                        target_is_debuff: crate::race_field::is_skill_demerit(x.get_i64("type").unwrap() as i32, x.get_f64("modifier").unwrap()),
                        value_usage: x.get_i64("usage").unwrap_or(1) as i32,
                        duration_usage: 1,
                        additional_activate: 0,
                        target: x.get_i64("target").unwrap_or(1) as i32,
                        target_value: x.get_i64("target_value").unwrap_or(0) as i32,
                    }],
                }
            })
            .collect();
        let mut solver = RaceSolver::new(
            horse,
            course,
            Rule30CARng::new(1, 0),
            skills,
            hp,
            None,
            None,
            None,
            0,
            false,
        );
        solver.lane = trace.get_f64("lane").unwrap_or(0.3_f32 as f64);
        solver.lane_target = solver.lane;
        let row = HorseInitialization {
            start_delay: trace.get_f64("start_delay").unwrap() as f32,
            rushing_section: Some(3),
            section_modifiers: (0..24)
                .map(|i| (-0.05_f64 + i as f64 * 0.005) as f32)
                .collect(),
            force_in_move_coefficient: 0.05,
            ..Default::default()
        };
        solver.enable_retail_runtime(config, &row, ground).unwrap();

        solver
    }
    fn check(
        solver: &RaceSolver,
        row: &Value,
        rng: &Rc<RefCell<RetailRandom>>,
        tick: usize,
        label: &str,
    ) {
        let values = [
            solver.accumulatetime.get(),
            solver.pos,
            solver.current_speed,
            solver.retail.as_ref().unwrap().observed_speed as f64,
            solver.target_speed,
            solver.hp.hp_value().unwrap(),
            solver.lane,
            solver.lane_target,
            solver.lane_change_speed,
            solver.last_spurt_transition,
            solver.last_spurt_speed,
        ];
        let expected = row.as_array().unwrap();
        for (i, value) in values.iter().enumerate() {
            assert_eq!(
                (*value as f32).to_bits(),
                expected[i].as_i64().unwrap() as u32,
                "{label} tick {tick} channel {i}: {value}"
            );
        }
        let flags = [
            solver.phase as usize,
            solver.rushing_mode as usize,
            usize::from(solver.is_downhill_mode),
            usize::from(solver.is_last_spurt),
            rng.borrow().draw_count,
            solver.activate_count[0] as usize,
            solver.activate_count[1] as usize,
            solver.activate_count[2] as usize,
            solver.activate_count_heal as usize,
            solver.activate_count_last_frame as usize,
            usize::from(solver.spot_struggle_started),
            usize::from(solver.spot_struggle_active),
            solver.compete_fight_count as usize,
            usize::from(solver.duel_active),
            solver.retail.as_ref().unwrap().phase,
        ];
        for (i, value) in flags.iter().enumerate() {
            assert_eq!(
                *value,
                expected[i + 11].as_i64().unwrap() as usize,
                "{label} tick {tick} flag {i}"
            );
        }
    }
    #[test]
    fn zero_time_preserves_course_event_readiness() {
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_solver_port.json")).unwrap();
        let mut s = solver(&fixture.get_array("traces").unwrap()[0], course(), config(1));
        s.pos = 500.; // Within the fixture's downhill; it is not exposed yet.
        s.retail.as_mut().unwrap().course_events_ready = false;
        for _ in 0..2 {
            s.prepare_retail_tick(0., None, false);
            assert!(!s.retail.as_ref().unwrap().course_events_ready);
            assert_eq!(s.slope_per, 0.);
        }
        s.prepare_retail_tick(0.0666, None, false);
        assert!(s.retail.as_ref().unwrap().course_events_ready);
        assert_eq!(s.slope_per, 0.);
        s.prepare_retail_tick(0.0666, None, false);
        assert_eq!(s.slope_per, -20000.);
    }
    #[test]
    fn complete_isolated_races_match_python_tick_bits() {
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_solver_port.json"))
        .unwrap();
        let course = course();
        for trace in fixture.get_array("traces").unwrap() {
            let config = config(trace.get_i64("seed").unwrap() as i32);
            let rng = config.random.clone();
            let field = RaceField::new(vec![solver(trace, course, config)]);
            for (tick, row) in trace.get_array("rows").unwrap().iter().enumerate() {
                let mut s = field.solvers[0].borrow_mut();
                s.step_forward(fixture.get_f64("dt").unwrap(), Some(&field));
                check(&s, row, &rng, tick, "isolated");
                drop(s);
                field.update();
            }
            assert!(field.solvers[0].borrow().pos >= 2400.);
        }
    }
    #[test]
    fn retail_phase_uses_cached_enum_and_preserves_boundary_window() {
        use crate::race_parameters::{RaceParameters, RaceParametersWithSkillId};
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_solver_port.json"))
        .unwrap();
        let course = course();
        let mut s = solver(&fixture.get_array("traces").unwrap()[0], course, config(1));
        let table = Box::leak(Box::new(crate::activation_conditions::build_conditions()));
        let extra = RaceParametersWithSkillId {
            params: &RaceParameters::default(),
            skill_id: "probe".into(),
        };
        let expressions = [
            ("phase==3", [false, false, false, true]),
            ("phase!=3", [true, true, true, false]),
            ("phase<2", [true, true, false, false]),
            ("phase<=2", [true, true, true, false]),
            ("phase>1", [false, false, true, true]),
            ("phase>=2", [false, false, true, true]),
        ];
        for (expression, expected) in expressions {
            let op = crate::condition_parser::parse_with_retail_runtime(
                table,
                expression,
                Some(vec![]),
                true,
            )
            .unwrap();
            let input = vec![Region::new(0., 2400.)];
            let (regions, check) = op.apply(&input, course, &s.horse, &extra).unwrap();
            assert_eq!(regions, input);
            for (phase, &answer) in expected.iter().enumerate() {
                s.pos = 2399.; // distance cannot substitute for the cached phase
                s.retail.as_mut().unwrap().phase = phase;
                s.phase = (phase as i32).min(2);
                assert_eq!(
                    check.as_ref().unwrap()(&s, None),
                    answer,
                    "{expression} {phase}"
                );
            }
        }
    }

    #[test]
    fn whole_field_races_match_python_tick_bits() {
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_field_port.json"))
        .unwrap();
        let course = course();
        for (race, trace) in fixture.get_array("races").unwrap().iter().enumerate() {
            let config = config(trace.get_i64("seed").unwrap() as i32);
            let rng = config.random.clone();
            let solvers = trace
                .get_array("horses")
                .unwrap()
                .iter()
                .map(|t| solver(t, course, config.clone()))
                .collect();
            let field = RaceField::new(solvers);
            for (tick, rows) in trace.get_array("rows").unwrap().iter().enumerate() {
                let active: Vec<_> = field
                    .solvers
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| (s.borrow().pos < 2400.).then_some(i))
                    .collect();
                field.step_all(fixture.get_f64("dt").unwrap(), &active);
                for (i, row) in rows.as_array().unwrap().iter().enumerate() {
                    check(
                        &field.solvers[i].borrow(),
                        row,
                        &rng,
                        tick,
                        &format!("race {race} runner {i}"),
                    );
                }
            }
            assert!(field.solvers.iter().all(|s| s.borrow().pos >= 2400.));
        }
    }
}

impl RetailRuntimeConfig {
    pub fn new(
        value: &Value,
        random: Rc<RefCell<RetailRandom>>,
        geometry: Option<Rc<CourseGeometry>>,
        empty_in_move: bool,
    ) -> Result<Rc<Self>, String> {
        let group = |key| {
            value
                .get(key)
                .ok_or_else(|| format!("Missing retail runtime group {key}"))
        };
        let empty_in_move = geometry.as_ref().map_or(empty_in_move, |g|g.empty_in_move);
        Ok(Rc::new(Self {
            physics: Rc::new(Physics::from_value(value)?),
            pace: pace::Parameters::from_value(group("PositionKeep")?)?,
            lane: retail_lane::TargetParameters::from_value(value)?,
            movement: MoveParameters::from_value(value)?,
            rushing: RushingParameters::from_value(group("Temptation")?)?,
            downhill: DownhillParameters::from_value(group("Slope")?)?,
            random,
            event_geometry: geometry.clone(),
            geometry,
            empty_in_move,
            duel_distance: group("CompeteFight")?.get_f64("DistanceGap")? as f32,
            duel_lane: group("CompeteFight")?.get_f64("LaneGap")? as f32,
            spot_speed_coefficients: [group("CompeteTop")?.get_f64("AddParam1Coef1")? as f32, group("CompeteTop")?.get_f64("AddParam1Coef2")? as f32, group("CompeteTop")?.get_f64("AddParam1Coef3")? as f32],
            duel_speed_coefficients: [group("CompeteFight")?.get_f64("AddParam1Coef1")? as f32, group("CompeteFight")?.get_f64("AddParam1Coef2")? as f32, group("CompeteFight")?.get_f64("AddParam1Coef3")? as f32],
            duel_accel_coefficients: [group("CompeteFight")?.get_f64("AddParam2Coef1")? as f32, group("CompeteFight")?.get_f64("AddParam2Coef2")? as f32, group("CompeteFight")?.get_f64("AddParam2Coef3")? as f32],
        }))
    }
}

pub(crate) struct RetailRuntimeState {
    pub skill_cooldowns: BTreeMap<String, Timer>,
    pub config: Rc<RetailRuntimeConfig>,
    pub rushing: RushingState,
    pub downhill: DownhillState,
    pub slope: SlopeState,
    pub pace: pace::State,
    pub lane: TargetState,
    live_horses: Vec<retail_lane::Horse>,
    pub spurt: SpurtState,
    pub force_in_coefficient: f32,
    pub observed_speed: f32,
    pub duel_preupdate_speed: f32,
    pub duel_preupdate_finished: bool,
    pub duel_preupdate_final_straight: bool,
    /// Hypothesis: the first duel snapshot after a goal-crossing movement still
    /// treats that runner as unfinished and in the final straight for new duels.
    pub duel_goal_crossing_snapshot: bool,
    pub minimum: f32,
    pub phase: usize,
    pub lane_max: f32,
    pub front_blocker: Option<(usize, f32)>,
    pub world: [f32; 2],
    pub delay_remaining: f32,
    pub prepared: bool,
    pub course_events_ready: bool,
    pub corner: i32,
    pub final_corner: bool,
    pub pending_hp: f32,
    pub finished: bool,
    pub hp_empty_on_race: bool,
    pub after_goal_time: f32,
}

impl RaceSolver {
    pub(crate) fn collect_retail_duel_proximity(&mut self, rows: &[(f32, f32, bool)], dt: f32) {
        let r = self.retail.as_mut().unwrap();
        r.duel_preupdate_speed = r.observed_speed;
        let index = self.field_index.unwrap();
        let (position, lane, straight) = rows[index];
        let r = self.retail.as_mut().unwrap();
        let crossing = std::mem::take(&mut r.duel_goal_crossing_snapshot);
        r.duel_preupdate_finished = position >= self.course.distance as f32 && !crossing;
        r.duel_preupdate_final_straight = straight || crossing;
        let config = &self.retail.as_ref().unwrap().config;
        // Keep surviving entries in place; each timer depends only on its own
        // preceding value and this immutable pre-movement snapshot.
        for (j, &(other_pos, other_lane, other_straight)) in rows.iter().enumerate() {
            if j != index
                && (other_pos - position).abs() <= config.duel_distance
                && (other_lane - lane).abs() <= config.duel_lane
                && (straight || other_straight) {
                self.duel_proximity.entry(j)
                    .and_modify(|v| *v = (*v as f32 + dt) as f64)
                    .or_insert(0.);
            } else {
                self.duel_proximity.remove(&j);
            }
        }
    }

    /// Called once after gate effects and shared section/rushing initialization,
    /// matching the existing experimental Python runner's setup order.
    pub(crate) fn enable_retail_runtime(
        &mut self,
        config: Rc<RetailRuntimeConfig>,
        row: &HorseInitialization,
        ground: i32,
    ) -> Result<(), String> {
        if self.retail.is_some() {
            return Err("Retail solver runtime is already configured".into());
        }
        let distance = self.course.distance as f32;
        let start = row
            .rushing_section
            .map(|s| (s - 1) as f32 * (distance / 24.))
            .unwrap_or(distance + 9999.);
        self.start_delay = row.start_delay as f64;
        self.section_modifier = row
            .section_modifiers
            .iter()
            .map(|&x| x as f64)
            .chain(std::iter::once(0.))
            .collect();
        if self.section_modifier.len() != 25 {
            return Err("Retail solver requires 24 initialized section modifiers".into());
        }
        let world = match &config.geometry {
            Some(g) => g.position(self.pos as f32, self.lane as f32)?,
            None => [0., 0.],
        };
        self.hp = HpPolicy::Game(crate::hp_policy::GameHpState::with_retail(
            self.course,
            ground,
            config.physics.clone(),
            config.random.clone(),
        ));
        self.hp.init(&self.horse);
        let p = &config.physics;
        let aptitude =
            ([10500u32, 10000, 9000, 8000, 6000, 4000, 2000, 1000][self.horse.distance_aptitude as usize] as f32 * 0.0001_f32);
        self.base_target_speed = [0, 1, 2].map(|phase| {
            p.base_target_speed(
                self.horse.speed as f32,
                self.horse.strategy as usize,
                phase,
                aptitude,
                distance,
            ) as f64
        });
        // Preserve the initialized section target while the start delay is pending.
        self.target_speed = (self.base_target_speed[0] as f32 + self.section_modifier[0] as f32) as f64;
        self.min_speed = p.minimum_speed(self.horse.guts as f32, distance) as f64;
        self.last_spurt_speed = p.maximum_spurt_speed(
            self.horse.speed as f32,
            self.horse.guts as f32,
            self.horse.strategy as usize,
            aptitude,
            distance,
        ) as f64;
        self.kakari_start = start as f64;
        let rushing = RushingState::new(config.rushing, self.horse.strategy as usize, start);
        let lane_max = config
            .geometry
            .as_ref()
            .map(|g| g.lane_max_at(self.pos as f32))
            .unwrap_or(self.course.max_lane as f32);
        self.armed_scan = false;
        self.hill_idx = -1;
        self.slope_per = 0.;
        self.is_downhill_mode = false;
        self.retail = Some(RetailRuntimeState {
            skill_cooldowns: BTreeMap::new(),
            config,
            rushing,
            downhill: Default::default(),
            slope: Default::default(),
            pace: Default::default(),
            lane: Default::default(),
            live_horses: Vec::new(),
            spurt: Default::default(),
            force_in_coefficient: row.force_in_move_coefficient,
            observed_speed: self.current_speed as f32,
            duel_preupdate_speed: self.current_speed as f32,
            duel_preupdate_finished: false,
            duel_preupdate_final_straight: false,
            duel_goal_crossing_snapshot: false,
            minimum: 3.,
            phase: 0,
            lane_max,
            front_blocker: None,
            world,
            delay_remaining: row.start_delay,
            prepared: false,
            course_events_ready: false,
            corner: 0,
            final_corner: false,
            pending_hp: 0.,
            finished: false,
            hp_empty_on_race: false,
            after_goal_time: 0.,
        });
        Ok(())
    }

    pub(crate) fn pace_cancel_current_speed(&self) -> f64 {
        let values = self.active_current_speed_skills.iter()
            .filter(|s| !s.natural_deceleration).map(|s| s.modifier);
        if self.retail.is_none() {
            return values.sum();
        }
        retail_physics::apply_additive_modifiers(0., values.map(|v| v as f32)) as f64
    }

    pub(crate) fn retail_current_speed(&self, base: f64) -> f64 {
        if self.retail.is_none() {
            return base + self.modifiers_current_speed.acc + self.modifiers_current_speed.err;
        }
        let ordinary = self.pace_cancel_current_speed() as f32;
        let natural = retail_physics::apply_additive_modifiers(0.,
            self.active_current_speed_skills.iter()
                .filter(|s| s.natural_deceleration).map(|s| s.modifier as f32));
        ((natural + ordinary) + base as f32) as f64
    }

    pub(crate) fn retail_target_speed_with_current_skills(&self) -> f32 {
        retail_physics::apply_additive_modifiers(self.target_speed as f32,
            self.active_current_speed_skills.iter()
                .filter(|s| !s.natural_deceleration).map(|s| s.modifier as f32))
    }

    pub(crate) fn retail_lane_context(&self) -> TargetContext {
        let retail = self.retail.as_ref().unwrap();
        TargetContext {
            lane_preference: self.lane_preference_flags(),
            owner: retail_lane::Horse {
                point: retail_lane::Row {
                    position: self.pos as f32,
                    lane: self.lane as f32,
                },
                speed: retail.observed_speed,
                target: self.retail_target_speed_with_current_skills(),
            },
            phase: self.phase,
            max_lane: retail.lane_max,
            vision: self.visible_distance() as f32,
            first_move_point: retail.config.geometry.as_ref().map_or(self.course.first_move_lane_point as f32, |g|g.first_move_lane_point),
            final_corner_start: self.course.corners.last().map(|c| c.start as f32),
            empty_in_move: retail.config.empty_in_move,
            previous_target: self.lane_target as f32,
            fixed_goal: self
                .active_aux_skills
                .iter()
                .rev()
                .find(|s| s.effect_type == SkillType::SetTargetLane)
                .map_or(0., |s| s.modifier as f32),
            has_hp: self.hp.has_remaining_hp(),
            pace_down: self.is_pace_down,
        }
    }

    pub(crate) fn retail_other_horses(&self, field: Option<&RaceField>) -> Vec<retail_lane::Horse> {
        let mut others = Vec::new();
        self.fill_retail_other_horses(field, &mut others);
        others
    }

    fn fill_retail_other_horses(&self, field: Option<&RaceField>, others: &mut Vec<retail_lane::Horse>) {
        others.clear();
        if let Some(field) = field {
            others.extend(field
                .solvers
                .iter()
                .enumerate()
                .filter_map(|(i, other)| {
                    if Some(i) == self.field_index {
                        return None;
                    }
                    let s = other.borrow();
                    Some(retail_lane::Horse {
                        point: retail_lane::Row {
                            position: s.pos as f32,
                            lane: s.lane as f32,
                        },
                        speed: s
                            .retail
                            .as_ref()
                            .map_or(s.current_speed as f32, |r| r.observed_speed),
                        target: s.retail_target_speed_with_current_skills(),
                    })
                }));
        }
    }

    pub(crate) fn prepare_retail_tick(
        &mut self,
        dt: f32,
        field: Option<&RaceField>,
        activate: bool,
    ) {
        // v14: retail is_activate_any_skill tests the previous tick's own
        // activations (official casts are exactly one tick after another
        // own activation in 37/37 corpus casts of skill 120011).
        let total: i32 = self.activate_count.iter().sum();
        self.activate_count_previous_tick = total - self.activate_total_at_tick_start;
        self.activate_total_at_tick_start = total;
        self.activated_ids_previous_tick = std::mem::take(&mut self.activated_ids_this_tick);
        let retail = self.retail.as_mut().unwrap();
        if retail.finished {
            retail.after_goal_time += dt;
        }
        for timer in &self.timers {
            timer.set((timer.get() as f32 + dt) as f64);
        }
        if let Some(geometry) = &retail.config.event_geometry {
            (retail.corner, retail.final_corner) = geometry.corner_state_at(
                self.pos as f32, retail.course_events_ready);
        }
        if retail.course_events_ready {
            retail.slope.update(self.pos as f32, &self.course.slopes);
        }
        self.hill_idx = retail.slope.region.map_or(-1, |i| i as i64);
        self.slope_per = retail.slope.value as f64;
        // The opening AI pass precedes the first course-event update.
        // Zero-time UpdateFirstHorse does not run RaceEventManager.Update.
        if dt > 0. {
            retail.course_events_ready = true;
        }
        // Whole-field priority scheduling will call preparation and skill
        // processing separately. This entry point supports isolated stepping.
        self.expire_skill_effects(field);
        self.start_expired_skill_cooldowns();
        self.activate_count_last_frame = 0;
        if activate {
            self.process_skill_priority(None, field);
        }
        let retail = self.retail.as_mut().unwrap();
        retail.prepared = true;
    }

    fn update_retail_pace(&mut self, dt: f32, field: Option<&RaceField>) {
        let (field, index, pacemaker) =
            match field.and_then(|f| Some((f, self.field_index?, f.pacemaker()?))) {
                Some(x) => x,
                None => return,
            };
        let (pacemaker_pos, pacemaker_style) = if pacemaker == index {
            (self.pos, self.horse.strategy)
        } else {
            let other = field.solvers[pacemaker].borrow();
            (other.pos, other.horse.strategy)
        };
        let retail = self.retail.as_ref().unwrap();
        let context = pace::Context {
            distance: self.course.distance as f32,
            position: self.pos as f32,
            wisdom: self.horse.wisdom as f32,
            strategy: self.horse.strategy as usize,
            rushing: self.is_kakari,
            rushing_mode: retail.rushing.mode,
            phase: self.phase,
            rank: field.order_of(index),
            pacemaker_rank: field.order_of(pacemaker),
            pacemaker_style: pacemaker_style as usize,
            pacemaker_gap: (pacemaker_pos - self.pos) as f32,
            pacemaker_is_self: pacemaker == index,
            later_style_ahead: field.later_style_ahead(index, self.horse.strategy),
            ordinary_nige_count: field.ordinary_front_runner_count(),
            behind_gap: field.distance_to_behind(index, self.pos).map(|x| x as f32),
            has_speed_skill: self.pace_cancel_current_speed() > 0.
                || self.retail_target_modifiers.apply(0.) > 0.,
        };
        let retail = self.retail.as_mut().unwrap();
        retail.pace.update(
            context,
            dt,
            &retail.config.pace,
            &mut *retail.config.random.borrow_mut(),
        );
        self.is_pace_down = retail.pace.mode == Some(pace::Mode::PaceDown);
        self.pos_keep_speed_coef = retail.pace.coefficient as f64;
        // v8 conserve power: accumulate time in position-keep sections, normal mode x6.7, pace-down x4.2,
        // times the running-style coefficient.
        let section = (self.course.distance as i64 / 24) as f64;
        let (start, end) = (retail.config.pace.start_section() as f64, retail.config.pace.end_section() as f64);
        if section * (start - 1.) <= self.pos && self.pos < section * end {
            let rate = match retail.pace.mode { None => 6.7, Some(pace::Mode::PaceDown) => 4.2, _ => 0.0 };
            let style = match self.horse.strategy as usize { 2 => 0.75, 4 => 0.7, _ => 1.0 };
            self.cp_acc += dt as f64 * rate * style;
            if self.spot_struggle_active { self.cp_struggle = true; }
            if self.is_kakari { self.cp_kakari = true; }
        }
    }

    fn update_retail_spurt(&mut self) {
        if self.phase < 2 {
            return;
        }
        let mut spurt = std::mem::take(&mut self.retail.as_mut().unwrap().spurt);
        let active = spurt.update(self.pos as f32, || {
            // Stats are sampled when the plan is recalculated. Computing this
            // value on ticks that keep the existing plan has no effect.
            let config = &self.retail.as_ref().unwrap().config;
            let maximum = config.physics.maximum_spurt_speed(
                self.horse.speed as f32,
                self.horse.guts as f32,
                self.horse.strategy as usize,
                ([10500u32, 10000, 9000, 8000, 6000, 4000, 2000, 1000][self.horse.distance_aptitude as usize] as f32 * 0.0001_f32),
                self.course.distance as f32,
            );
            self.hp
                .get_last_spurt_pair(self.pos, maximum as f64, self.base_target_speed[2]);
            match &self.hp {
                HpPolicy::Game(hp) => hp.last_retail_spurt_plan.unwrap(),
                _ => unreachable!("Retail solver needs game HP"),
            }
        });
        self.last_spurt_transition = spurt.start as f64;
        self.last_spurt_speed = spurt.speed as f64;
        self.last_spurt_calc_count = spurt.count as i32;
        self.last_spurt_calc_result = spurt.result;
        self.retail.as_mut().unwrap().spurt = spurt;
        if active && !self.is_last_spurt {
            self.actual_last_spurt_start_time = Some(self.accumulatetime.get());
            self.actual_last_spurt_start_distance = Some(self.pos);
        }
        self.is_last_spurt = active;
    }

    pub(super) fn step_forward_retail(&mut self, dt: f32, field: Option<&RaceField>) -> f64 {
        if !self.retail.as_ref().unwrap().prepared {
            let others = self.retail_other_horses(field);
            let context = self.retail_lane_context();
            let retail = self.retail.as_mut().unwrap();
            retail.lane.collect(&context, &others, retail.config.lane);
            self.prepare_retail_tick(dt, field, true);
        }
        let config = self.retail.as_ref().unwrap().config.clone();
        self.retail.as_mut().unwrap().prepared = false;
        let hp_change = std::mem::replace(&mut self.retail.as_mut().unwrap().pending_hp, 0.);
        if hp_change != 0. {
            self.hp.spend(-hp_change as f64);
            if hp_change > 0. && self.phase >= 2 {
                self.retail.as_mut().unwrap().spurt.pending = true;
            }
        }
        let blocker = self.retail.as_ref().unwrap().front_blocker;
        // v78: HorseRaceInfo.UpdateMinSpeed (0x3438f00) gives a finished runner (flag 0x150)
        // MinSpeed 5.0 and skips the front-blocker check. A runner blocked by a finished
        // runner inherits it (min of the two), so with no HP left its target is the
        // TargetSpeedMin floor (13), frozen when it crosses the goal.
        let finished_runner = self.retail.as_ref().unwrap().finished;
        let mut minimum = if finished_runner {
            5.
        } else if self.start_dash {
            3.
        } else {
            config
                .physics
                .minimum_speed(self.horse.guts as f32, self.course.distance as f32)
        };
        if !self.start_dash && !finished_runner {
            if let Some((i, _)) = blocker {
                let other = field.unwrap().solvers[i].borrow();
                minimum = minimum.min(
                    other
                        .retail
                        .as_ref()
                        .map_or(other.min_speed as f32, |r| r.minimum),
                );
            }
        }
        self.min_speed = minimum as f64;
        self.retail.as_mut().unwrap().minimum = minimum;
        let (remaining, gate_fraction) = retail_physics::advance_start_delay(
            self.retail.as_ref().unwrap().delay_remaining, dt);
        self.retail.as_mut().unwrap().delay_remaining = remaining;
        let gate_fraction = match gate_fraction { Some(value) => value, None => return 0. };
        let finished = self.retail.as_ref().unwrap().finished;
        if !finished {
            self.update_retail_pace(dt, field);
        }
        let mut others = std::mem::take(&mut self.retail.as_mut().unwrap().live_horses);
        self.fill_retail_other_horses(field, &mut others);
        let context = self.retail_lane_context();
        self.retail
            .as_mut()
            .unwrap()
            .lane
            .update_targets(&context, &others, dt, config.lane);
        let transition = if finished { 0 } else { self.retail.as_mut().unwrap().rushing.step(
            self.pos as f32,
            dt,
            &mut *config.random.borrow_mut(),
        ) };
        self.is_kakari = self.retail.as_ref().unwrap().rushing.active;
        self.rushing_mode = self.retail.as_ref().unwrap().rushing.mode as u8;
        if transition == 1 {
            self.temptation_count += 1;
            self.call_activate_hook("kakari", Perspective::Self_);
        }
        if transition == -1 {
            self.call_deactivate_hook("kakari", Perspective::Self_);
        }
        if !finished && !self.is_last_spurt {
            let mode = self.retail.as_ref().unwrap().pace.mode;
            self.cp_clean.update(self.accumulatetime.get(), dt, self.pos,
                self.course.distance, self.horse.raw_power, self.horse.strategy as usize,
                !matches!(mode, None | Some(pace::Mode::PaceDown)),
                mode == Some(pace::Mode::PaceDown),
                // Preparation already advanced this timer. Natural expiry is
                // visible now; distance/cutoff exits remain post-movement.
                self.is_kakari, self.spot_struggle_active && self.spot_struggle_timer.get() < 0.);
        }

        let context = self.retail_lane_context();
        if config.empty_in_move && self.retail.as_ref().unwrap().after_goal_time < 1. {
            self.lane_target = self.retail.as_mut().unwrap().lane.target(
            &context,
            &others,
            config.lane,
            &mut *config.random.borrow_mut(),
            ) as f64;
        }
        self.retail.as_mut().unwrap().live_horses = others;
        let was_spurting = self.is_last_spurt;
        self.update_retail_spurt();
        if !was_spurting && self.is_last_spurt {
            self.release_conserve_power_now(field);
        }
        let transition = self.retail.as_mut().unwrap().downhill.update(
            self.slope_per < 0.,
            self.horse.wisdom as f32,
            dt,
            &mut *config.random.borrow_mut(),
            config.downhill,
        );
        self.is_downhill_mode = self.retail.as_ref().unwrap().downhill.active;
        if transition == 1 {
            self.call_activate_hook("downhill", Perspective::Self_);
        }
        if transition == -1 {
            self.call_deactivate_hook("downhill", Perspective::Self_);
        }
        // Diagnostic hypothesis: remove the expired speed receiver before target calculation.
        if self.spot_struggle_active && self.spot_struggle_timer.get() >= 0. {
            self.retail_target_modifiers.remove(1);
        }
        let (section_index, target_phase) = retail_physics::target_speed_indices(
            self.pos as f32, self.course.distance as f32);
        let base = if self.is_last_spurt {
            self.last_spurt_speed
        } else {
            self.base_target_speed[target_phase]
        } as f32;
        let section = if self.is_last_spurt {
            0.
        } else {
            self.section_modifier[section_index] as f32
        };
        let retail = self.retail.as_ref().unwrap();
        let target = TargetSpeed {
            base,
            section,
            pace: self.pos_keep_speed_coef as f32,
            force_in: if retail.lane.force_in {
                retail.force_in_coefficient
            } else {
                0.
            },
            power: self.horse.power as f32,
            moving_lane_skill: !self.active_lane_speed_skills.is_empty()
                && self.moved_lane_this_tick,
            slope: (self.slope_per / 10000.) as f32,
            downhill: self.is_downhill_mode,
            hp_remaining: self.hp.has_remaining_hp(),
            minimum,
        };
        if !finished {
            self.target_speed = config.physics.target_speed_with_late_terms_iter(
            target,
            self.retail_target_modifiers.values_except_tokens(&[1, 4]),
            self.retail_target_modifiers.value(1).unwrap_or(0.),
            self.retail_target_modifiers.value(4).unwrap_or(0.),
            ) as f64;
        }
        let cap = blocker.map_or(30., |(i, gap)| {
            let other = field.unwrap().solvers[i].borrow();
            config.physics.blocking_speed_cap(
                minimum,
                other
                    .retail
                    .as_ref()
                    .map_or(other.current_speed as f32, |r| r.observed_speed),
                gap,
            )
        });
        let mut movement = MoveState {
            lane: self.lane as f32,
            target: self.lane_target as f32,
            speed: self.lane_change_speed as f32,
            continue_time: self.move_lane_continue_time,
            continue_direction: self.move_lane_continue_direction,
        };
        let retail = self.retail.as_ref().unwrap();
        self.moved_lane_this_tick = movement.step(
            MoveContext {
                position: self.pos as f32,
                power: self.horse.power as f32,
                phase: self.phase,
                rank: field
                    .zip(self.field_index)
                    .map_or(1, |(f, i)| f.order_of(i)),
                first_move_point: retail.config.geometry.as_ref().map_or(self.course.first_move_lane_point as f32, |g|g.first_move_lane_point),
                max_lane: retail.lane_max,
                side: retail.lane.side.unwrap(),
                modifiers: self.active_lane_speed_skills.iter().map(|s| s.modifier as f32),
            },
            dt,
            config.movement,
        );
        self.lane = movement.lane as f64;
        self.lane_target = movement.target as f64;
        self.lane_change_speed = movement.speed as f64;
        self.move_lane_continue_time = movement.continue_time;
        self.move_lane_continue_direction = movement.continue_direction;
        let current = (self.current_speed + self.modifiers_one_frame_accel) as f32;
        let target = self.target_speed as f32;
        let acceleration = if finished {
            // _UpdateSelfSpeed uses a dedicated finished-runner deceleration;
            // CalcAccelPerSec returns 1.0 for a finished runner below target.
            if current > target { -4.0 } else if current < target { 1.0 } else { 0.0 }
        } else if !self.hp.has_remaining_hp() {
            -1.2
        } else if current > target {
            if self.is_pace_down {
                -0.5
            } else {
                PHASE_DECELERATION[self.phase as usize] as f32
            }
        } else {
            let cp = self.cp_accel_token.unwrap_or(u64::MAX);
            let excluded = [cp, 4];
            config.physics.acceleration_iter(
                Acceleration {
                    power: self.horse.power as f32,
                    strategy: self.horse.strategy as usize,
                    phase: self.phase as usize,
                    // v73: InitProperGroundType (0x3435e10) stores the integer
                    // race_proper_ground_rate times the float32 0.0001 (cvtdq2ps, mulss),
                    // like InitProperDistance; C, F and G are 1 ulp below the decimals.
                    ground: [10500u32, 10000, 9000, 8000, 7000, 5000, 3000, 1000]
                        [self.horse.surface_aptitude as usize] as f32 * 0.0001_f32,
                    // InitProperDistance converts integer master basis points
                    // to float, then multiplies by the float32 0.0001 constant.
                    distance: [10000u32, 10000, 10000, 10000, 10000, 6000, 5000, 4000]
                        [self.horse.distance_aptitude as usize] as f32 * 0.0001_f32,
                    uphill: self.slope_per > 0.,
                    start_dash: self.start_dash,
                },
                // v69: the Conserve Power release and duel accelerations are summed after
                // the skill acceleration modifiers, not in activation order (doc 13.188).
                self.retail_accel_modifiers.values_except_tokens(&excluded)
                    .chain(self.retail_accel_modifiers.value(cp))
                    .chain(self.retail_accel_modifiers.value(4)),
            )
        };
        self.accel = acceleration as f64;
        let mut speed = current + acceleration * dt;
        speed = if current <= target {
            speed.min(target)
        } else {
            speed.max(target)
        };
        let dash_limit = config.physics.start_dash_speed(self.course.distance as f32);
        if self.start_dash {
            speed = speed.min(dash_limit);
        }
        speed = speed.min(cap).max(minimum);
        self.current_speed = speed as f64;
        let actual = (self.retail_current_speed(speed as f64) as f32)
            .max(minimum)
            .min(cap);
        self.retail.as_mut().unwrap().observed_speed = actual;
        let mut distance = actual * dt;
        if gate_fraction != 1. {
            distance = (distance as f64 * gate_fraction) as f32;
        }
        let ctx = HpTickCtx {
            is_pace_down: self.is_pace_down,
            is_downhill_mode: self.is_downhill_mode,
            is_kakari: self.is_kakari,
            phase: self.phase,
            current_speed: self.current_speed,
            // The bookkeeping flag is cleared after movement; an exhausted
            // timer must not charge an extra tick of competition HP cost.
            // Evidence: five pre-movement-divergence official HP windows.
            is_compete_top: self.spot_struggle_active && self.spot_struggle_timer.get() < 0.0,
            is_runaway: self.horse.strategy == Strategy::Oonige,
        };
        if !finished && self.start_dash {
            if let HpPolicy::Game(hp) = &mut self.hp {
                hp.tick_start_dash(ctx, dt);
            }
        } else if !finished {
            self.hp.tick(ctx, dt as f64);
        }
        if !finished && !self.hp.has_remaining_hp() {
            self.retail.as_mut().unwrap().hp_empty_on_race = true;
        }
        let phase = retail_physics::phase_boundaries(self.course.distance as f32)
        .iter()
        .filter(|&&boundary| self.pos >= boundary as f64)
        .count();
        self.phase = phase.min(2) as i32;
        self.retail.as_mut().unwrap().phase = phase;
        let next = match &config.geometry {
            Some(g) => {
                let retail = self.retail.as_mut().unwrap();
                let (next, world) = g
                    .advance(self.pos as f32, self.lane as f32, distance, retail.world)
                    .expect("Retail solver position within verified geometry");
                retail.world = world;
                next
            }
            None => self.pos as f32 + distance,
        };
        self.pos = next as f64;
        if !finished && self.pos > self.course.distance {
            // ClearSkills stops new activations, not receiver-owned modifiers.
            self.retail.as_mut().unwrap().finished = true;
            self.retail.as_mut().unwrap().duel_goal_crossing_snapshot = true;
            self.pending_skills.clear();
        }
        if self.start_dash && actual >= dash_limit {
            self.start_dash = false;
            self.modifiers_accel.add(-24.);
        }
        self.modifiers_one_frame_accel = 0.;
        dt as f64
    }
}
