//! Port of race_field.py -- multi-horse field coordination. Not part of the
//! original TS engine (which runs every RaceSolver in isolation, oblivious
//! to other horses).
//!
//! `RaceField` owns every solver in `RefCell`s so one can be borrowed
//! mutably (to step it) while its own methods borrow the others
//! immutably to answer blocking/lane/order queries -- the same access
//! pattern Python gets for free via shared references. Solvers do not hold
//! a reference back to the field: instead `field: Option<&RaceField>` is
//! threaded as an explicit parameter through `RaceSolver::step` and every
//! condition-checking closure (see `activation_conditions::DynCond`), so
//! nothing here needs a self-referential pointer or unsafe code.
//!
//! Ported already carrying the perf fixes validated on the Python engine:
//! each solver's own roster index is stored on it directly (`field_index`)
//! rather than found by a linear scan, `side_room`/`is_surrounded` early-out
//! once both/all flags are set, and `pacemaker()` is computed once per tick
//! and cached rather than recomputed per asking horse.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

use crate::horse_types::{strategy_matches, Strategy};
use crate::race_solver::{RaceSolver, SkillEffect, SkillRarity};

#[path = "race_field_retail.rs"]
mod retail_impl;

struct ExternalEffect {
    source: usize, target: usize, skill_id: String, rarity: SkillRarity,
    effect: SkillEffect, duration: f64,
}

/// 1 horse-lane = 1/18 course-width (doc). `lane` fields elsewhere are
/// stored in course-width units; this converts a horse-lane count into
/// that unit wherever the doc states a threshold in horse-lanes.
pub const HORSE_LANE: f64 = 1.0 / 18.0;

/// Doc's pacemaker strategy groups, most-forward first.
const PACEMAKER_GROUPS: &[&[Strategy]] = &[&[Strategy::Nige, Strategy::Oonige], &[Strategy::Senkou], &[Strategy::Sasi], &[Strategy::Oikomi]];

/// Global SkillTriggerOtherCharacterActivateAdvantageSkill special-cases 9;
/// other types negate RaceUtil.IsSkillDemerit (0x3613a10).
pub(crate) fn is_advantage_ability(kind: i32, value: f64) -> bool {
    if kind == 9 {value > 0.0} else {!is_skill_demerit(kind, value)}
}

pub(crate) fn is_skill_demerit(kind: i32, value: f64) -> bool {
    match kind {
        13 | 14 => true,
        10 => value > 1.0,
        29 => !(value < 0.0),
        1..=5 | 8 | 9 | 21 | 22 | 27 | 28 | 30 | 31 | 32 => value < 0.0,
        _ => false,
    }
}

pub struct RaceField {
    retail_block: Option<crate::retail_lane::BlockParameters>,
    pub solvers: Vec<RefCell<RaceSolver>>,
    pub n: usize,
    pub player_horse_index: usize,
    /// Each solver's fixed strategy, mirrored outside its `RefCell` so
    /// `pacemaker()` can scan every horse's strategy -- including the one
    /// currently mid-`step()` and thus already mutably borrowed -- without
    /// re-entering that `RefCell`.
    strategies: Vec<Strategy>,
    teams: Vec<i64>,
    characters: Vec<i64>,
    pub heal_history: RefCell<Vec<usize>>,
    advantage_activations: RefCell<BTreeSet<(usize, i32)>>,
    previous_advantage_activations: RefCell<BTreeSet<(usize, i32)>>,
    overtake_cooldowns: RefCell<BTreeMap<(usize, usize), f64>>,
    skill_effect_queue: RefCell<Vec<ExternalEffect>>,
    rank_of: RefCell<Vec<usize>>,
    order: RefCell<Vec<usize>>,
    prev_rank_of: RefCell<Vec<usize>>,
    previous_positions: RefCell<Option<Vec<f64>>>,
    pacemaker_idx: Cell<Option<usize>>,
    pacemaker_valid: Cell<bool>,
    // Experimental: eligible leader section trigger; server semantics unverified.
    section_pacemaker: Cell<Option<(usize, usize)>>,
    // v17: overall race leader recorded at the previous section refresh
    // (None for the start-of-race selection).
    refresh_overall: Cell<Option<usize>>,
    refresh_overall2: Cell<Option<usize>>,
    handover_start: Cell<Option<usize>>,
}

