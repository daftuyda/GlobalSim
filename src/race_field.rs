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

#[derive(Default)]
struct RankBuffers {
    positions: Vec<f64>,
    indices: Vec<usize>,
    ranks: Vec<usize>,
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
    retail_tick: RefCell<retail_impl::TickBuffers>,
    rank_buffers: RefCell<RankBuffers>,
    pub solvers: Vec<RefCell<RaceSolver>>,
    pub n: usize,
    order_band_thresholds: [f64; 5],
    pub player_horse_index: usize,
    /// Each solver's fixed strategy, mirrored outside its `RefCell` so
    /// `pacemaker()` can scan every horse's strategy -- including the one
    /// currently mid-`step()` and thus already mutably borrowed -- without
    /// re-entering that `RefCell`.
    strategies: Vec<Strategy>,
    teams: Vec<i64>,
    characters: Vec<i64>,
    pub heal_history: RefCell<Vec<usize>>,
    cached_heal_count: Cell<usize>,
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
    // Global section-based handover state, validated against captured races.
    section_pacemaker: Cell<Option<(usize, usize)>>,
    pm_count: Cell<usize>,
    pm_handed: Cell<bool>,
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
            retail_tick: RefCell::new(retail_impl::TickBuffers::default()),
            rank_buffers: RefCell::new(RankBuffers::default()),
            solvers: solvers.into_iter().map(RefCell::new).collect(),
            n,
            order_band_thresholds: [0.2, 0.4, 0.5, 0.7, 0.8].map(|rate| (rate * n as f64).round()),
            player_horse_index: 0,
            strategies,
            teams, characters, heal_history: RefCell::new(Vec::new()),
            cached_heal_count: Cell::new(0),
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
            pm_count: Cell::new(0),
            pm_handed: Cell::new(false),
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