impl RaceField {
    pub fn new(mut solvers: Vec<RaceSolver>) -> RaceField {
        let n = solvers.len();
        for (i, s) in solvers.iter_mut().enumerate() {
            s.field_index = Some(i);
        }
        let strategies = solvers.iter().map(|s| s.horse.strategy).collect();
        let teams = solvers.iter().map(|s| s.race_context.team_id).collect();
        let characters = solvers.iter().map(|s| s.race_context.chara_id).collect();
        let retail_block = solvers.first().and_then(|s| s.retail.as_ref().map(|r|r.config.lane.block));
        let initial: BTreeSet<_> = solvers.iter_mut().enumerate().flat_map(|(i, s)| {
            std::mem::take(&mut s.initial_ability_activations).into_iter()
                .filter(|&(kind, value)| is_advantage_ability(kind, value)).map(move |(kind, _)| (i, kind))
        }).collect();
        assert!(solvers.iter().all(|s|s.retail.is_some()==retail_block.is_some()), "Mixed retail field profiles");
        let field = RaceField {
            retail_block,
            solvers: solvers.into_iter().map(RefCell::new).collect(),
            n,
            player_horse_index: 0,
            strategies,
            teams, characters, heal_history: RefCell::new(Vec::new()),
            advantage_activations: RefCell::new(initial),
            previous_advantage_activations: RefCell::new(BTreeSet::new()),
            overtake_cooldowns: RefCell::new(BTreeMap::new()),
            skill_effect_queue: RefCell::new(Vec::new()),
            rank_of: RefCell::new(vec![0; n]),
            order: RefCell::new((0..n).collect()),
            prev_rank_of: RefCell::new(vec![0; n]),
            previous_positions: RefCell::new(None),
            pacemaker_idx: Cell::new(None),
            pacemaker_valid: Cell::new(false),
            section_pacemaker: Cell::new(None),
            refresh_overall: Cell::new(None),
            refresh_overall2: Cell::new(None),
            handover_start: Cell::new(None),
        };
        field.update();
        field
    }

    /// Recompute live order/rank from each solver's current `pos`. Call
    /// once after every horse has been stepped for this tick.
    pub fn update(&self) {
        let mut prev = self.prev_rank_of.borrow_mut();
        let rank = self.rank_of.borrow();
        prev.copy_from_slice(&rank);
        drop(rank);
        self.pacemaker_valid.set(false);

        let positions: Vec<f64> = self.solvers.iter().map(|s| s.borrow().pos).collect();
        let mut idx: Vec<usize> = (0..self.n).collect();
        idx.sort_by(|&a, &b| positions[b].partial_cmp(&positions[a]).unwrap());
        let mut rank_of = vec![0usize; self.n];
        for (rank, &i) in idx.iter().enumerate() {
            rank_of[i] = rank + 1;
        }
        // v27: order-band "continue" conditions track every rank pass, not only while a
        // skill that uses them is being checked.
        for (i, &r) in rank_of.iter().enumerate() {
            if let Ok(s) = self.solvers[i].try_borrow() {
                crate::activation_conditions::track_order_bands(&s, r, self.n);
            }
        }
        *self.rank_of.borrow_mut() = rank_of;
        // v9: the section pacemaker is chosen from the preceding rank pass (live positions).
        let stale_order = std::mem::replace(&mut *self.order.borrow_mut(), idx);
        if let Some(previous) = self.previous_positions.borrow().as_ref() {
            for i in 0..self.n {
                let distance = self.solvers[i].borrow().course.distance;
                if previous[i] >= distance { continue; }
                let improved = prev[i] > self.rank_of.borrow()[i];
                let mut event = improved;
                if event {
                    let target = self.order.borrow()[self.rank_of.borrow()[i]];
                    let now = self.solvers[i].borrow().accumulatetime.get();
                    let mut cooldowns = self.overtake_cooldowns.borrow_mut();
                    event = now >= *cooldowns.get(&(i, target)).unwrap_or(&-1.0);
                    if event { cooldowns.insert((i, target), now + 6.0); }
                }
                let mut solver=self.solvers[i].borrow_mut();
                if event {solver.overtake_target_no_order_up_time=0.0;}
                if prev[i]<self.rank_of.borrow()[i] {solver.overtake_target_no_order_down_time=0.0;}
                solver.record_overtakes(usize::from(event));
            }
        }
        *self.previous_positions.borrow_mut() = Some(positions);
        if self.retail_block.is_some() {
            if let Some(leader) = Self::leading_eligible_in(&stale_order, &self.strategies) {
                // Front-less pacemaker handover (no Front runner present; doc 13.173).
                let leader = if self.strategies.iter().any(|s| matches!(s, Strategy::Nige | Strategy::Oonige)) {
                    leader
                } else {
                    let section = crate::retail_physics::target_speed_indices(
                        self.solvers[stale_order[0]].borrow().pos as f32,
                        self.solvers[leader].borrow().course.distance as f32).0;
                    let chosen = self.frontless_pacemaker(&stale_order, leader, section);
                    self.frontless_keep(&stale_order, leader, chosen, section)
                };
                let solver = self.solvers[leader].borrow();
                // v21: the refresh is keyed on the overall leader's course section
                // (preceding rank pass), not the leading Front-group runner's.
                // Corpus (501 races): 8 better / 0 worse, first divergence later in 7.
                let section = crate::retail_physics::target_speed_indices(
                    self.solvers[stale_order[0]].borrow().pos as f32,
                    solver.course.distance as f32).0;
                // Inferred Front/Pace handover; isolated development candidate.
                let previous = self.section_pacemaker.get();
                let front = matches!(solver.horse.strategy, Strategy::Nige | Strategy::Oonige);
                let is_front = |i: usize| matches!(self.solvers[i].borrow().horse.strategy, Strategy::Nige | Strategy::Oonige);
                // v15: no pacemaker refresh (or undo) once the leader has reached
                // PositionKeepEndSection; the incumbent keeps serving pace-mode exits.
                let end_section = solver.retail.as_ref().map_or(f32::INFINITY, |r| r.config.pace.end_section());
                if previous.is_some() && section as f32 >= end_section {
                } else if previous.map(|(_, old)| old) != Some(section) {
                    let overall = stale_order[0];
                    let previous_overall = self.refresh_overall
                        .replace(if section >= 1 { Some(overall) } else { None });
                    self.refresh_overall2.set(previous_overall);
                    // v6: Senkou lead only matters at opening-section refreshes (0-3).
                    let senkou_leads = front
                        && self.solvers[overall].borrow().horse.strategy == Strategy::Senkou;
                    // v17: the Senkou takes over only when it was also the overall
                    // leader at the previous refresh; otherwise the incumbent is held
                    // (no Front re-selection while a Senkou leads).
                    let pace_challenge = section < 4 && senkou_leads && previous_overall.is_some_and(|i| self.solvers[i].borrow().horse.strategy == Strategy::Senkou);
                    let selected = match previous {
                        Some(_) if pace_challenge => overall,
                        Some((prev, _)) if senkou_leads => prev,
                        _ => leader,
                    };
                    self.section_pacemaker.set(Some((selected, section)));
                } else if let Some((selected, _)) = previous {
                    // Restore only after the preceding rank pass puts the Front ahead.
                    if front && !is_front(selected) && stale_order.iter().position(|&i| i == leader) < stale_order.iter().position(|&i| i == selected) {
                        // v17: undo to the leading Front (v6 restored the pre-challenge pacer).
                        self.section_pacemaker.set(Some((leader, section)));
                    }
                }
            }
        }

    }

    /// Advance all positions before lateral checks, and all lanes before
    /// proximity/competition checks. Preserve each runner's partial gate dt.
    pub fn step_all(&self, dt: f64, active_indices: &[usize]) {
        if self.retail_block.is_some() {
            self.step_all_retail(dt as f32, active_indices);
            return;
        }
        let overtake_times=self.next_overtake_times(dt);
        let advanced: Vec<(usize, f64)> = active_indices.iter().map(|&i| {
            let effective_dt = self.solvers[i].borrow_mut().step_forward(dt, Some(self));
            (i, effective_dt)
        }).collect();
        for &i in active_indices {
            let mut solver=self.solvers[i].borrow_mut();
            (solver.overtake_target_time,solver.overtake_target_no_order_up_time,
             solver.overtake_target_no_order_down_time)=overtake_times[i];
        }
        self.update();
        for &(i, effective_dt) in &advanced {
            if effective_dt > 0.0 {
                self.solvers[i].borrow_mut().step_lateral(effective_dt, Some(self));
            }
        }
        for &(i, effective_dt) in &advanced {
            if effective_dt > 0.0 {
                self.solvers[i].borrow_mut().step_field_conditions(effective_dt, Some(self));
            }
        }
        self.flush_skill_effects();
        self.cache_skill_activations();
    }

    pub(super) fn retail_overtake_times_for(&self, i: usize, dt: f32) -> (f64, f64, f64) {
        let ranks = self.rank_of.borrow();
        let s = self.solvers[i].borrow();
        // Each list excludes its owner. Earlier runners have updated their
        // real passing list this tick; later runners still expose the prior list.
        let targeted = self.solvers.iter().enumerate().any(|(j, cell)| {
            if i == j || ranks[j] <= ranks[i] { return false; }
            let other = cell.borrow();
            let target_index = if i < j { i } else { i - 1 };
            other.retail.as_ref().unwrap().lane.targets.contains(&target_index)
        });
        let has_targets = !s.retail.as_ref().unwrap().lane.targets.is_empty();
        let advance = |value: f64, enabled: bool| if enabled { (value as f32 + dt) as f64 } else { 0. };
        (advance(s.overtake_target_time, targeted),
         advance(s.overtake_target_no_order_up_time, has_targets),
         advance(s.overtake_target_no_order_down_time, has_targets))
    }