        let mut buffers = self.rank_buffers.borrow_mut();
        let RankBuffers { positions, indices: idx, ranks: rank_of } = &mut *buffers;
        positions.clear();
        positions.extend(self.solvers.iter().map(|s| s.borrow().pos));
        idx.clear();
        idx.extend(0..self.n);
        idx.sort_by(|&a, &b| positions[b].partial_cmp(&positions[a]).unwrap());
        rank_of.clear();
        rank_of.resize(self.n, 0);
        for (rank, &i) in idx.iter().enumerate() {
            rank_of[i] = rank + 1;
        }
        // v27: order-band "continue" conditions track every rank pass, not only while a
        // skill that uses them is being checked.
        for (i, &r) in rank_of.iter().enumerate() {
            if let Ok(s) = self.solvers[i].try_borrow() {
                // Continuity is updated from the existing rank before the new
                // order is published. Skills read that stored history.
                let tracked_rank = if s.retail.is_some() { prev[i] } else { r };
                crate::activation_conditions::track_order_bands(&s, tracked_rank, &self.order_band_thresholds);
            }
        }
        std::mem::swap(&mut *self.rank_of.borrow_mut(), rank_of);
        // v9: the section pacemaker is chosen from the preceding rank pass (live positions).
        let stale_order = std::mem::replace(&mut *self.order.borrow_mut(), std::mem::take(idx));
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
        // Keep the previous positions and ordering available until all
        // observers finish, then recycle those buffers for the next pass.
        *positions = self.previous_positions.borrow_mut()
            .replace(std::mem::take(positions)).unwrap_or_default();
        if self.retail_block.is_some() {
            self.update_pacemaker(&stale_order);
        }
        *idx = stale_order;

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
        self.cached_heal_count.set(self.heal_history.borrow().len());
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
                // Healer history is published after the whole field's skill
                // pass. A heal this tick cannot replace last tick's target.
                let history = self.heal_history.borrow();
                let count = if owner.retail.is_some() { self.cached_heal_count.get() } else { history.len() };
                for &i in history[..count].iter().rev() {
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
            use crate::retail_lane::{Row, front_index_iter};
            let rows = (0..self.n).filter(|&j| j != i).map(|j| {
                let s = self.solvers[j].borrow();
                Row { position: s.pos as f32, lane: s.lane as f32 }
            });
            return front_index_iter(Row { position: pos as f32, lane: lane as f32 }, rows, parameters)
                .map(|at| {
                    let j = if at < i { at } else { at + 1 };
                    (j, (self.solvers[j].borrow().pos as f32 - pos as f32) as f64)
                });
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

    /// Count non-forward-led section checks cumulatively until recalculation.
    /// A forward-led check alone does not clear the count. After a handover,
    /// retaking the lead restores the forward group immediately and clears it.
    /// Use the preceding rank pass and live positions, as in v70.
    fn update_pacemaker(&self, order: &[usize]) {
        // Runaway shares the Front Runner group; it does not outrank it.
        let style = |i: usize| self.strategies[i].base_running_style() as usize;
        let pos = |i: usize| self.solvers[i].borrow().pos as f32;
        let Some(overall) = order.iter().copied().find(|&i| {
            let h=self.solvers[i].borrow(); h.pos < h.course.distance
        }) else { return; };
        let distance=self.solvers[overall].borrow().course.distance as f32;
        let section=crate::retail_physics::target_speed_indices(pos(overall),distance).0;
        let previous=self.section_pacemaker.get();
        let (end, range, threshold) = {
            let solver = self.solvers[overall].borrow();
            let pace = &solver.retail.as_ref().unwrap().config.pace;
            (pace.end_section(), pace.pacemaker_forward_range, pace.pacemaker_not_forward_count)
        };
        // Section-based selection ends here, but an active position-keep
        // effect can still observe restoration of the forward group.
        // Once restored, late ticks need neither allocation nor style scans.
        if previous.is_some() && section as f32 >= end && !self.pm_handed.get() { return; }
        let active: Vec<_> = order.iter().copied().filter(|&i| {
            let h=self.solvers[i].borrow(); h.pos < h.course.distance
        }).collect();
        let forward=active.iter().map(|&i|style(i)).min().unwrap();
        let leader=*active.iter().find(|&&i|style(i)==forward).unwrap();
        let (mut selected, old_section)=previous.unwrap_or((leader,0));
        if previous.is_none() {self.section_pacemaker.set(Some((leader,0)));}
        let entering_section = section > old_section && (section as f32) < end;
        if entering_section {
            // A returning forward leader first refreshes the handed-over
            // group's incumbent at a section boundary. A returning Runaway
            // retains the incumbent through this pass. Restoration then
            // compares against that group's current leader on later ticks.
            if self.pm_handed.get()
                && self.strategies[leader] != Strategy::Oonige
                && active.iter().position(|&i|i==leader) < active.iter().position(|&i|i==selected) {
                selected=active.iter().copied().find(|&i|style(i)==style(selected)).unwrap_or(selected);
            }
            if style(overall)==forward && !self.pm_handed.get() {selected=leader;}
            // A detached forward group yields to the most-forward style
            // within range, preserving rank within that style. Front Runners
            // participate in this refresh once a handover has occurred.
            if (forward > 1 || self.pm_handed.get()) && pos(overall)-pos(leader)>=range {
                selected=*active.iter().filter(|&&i|pos(overall)-pos(i)<range)
                    .min_by_key(|&&i|style(i)).unwrap_or(&overall);
                // A detached-group refresh starts a new counting interval,
                // including when it selects the same incumbent. The current
                // section still contributes to the check below.
                self.pm_count.set(0);
            }
            if style(overall)!=forward {
                let count=self.pm_count.get()+1;
                self.pm_count.set(count);
                let below=count<threshold;
                if !below || self.pm_handed.get() {
                    if !below {self.pm_count.set(0);}
                    let candidate=if style(overall)<3 {overall} else {
                        active.iter().copied().find(|&i|style(i)<style(overall) && pos(overall)-pos(i)<=range).unwrap_or(overall)
                    };
                    if !below || (style(candidate)==forward && forward<=1) {
                        selected=candidate;
                        if style(candidate)!=forward {self.pm_count.set(0);self.pm_handed.set(true);}
                    }
                }
            }
            self.section_pacemaker.set(Some((selected,section)));
        }
        // Restoration runs between section-selection passes. If a forward
        // runner retakes the lead on a new section's tick, the following tick
        // restores it. This can also happen after section selection has ended.
        if !entering_section && (self.pm_handed.get() || self.pm_count.get()>=threshold)
            && (style(overall)==forward || (leader != selected
                && active.iter().position(|&i|i==leader) < active.iter().position(|&i|i==selected))) {
            self.section_pacemaker.set(Some((leader,section)));
            self.pm_count.set(0);self.pm_handed.set(false);
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