    fn next_overtake_times(&self, dt:f64)->Vec<(f64,f64,f64)> {
        let blockers:Vec<_>=self.solvers.iter().enumerate().map(|(i,s)| {
            let s=s.borrow();self.front_blocker(i,s.pos,s.lane).map(|b|b.0)
        }).collect();
        let targets:Vec<Vec<usize>>=self.solvers.iter().enumerate().map(|(i,owner)| {
            let owner=owner.borrow();
            self.solvers.iter().enumerate().filter_map(|(j,other)| {
                if i==j {return None;}
                let other=other.borrow();
                if other.pos>=owner.course.distance {return None;}
                let owner_speed=owner.retail.as_ref().map_or(owner.current_speed,|r|r.observed_speed as f64);
                let other_speed=other.retail.as_ref().map_or(other.current_speed,|r|r.observed_speed as f64);
                (blockers[i]==Some(j) || crate::overtaking::is_target(other.pos-owner.pos,owner_speed-other_speed,
                    owner.target_speed,other.target_speed,other_speed,
                    blockers[i]==Some(j),blockers[j].is_some())).then_some(j)
            }).collect()
        }).collect();
        let ranks=self.rank_of.borrow();
        self.solvers.iter().enumerate().map(|(i,s)| {
            let s=s.borrow();
            let targeted=targets.iter().enumerate().any(|(j,row)|ranks[j]>ranks[i] && row.contains(&i));
            let advance=|value:f64,enabled:bool|if enabled {(value as f32+dt as f32) as f64} else {0.0};
            (advance(s.overtake_target_time,targeted),
             advance(s.overtake_target_no_order_up_time,!targets[i].is_empty()),
             advance(s.overtake_target_no_order_down_time,!targets[i].is_empty()))
        }).collect()
    }

    pub fn has_character(&self, id: i64) -> bool { self.characters.contains(&id) }

    pub fn record_heal(&self, i: usize) {
        self.heal_history.borrow_mut().push(i);
    }

    pub fn record_ability_activation(&self, i: usize, kind: i32, value: f64) {
        if is_advantage_ability(kind, value) {
            self.advantage_activations.borrow_mut().insert((i, kind));
        }
    }

    fn cache_skill_activations(&self) {
        // Global CachePrevActivateSkillHorseDict runs after the entire field.
        std::mem::swap(&mut *self.previous_advantage_activations.borrow_mut(),
                      &mut *self.advantage_activations.borrow_mut());
        self.advantage_activations.borrow_mut().clear();
    }

    pub fn other_advantage_active(&self, owner: &RaceSolver, kind: i32) -> bool {
        let index = owner.field_index.unwrap();
        self.previous_advantage_activations.borrow().iter().any(|&(i, ability)| i != index
            && ability == kind
            && !(self.teams[index] != 0 && self.teams[index] == self.teams[i]))
    }

    pub fn viewer_remaining(&self, owner: &RaceSolver) -> f64 {
        let pos = if owner.field_index == Some(self.player_horse_index) { owner.pos }
            else { self.solvers[self.player_horse_index].borrow().pos };
        owner.course.distance - pos.floor()
    }

    pub fn visible(&self, owner: &RaceSolver, other: &RaceSolver) -> bool {
        let gap = other.pos - owner.pos;
        let distance = owner.visible_distance();
        gap >= 0.0 && gap <= distance && (other.lane - owner.lane).abs() <=
            (gap / distance * 11.5 + 2.0) * HORSE_LANE / 2.0
    }

    pub(crate) fn skill_targets(&self, owner: &mut RaceSolver, effect: &SkillEffect, _modifier: f64) -> Vec<usize> {
        let index = owner.field_index.unwrap();
        let rank = self.order_of(index);
        let order = self.order.borrow();
        let ahead: Vec<usize> = order[..rank-1].iter().rev().copied().collect();
        let behind: Vec<usize> = order[rank..].to_vec();
        let value = effect.target_value;
        let limit = value.max(0) as usize;
        let candidates: Vec<usize> = match effect.target {
            1 => vec![index],
            2 => order.clone(),
            // v72 retail: the owner's visible list cached by this tick's lane collect (before
            // effects expire), as the visiblehorse condition uses. Official survey-3515 (c10810):
            // a -5 m vision debuff expiring this tick still limited 201441's targets to 3 and 5.
            4 if owner.retail.is_some() => {
                let cached = &owner.retail.as_ref().unwrap().lane.visible;
                order.iter().copied().filter(|&i| i != index
                    && cached.contains(&(if i < index { i } else { i - 1 }))).take(limit).collect()
            }
            4 => order.iter().copied().filter(|&i| i != index && self.visible(owner, &self.solvers[i].borrow())).take(limit).collect(),
            9 => ahead.into_iter().take(limit).collect(),
            10 => behind.into_iter().take(limit).collect(),
            19 | 20 => {
                let mut pool: Vec<usize> = (if effect.target == 19 { ahead } else { behind }).into_iter()
                    .filter(|&i| self.solvers[i].borrow().is_kakari).collect();
                let mut selected = Vec::new();
                for _ in 0..limit.min(pool.len()) {
                    let at = owner.rng_uniform(pool.len() as f64) as usize;
                    selected.push(pool.remove(at));
                }
                selected
            },
            18 | 21 => order.iter().copied().filter(|&i| i != index && Strategy::from_i32(value).is_ok_and(|strategy| strategy_matches(self.strategies[i], strategy))
                && (effect.target != 21 || self.solvers[i].borrow().is_kakari)).collect(),
            22 => order.iter().copied().filter(|&i| self.characters[i] == value as i64).collect(),
            23 => {
                let mut chosen = Vec::new();
                for &i in self.heal_history.borrow().iter().rev() {
                    if chosen.len() >= limit { break; }
                    if !chosen.contains(&i) { chosen.push(i); }
                }
                chosen
            }
            11 => order.iter().copied().filter(|&i| self.teams[index] != 0 && self.teams[index] == self.teams[i]).collect(),
            _ => Vec::new(),
        };
        let debuff = effect.target_is_debuff;
        candidates.into_iter().filter(|&i| {
            if !debuff {return true;}
            let immune = if i == index {owner.is_debuff_immune()} else {self.solvers[i].borrow().is_debuff_immune()};
            !immune && !(i != index && self.teams[index] != 0 && self.teams[i] == self.teams[index])
        }).collect()
    }

    pub(crate) fn queue_skill_effect(&self, owner: &mut RaceSolver, skill_id: &str, rarity: SkillRarity, mut effect: SkillEffect, modifier: f64, duration: f64, selected_targets: Option<Vec<usize>>) -> Vec<usize> {
        let targets = selected_targets.unwrap_or_else(|| self.skill_targets(owner, &effect, modifier));
        effect.modifier = modifier;
        effect.value_usage = 1;
        effect.target = 1;
        effect.additional_activate = 0;
        for &target in &targets {
            if owner.retail.is_some() {
                if Some(target)==owner.field_index {
                    owner.apply_external_effect(skill_id,rarity,effect,duration,true,self);
                } else {
                    self.solvers[target].borrow_mut().apply_external_effect(skill_id,rarity,effect,duration,false,self);
                }
            } else {
                self.skill_effect_queue.borrow_mut().push(ExternalEffect { source: owner.field_index.unwrap(),
                    target, skill_id: skill_id.to_string(), rarity, effect, duration });
            }
        }
        targets
    }

    fn flush_skill_effects(&self) {
        let queue = std::mem::take(&mut *self.skill_effect_queue.borrow_mut());
        for event in queue {
            self.solvers[event.target].borrow_mut().apply_external_effect(&event.skill_id, event.rarity,
                event.effect, event.duration, event.source == event.target, self);
        }
    }

    pub fn order_of(&self, i: usize) -> usize {
        self.rank_of.borrow()[i]
    }

    pub fn previous_order_of(&self, i: usize) -> usize {
        self.prev_rank_of.borrow()[i]
    }

    /// `my_pos`/`my_lane` below are the querying horse `i`'s own current
    /// values, passed in by the caller instead of re-borrowed here -- `i`
    /// may be the solver currently mid-`step()` and thus already mutably
    /// borrowed via its `RefCell`, so this must never index `self.solvers[i]`.
    pub fn distance_to_leader(&self, i: usize, my_pos: f64) -> f64 {
        let leader = self.order.borrow()[0];
        let leader_pos = if leader == i { my_pos } else { self.solvers[leader].borrow().pos };
        (leader_pos - my_pos).max(0.0)
    }

    /// Ranked endpoints for skill triggers. Avoid re-borrowing the caller
    /// and preserve the live signed gaps between rank updates.
    pub fn ranked_distance_bounds(&self, i: usize, my_pos: f64) -> (f64, f64) {
        let order = self.order.borrow();
        let position = |index: usize| if index == i { my_pos } else { self.solvers[index].borrow().pos };
        (position(order[0]), position(order[self.n - 1]))
    }

    pub fn distance_to_ahead(&self, i: usize, my_pos: f64) -> Option<f64> {
        let r = self.rank_of.borrow()[i];
        if r <= 1 {
            return None;
        }
        let ahead = self.order.borrow()[r - 2];
        Some((self.solvers[ahead].borrow().pos - my_pos).max(0.0))
    }

    pub fn distance_to_behind(&self, i: usize, my_pos: f64) -> Option<f64> {
        let r = self.rank_of.borrow()[i];
        if r >= self.n {
            return None;
        }
        let behind = self.order.borrow()[r];
        Some((my_pos - self.solvers[behind].borrow().pos).max(0.0))
    }

    pub fn lane_gap_to_ahead(&self, i: usize, my_lane: f64) -> Option<f64> {
        let r = self.rank_of.borrow()[i];
        if r <= 1 {
            return None;
        }
        let ahead = self.order.borrow()[r - 2];
        Some(self.solvers[ahead].borrow().lane - my_lane)
    }

    pub fn lane_gap_to_behind(&self, i: usize, my_lane: f64) -> Option<f64> {
        let r = self.rank_of.borrow()[i];
        if r >= self.n {
            return None;
        }
        let behind = self.order.borrow()[r];
        Some(my_lane - self.solvers[behind].borrow().lane)
    }

    /// True on the tick this solver's rank improved (took over at least one
    /// other horse) versus the previous tick.
    pub fn did_overtake(&self, i: usize) -> bool {
        self.rank_of.borrow()[i] < self.prev_rank_of.borrow()[i]
    }

    /// Doc: front block = 0 < distGap < 2m AND |laneGap| <= a tolerance that
    /// shrinks from 0.75 horse-lanes at 0m to 0.3 at 2m. Lowest
    /// distance-gap qualifying horse wins. Returns (other_index, dist_gap).
    pub fn front_blocker(&self, i: usize, pos: f64, lane: f64) -> Option<(usize, f64)> {
        if let Some(parameters) = self.retail_block {
            use crate::retail_lane::{Row, front_index};
            let peers:Vec<_> = (0..self.n).filter(|&j|j!=i).collect();
            let rows:Vec<_> = peers.iter().map(|&j| {
                let s=self.solvers[j].borrow();Row{position:s.pos as f32,lane:s.lane as f32}
            }).collect();
            return front_index(Row{position:pos as f32,lane:lane as f32},&rows,parameters)
                .map(|at|(peers[at],(rows[at].position-pos as f32) as f64));
        }
        let mut best: Option<(usize, f64)> = None;
        for j in 0..self.n {
            if j == i {
                continue;
            }
            let other = self.solvers[j].borrow();
            if other.pos >= other.course.distance {
                continue;
            }
            let dist_gap = other.pos - pos;
            if !(dist_gap > 0.0 && dist_gap < 2.0) {
                continue;
            }
            let lane_gap = (other.lane - lane).abs();
            let tolerance = (1.0 - 0.6 * dist_gap / 2.0) * 0.75 * HORSE_LANE;
            if lane_gap <= tolerance && best.map_or(true, |(_, bd)| dist_gap < bd) {
                best = Some((j, dist_gap));
            }
        }
        best
    }

    /// Doc: side block = |distGap| < 1.05m AND |laneGap| < 2 horse-lanes.
    /// Returns (inner_blocked, outer_blocked) -- inner = lower lane (toward
    /// the rail), outer = higher lane.
    pub fn side_room(&self, i: usize, pos: f64, lane: f64) -> (bool, bool) {
        let mut inner_blocked = false;
        let mut outer_blocked = false;
        if let Some(block) = self.retail_block {
            // v6: retail float32 side-block test (SideBlockDistanceGap/SideBlockLaneGap, strict <)
            // used by the blocked-time counters, instead of the doc's float64 2/18 lanes.
            for j in 0..self.n {
                if j == i { continue; }
                let other = self.solvers[j].borrow();
                if other.pos >= other.course.distance { continue; }
                if !((other.pos as f32 - pos as f32).abs() < block.side_distance) { continue; }
                let gap = other.lane as f32 - lane as f32;
                if !(gap.abs() < block.side_lane) { continue; }
                if gap < 0.0 { inner_blocked = true; } else if gap > 0.0 { outer_blocked = true; }
            }
            return (inner_blocked, outer_blocked);
        }
        let lane_tol = 2.0 * HORSE_LANE;
        for j in 0..self.n {
            if j == i {
                continue;
            }
            let other = self.solvers[j].borrow();
            if other.pos >= other.course.distance {
                continue;
            }
            let d = other.pos - pos;
            if !(-1.05 < d && d < 1.05) {
                continue;
            }
            let lane_gap = other.lane - lane;
            if !(-lane_tol < lane_gap && lane_gap < lane_tol) {
                continue;
            }
            if lane_gap < 0.0 {
                inner_blocked = true;
                if outer_blocked {
                    break;
                }
            } else if lane_gap > 0.0 {
                outer_blocked = true;
                if inner_blocked {
                    break;
                }
            }
        }
        (inner_blocked, outer_blocked)
    }

    /// Doc: true if ALL THREE of Out/Front/Behind each find another horse
    /// (the same horse may satisfy more than one direction).
    pub fn is_surrounded(&self, i: usize, pos: f64, lane: f64) -> bool {
        let mut out = false;
        let mut front = false;
        let mut behind = false;
        for j in 0..self.n {
            if j == i {
                continue;
            }
            let other = self.solvers[j].borrow();
            // UpdateAroundHorsesParam stores single-precision SUBSS gaps.
            let dist_gap = (other.pos - pos) as f32 as f64;
            let lane_gap = (other.lane - lane) as f32 as f64;
            if dist_gap.abs() <= 1.5 && lane_gap >= 0.0 && lane_gap <= 0.16666_f32 as f64 {
                out = true;
            }
            if dist_gap > 0.0 && dist_gap <= 3.0 && lane_gap.abs() <= 0.08333_f32 as f64 {
                front = true;
            }
            if dist_gap >= -3.0 && dist_gap < 0.0 && lane_gap.abs() <= 0.08333_f32 as f64 {
                behind = true;
            }
            if out && front && behind {
                break;
            }
        }
        out && front && behind
    }

    pub fn is_solo_front_runner(&self, i: usize) -> bool {
        for j in 0..self.n {
            if j == i {
                continue;
            }
            if strategy_matches(self.strategies[j], Strategy::Nige) {
                return false;
            }
        }
        true
    }

    pub fn ordinary_front_runner_count(&self) -> usize {
        self.strategies.iter().filter(|&&s| s == Strategy::Nige).count()
    }

    pub fn later_style_ahead(&self, i: usize, strategy: Strategy) -> bool {
        let base = |s: Strategy| if s == Strategy::Oonige { 1 } else { s as usize };
        self.order.borrow().iter().take(self.order_of(i) - 1)
            .any(|&j| base(self.strategies[j]) > base(strategy))
    }

    pub fn pacemaker_is_later_style_ahead(&self, i: usize, strategy: Strategy) -> bool {
        let base = |s: Strategy| if s == Strategy::Oonige { 1 } else { s as usize };
        self.pacemaker().is_some_and(|j| self.order_of(j) < self.order_of(i)
            && base(self.strategies[j]) > base(strategy))
    }

    /// Doc's pre-1.5-anniversary pacemaker rule: the 1st-place horse among
    /// the field's most-forward strategy group that's actually present.
    /// Depends only on `order` (refreshed once per tick by `update`) and
    /// solvers' fixed strategies, so it is computed once per tick and
    /// cached rather than recomputed per asking horse.
    pub fn pacemaker(&self) -> Option<usize> {
        if let Some((leader, _)) = self.section_pacemaker.get() {
            return Some(leader);
        }
        if self.pacemaker_valid.get() {
            return self.pacemaker_idx.get();
        }
        let result = self.leading_eligible();
        self.pacemaker_idx.set(result);
        self.pacemaker_valid.set(true);
        result
    }

    /// Front-less handover (development labels from pacemaker-dirt-01/-02,
    /// pacemaker-validation-02/-04, frontless-handover-dev-03 and shooting-star-type22-01;
    /// doc 13.169-13.179):
    /// - From section 1, if the overall leader is >= 10 m ahead of `leader` (the
    ///   leading most-forward-style runner), pace goes to the leading runner of the
    ///   most forward style among runners within 10 m of the overall leader.
    /// - From section 2, while an End Closer leads, or until the section after the
    ///   handover began, an existing handover to a later style is held. Otherwise pace
    ///   goes to the frontmost non-End-Closer when an End Closer leads, unless `leader`
    ///   led overall at the previous refresh; from section 3 the previous refresh must
    ///   also have been led by an End Closer, or (section 3 only) `leader` led overall
    ///   two refreshes back.
    fn frontless_pacemaker(&self, stale_order: &[usize], leader: usize, section: usize) -> usize {
        if section < 1 {
            return leader;
        }
        let pos = |i: usize| self.solvers[i].borrow().pos;
        let lead = pos(stale_order[0]);
        let ten = if lead - pos(leader) >= 10.0 {
            let near: Vec<usize> = stale_order.iter().copied().filter(|&i| lead - pos(i) < 10.0).collect();
            Self::leading_eligible_in(&near, &self.strategies).unwrap_or(leader)
        } else {
            leader
        };
        // Section 1: unless a most-forward-style runner leads the race, the section-0
        // pacemaker is kept (doc 13.179).
        if section == 1 && ten == leader && self.strategies[stale_order[0]] != self.strategies[leader] {
            if let Some((p, _)) = self.section_pacemaker.get() {
                return p;
            }
        }
        if section < 2 || ten != leader {
            return ten;
        }
        let handed = self.section_pacemaker.get().map(|(p, _)| p)
            .filter(|&p| self.strategies[p] != self.strategies[leader]);
        let closer_leads = self.strategies[stale_order[0]] == Strategy::Oikomi;
        let hold = self.handover_start.get().map_or(false, |h| section <= h + 1);
        if !(closer_leads || (hold && handed.is_some())) {
            return ten;
        }
        if let Some(p) = handed {
            return p;
        }
        let previous_overall = self.refresh_overall.get();
        if previous_overall != Some(leader)
            && (section == 2
                || previous_overall.map_or(false, |o| self.strategies[o] == Strategy::Oikomi)
                || (section == 3 && self.refresh_overall2.get() == Some(leader))) {
            self.handover_start.set(Some(section));
            return stale_order.iter().copied().find(|&i| self.strategies[i] != Strategy::Oikomi).unwrap_or(ten);
        }
        ten
    }

    /// Among several most-forward-style runners, a standard choice (`chosen == leader`)
    /// keeps the previous same-style pacemaker unless a most-forward-style runner leads
    /// the race; at section 2 the choice is made anyway when no such runner led the
    /// section-1 refresh (doc 13.179).
    fn frontless_keep(&self, stale_order: &[usize], leader: usize, chosen: usize, section: usize) -> usize {
        let deferred = section == 2
            && self.refresh_overall.get().map_or(true, |o| self.strategies[o] != self.strategies[leader]);
        match self.section_pacemaker.get().map(|(p, _)| p) {
            Some(p) if chosen == leader && p != leader && self.strategies[p] == self.strategies[leader]
                && self.strategies[stale_order[0]] != self.strategies[leader] && !deferred => p,
            _ => chosen,
        }
    }

    fn leading_eligible(&self) -> Option<usize> {
        Self::leading_eligible_in(&self.order.borrow(), &self.strategies)
    }

    fn leading_eligible_in(order: &[usize], strategies: &[Strategy]) -> Option<usize> {
        let mut result = None;
        for group in PACEMAKER_GROUPS {
            for &i in order.iter() {
                if group.contains(&strategies[i]) {
                    result = Some(i);
                    break;
                }
            }
            if result.is_some() {
                break;
            }
        }
        result
    }

    pub fn gap_to_pacemaker(&self, i: usize, my_pos: f64) -> f64 {
        match self.pacemaker() {
            Some(pm) if pm != i => self.solvers[pm].borrow().pos - my_pos,
            _ => 0.0,
        }
    }

    /// Global _CheckNearHorse: inclusive extracted distance/lane limits.
    pub fn near_count(&self, i: usize, pos: f64, lane: f64) -> i32 {
        let lane_tol = 0.16666_f32 as f64;
        let mut count = 0;
        for j in 0..self.n {
            if j == i {
                continue;
            }
            let other = self.solvers[j].borrow();
            if (other.pos - pos).abs() <= 3.0 && (other.lane - lane).abs() <= lane_tol {
                count += 1;
            }
        }
        count
    }

    /// Same inclusive near corridor, restricted to positive distance gaps.
    pub fn near_infront_count(&self, i: usize, pos: f64, lane: f64) -> i32 {
        let lane_tol = 0.16666_f32 as f64;
        let mut count = 0;
        for j in 0..self.n {
            if j == i {
                continue;
            }
            let other = self.solvers[j].borrow();
            let dist_gap = other.pos - pos;
            if dist_gap > 0.0 && dist_gap <= 3.0 && (other.lane - lane).abs() <= lane_tol {
                count += 1;
            }
        }
        count
    }

    /// Doc: temptation_count_behind -- the number of horses ranked behind
    /// `i` (by finishing-position rank, not physical distance) that are
    /// currently rushing (kakari).
    pub fn is_opponent(&self, owner: usize, other: usize) -> bool {
        owner != other && !(self.teams[owner] != 0 && self.teams[owner] == self.teams[other])
    }

    pub fn temptation_count_behind(&self, i: usize, opponents_only: bool) -> i32 {
        let r = self.rank_of.borrow()[i];
        let order = self.order.borrow();
        order[r..].iter().filter(|&&j| (!opponents_only || self.is_opponent(i, j))
            && self.solvers[j].borrow().is_kakari).count() as i32
    }

    /// Doc: temptation_count_infront -- the number of horses ranked ahead
    /// of `i` that are currently rushing (kakari).
    pub fn temptation_count_infront(&self, i: usize, opponents_only: bool) -> i32 {
        let r = self.rank_of.borrow()[i];
        let order = self.order.borrow();
        order[..r - 1].iter().filter(|&&j| (!opponents_only || self.is_opponent(i, j))
            && self.solvers[j].borrow().is_kakari).count() as i32
    }

    /// Doc: running_style_temptation_count_{nige,senko,sashi,oikomi} -- the
    /// number of horses in the field (the querying horse counts too) with
    /// the given running strategy that are currently rushing. `self_idx`/
    /// `self_is_kakari` come from the caller directly (rather than
    /// re-borrowing `self.solvers[self_idx]`) since this is called from a
    /// condition check while that very solver is already mutably borrowed
    /// mid-`step()`.
    pub fn running_style_temptation_count(&self, strategy: Strategy, self_idx: usize, self_is_kakari: bool, opponents_only: bool) -> i32 {
        let others = (0..self.n)
            .filter(|&j| j != self_idx && (!opponents_only || self.is_opponent(self_idx, j))
                && strategy_matches(self.strategies[j], strategy) && self.solvers[j].borrow().is_kakari)
            .count() as i32;
        let self_counts = !opponents_only && strategy_matches(self.strategies[self_idx], strategy) && self_is_kakari;
        others + self_counts as i32
    }
}
