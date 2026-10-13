//! Experimental float32 passing search and cached side permissions.
//! These are native ports of the reconstructed Python behavior, not proof
//! that the complete client movement state machine has been recovered.
use crate::retail_random::RandomSource;
use crate::retail_sort::comparison_sort;

#[derive(Clone, Copy, Debug, Default)]
pub struct Row {
    pub position: f32,
    pub lane: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Horse {
    pub point: Row,
    pub speed: f32,
    /// Live target speed including active current-speed receivers.
    pub target: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct TargetParameters {
    pub block: BlockParameters,
    pub force_in_threshold: f32,
    pub cooldown: f32,
    pub outward_normalize: f32,
    pub outward_coefficient: f32,
    pub outward_random: f32,
}

impl TargetParameters {
    pub fn from_value(value: &crate::json::Value) -> Result<Self, String> {
        let block = value
            .get("Block")
            .ok_or("Missing retail lane block parameters")?;
        let n = |value: &crate::json::Value, key: &str| -> Result<f32, String> {
            let result = value.get_f64(key)? as f32;
            if result.is_finite() {
                Ok(result)
            } else {
                Err(format!("Invalid lane parameter {key}"))
            }
        };
        let result = Self {
            block: BlockParameters {
                front_distance: n(block, "FrontBlockDistanceGap")?,
                front_lane: n(block, "FrontBlockLaneGap")?,
                side_distance: n(block, "SideBlockDistanceGap")?,
                side_lane: n(block, "SideBlockLaneGap")?,
            },
            force_in_threshold: n(value, "forceInMoveLaneThreshold")?,
            cooldown: n(value, "overTakeCoolDownTime")?,
            outward_normalize: n(value, "lastMoveOutLaneNormalizeVal")?,
            outward_coefficient: n(value, "lastMoveOutLaneCoef")?,
            outward_random: n(value, "lastMoveOutLaneAddRandom")?,
        };
        if result.block.front_distance <= 0.
            || result.outward_normalize <= 0.
            || result.outward_random < 0.
            || result.cooldown < 0.
        {
            return Err("Invalid retail lane parameter range".into());
        }
        Ok(result)
    }
}

pub struct TargetContext {
    pub lane_preference: u8,
    pub owner: Horse,
    pub phase: i32,
    pub max_lane: f32,
    pub vision: f32,
    pub first_move_point: f32,
    pub final_corner_start: Option<f32>,
    pub empty_in_move: bool,
    pub previous_target: f32,
    pub fixed_goal: f32,
    pub has_hp: bool,
    pub pace_down: bool,
}

/// Roster indices must remain stable for the lifetime of this state, even
/// after peers finish. `collect` runs before any runner moves; subsequent
/// calls deliberately also accept the separately sampled live peer values.
#[derive(Default)]
pub struct TargetState {
    pub targets: Vec<usize>,
    pub cached: Vec<Horse>,
    live_points: Vec<Row>,
    cached_points: Vec<Row>,
    pub visible: Vec<usize>,
    pub side: Option<SideRoom>,
    pub cooldown: f32,
    pub force_check: bool,
    pub force_in: bool,
    pub extra_outward: Option<f32>,
}

impl TargetState {
    pub fn collect(&mut self, c: &TargetContext, others: &[Horse], p: TargetParameters) {
        self.cached.clear();
        self.cached.extend_from_slice(others);
        self.side = Some(side_room_iter(c.owner.point, c.max_lane, others.iter().map(|r| r.point), p.block));
        self.visible.clear();
        self.visible.extend(others
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                is_visible(
                    r.point.position - c.owner.point.position,
                    r.point.lane - c.owner.point.lane,
                    c.vision,
                )
            })
            .map(|(i, _)| i));
        self.force_in = c.empty_in_move
            && c.phase < 1
            && c.owner.point.lane >= p.force_in_threshold
            && others.iter().any(|r| {
                let gap = c.owner.point.lane - r.point.lane;
                (r.point.position - c.owner.point.position).abs() < p.block.side_distance
                    && gap > 0.
                    && gap < p.block.side_lane
            });
    }

    pub fn update_targets(
        &mut self,
        c: &TargetContext,
        live: &[Horse],
        dt: f32,
        p: TargetParameters,
    ) {
        assert_eq!(live.len(), self.cached.len());
        let front = front_index_iter(c.owner.point, self.cached.iter().map(|r| r.point), p.block);
        let eligible = |i: usize| {
            let row = self.cached[i];
            crate::overtaking::is_target_with_block_check(
                (row.point.position - c.owner.point.position) as f64,
                (c.owner.speed - row.speed) as f64,
                c.owner.target as f64,
                live[i].target as f64,
                live[i].speed as f64,
                front == Some(i),
                || front_index_iter(row.point, self.cached.iter().map(|r| r.point)
                    .chain(std::iter::once(c.owner.point)), p.block).is_some(),
            )
        };
        self.targets.retain(|&i| i < self.cached.len());
        for &i in &self.visible {
            if !self.targets.contains(&i) && eligible(i) {
                self.targets.push(i);
            }
        }
        self.targets.retain(|&i| eligible(i));
        if let Some(i) = front {
            if !self.targets.contains(&i) {
                self.targets.push(i);
            }
        }
        if !self.targets.is_empty() {
            self.cooldown = p.cooldown;
        } else if self.cooldown > 0. {
            self.cooldown -= dt;
            if self.cooldown <= 0. {
                self.force_check = true;
            }
        }
    }

    pub fn target(
        &mut self,
        c: &TargetContext,
        live: &[Horse],
        p: TargetParameters,
        rng: &mut impl RandomSource,
    ) -> f32 {
        let side = self
            .side
            .expect("Collect retail lane cache before target search");
        let lane = c.owner.point.lane;
        let delta = c.previous_target - lane;
        if !self.force_check
            && self.targets.is_empty()
            && delta.abs() >= 1. / 36.
            && if delta > 0. {
                side.can_out
            } else {
                side.can_in
            }
        {
            return c.previous_target;
        }
        self.force_check = false;
        if self.extra_outward.is_none()
            && c.final_corner_start
                .map_or(false, |start| c.owner.point.position >= start)
        {
            let normalized = (lane / p.outward_normalize).clamp(0., 1.);
            self.extra_outward = Some(
                (normalized * p.outward_coefficient + rng.real(0., p.outward_random))
                    .clamp(0., c.max_lane),
            );
        }
        let passing = (!self.targets.is_empty() || self.cooldown > 0.)
            && (c.phase >= 2
                || !(c.first_move_point - 200. <= c.owner.point.position
                    && c.owner.point.position <= c.first_move_point));
        if passing {
            self.live_points.clear();
            self.live_points.extend(live.iter().map(|r| r.point));
            self.cached_points.clear();
            self.cached_points.extend(self.cached.iter().map(|r| r.point));
            let mut target = PassingSearch {
                lane_preference: c.lane_preference,
                owner: c.owner.point,
                phase: c.phase,
                max_lane: c.max_lane,
                live: &self.live_points,
                cached: &self.cached_points,
                visible: &self.visible,
                targets: &self.targets,
                side,
            }
            .target();
            let extra = self.extra_outward.or_else(|| {
                if c.fixed_goal != 0. {
                    Some(c.fixed_goal)
                } else {
                    None
                }
            });
            if let Some(extra) = extra {
                if target >= lane && extra > target && side.reachable(lane, extra) {
                    target = extra;
                }
            }
            return target;
        }
        if c.fixed_goal != 0. {
            return if c.fixed_goal > lane && side.can_out {
                c.fixed_goal.min(lane + 0.05)
            } else if c.fixed_goal < lane && side.can_in {
                c.fixed_goal.max(lane - 0.05)
            } else {
                lane
            };
        }
        if !c.has_hp {
            return lane;
        }
        if c.pace_down {
            return 0.18;
        }
        if let Some(extra) = self.extra_outward {
            return if extra > lane && side.can_out {
                (lane + 0.05).min(extra)
            } else {
                lane
            };
        }
        if c.phase < 2 && side.can_in {
            return lane - 0.05;
        }
        if c.phase == 1 {
            if let Some(inner) = side.inner_blocker {
                if lane - inner < p.block.side_lane - (1. / 72.) {
                    // Normal.Search tests cached gap, then gets the cached
                    // blocker's live lane (3432a16..3432a54).
                    let live_inner = side.inner_blocker_index
                        .and_then(|i| live.get(i)).map_or(inner, |h| h.point.lane);
                    return (lane + 0.05).min(live_inner + p.block.side_lane);
                }
            }
        }
        lane
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BlockParameters {
    pub front_distance: f32,
    pub front_lane: f32,
    pub side_distance: f32,
    pub side_lane: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SideRoom {
    pub can_in: bool,
    pub can_out: bool,
    pub in_space: f32,
    pub out_space: f32,
    pub inner_blocker: Option<f32>,
    pub inner_blocker_index: Option<usize>,
}

impl SideRoom {
    pub fn reachable(&self, lane: f32, candidate: f32) -> bool {
        let delta = candidate - lane;
        delta == 0.
            || if delta > 0. {
                self.can_out && delta <= self.out_space
            } else {
                self.can_in && -delta <= self.in_space
            }
    }
}

pub fn is_visible(gap: f32, lane_gap: f32, vision: f32) -> bool {
    if vision <= 0. || !(0. ..=vision).contains(&gap) {
        return false;
    }
    let width = ((gap / vision) * 0.6388888955116272_f32 + 0.1111111119389534_f32) * 0.5;
    lane_gap.abs() <= width
}

pub fn front_index(owner: Row, rows: &[Row], p: BlockParameters) -> Option<usize> {
    front_index_iter(owner, rows.iter().copied(), p)
}

pub(crate) fn front_index_iter(owner: Row, rows: impl Iterator<Item = Row>, p: BlockParameters) -> Option<usize> {
    let mut found = None;
    let mut nearest = f32::INFINITY;
    for (i, row) in rows.enumerate() {
        let gap = row.position - owner.position;
        let width = (1. - (gap / p.front_distance) * 0.6) * p.front_lane;
        if gap > 0.
            && gap < p.front_distance
            && gap < nearest
            && (row.lane - owner.lane).abs() <= width
        {
            found = Some(i);
            nearest = gap;
        }
    }
    found
}

pub fn side_room(owner: Row, max_lane: f32, rows: &[Row], p: BlockParameters) -> SideRoom {
    side_room_iter(owner, max_lane, rows.iter().copied(), p)
}

fn side_room_iter(owner: Row, max_lane: f32, rows: impl Iterator<Item = Row>, p: BlockParameters) -> SideRoom {
    let mut result = SideRoom {
        can_in: owner.lane > 0.0001,
        can_out: owner.lane < max_lane,
        in_space: owner.lane,
        out_space: max_lane - owner.lane,
        inner_blocker: None,
        inner_blocker_index: None,
    };
    for (index, row) in rows.enumerate() {
        if (row.position - owner.position).abs() >= p.side_distance {
            continue;
        }
        let gap = row.lane - owner.lane;
        if gap < 0. {
            result.in_space = result.in_space.min(-gap);
            if -gap < p.side_lane {
                result.can_in = false;
                result.inner_blocker = Some(row.lane);
                result.inner_blocker_index = Some(index);
            }
        } else if gap > 0. {
            result.out_space = result.out_space.min(gap);
            if gap < p.side_lane {
                result.can_out = false;
            }
        }
    }
    result
}

pub struct PassingSearch<'a> {
    pub lane_preference: u8,
    pub owner: Row,
    pub phase: i32,
    pub max_lane: f32,
    /// Stable around-horse order excluding the owner, including finishers.
    pub live: &'a [Row],
    pub cached: &'a [Row],
    /// Visible indices from the cached PreUpdate view, sampled from live rows
    /// when checking the corridor. Do not recompute visibility after movement.
    pub visible: &'a [usize],
    pub targets: &'a [usize],
    pub side: SideRoom,
}

#[derive(Clone, Copy, Debug)]
pub struct MoveParameters {
    pub power_coefficient: f32,
    pub speed_add: f32,
    pub base_speed: f32,
    pub outer_coefficient: f32,
    pub acceleration: f32,
}

impl MoveParameters {
    pub fn from_value(value: &crate::json::Value) -> Result<Self, String> {
        let number = |key| -> Result<f32, String> {
            let result = value.get_f64(key)? as f32;
            if result.is_finite() {
                Ok(result)
            } else {
                Err(format!("Invalid lane parameter {key}"))
            }
        };
        Ok(Self {
            power_coefficient: number("laneMoveSpeedPowCoef")?,
            speed_add: number("laneMoveSpeedAdd")?,
            base_speed: number("laneMoveSpeedBase")?,
            outer_coefficient: number("laneMoveSpeedOutCoef")?,
            acceleration: number("laneMoveAccelMoveSpeedBaseCoef")?,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MoveState {
    pub lane: f32,
    pub target: f32,
    pub speed: f32,
    pub continue_time: f32,
    pub continue_direction: i32,
}

pub struct MoveContext<'a> {
    pub position: f32,
    pub power: f32,
    pub phase: i32,
    pub rank: usize,
    pub first_move_point: f32,
    pub max_lane: f32,
    pub side: SideRoom,
    pub modifiers: &'a [f32],
}

fn approximately(a: f32, b: f32) -> bool {
    (a - b).abs() < (1e-6_f32 * a.abs().max(b.abs())).max(1.1210387714598537e-44_f32)
}

impl MoveState {
    /// The separate field-overlap pass must run after every runner moves.
    /// Returns whether movement occurred; blocked moves may retain momentum.
    pub fn step(&mut self, context: MoveContext<'_>, dt: f32, p: MoveParameters) -> bool {
        use crate::retail_physics::{apply_additive_modifiers, bounded_stat};
        self.target = self.target.clamp(0., context.max_lane);
        if approximately(self.target, self.lane) {
            self.continue_time = 0.;
            return false;
        }
        let previous_direction = self.continue_direction;
        self.continue_direction = (self.target >= self.lane) as i32;
        // v20: lateral movement follows the backend's x87-style evaluation:
        // the base rate local is computed at double and rounded once; the
        // factor is an unrounded double temporary of `rate * factor`, whose
        // product is rounded once. Inward speed and the lane update are
        // likewise double expressions rounded on the store (full corpus:
        // exact distance samples +135k, never-drifting runners +2079).
        let rate = ((bounded_stat(context.power) as f64 * p.power_coefficient as f64
            + p.speed_add as f64)
            * p.base_speed as f64) as f32;
        let factor = if context.phase >= 2 {
            // v10: official late-race lateral speed uses the 0-based order (leader x1.00).
            1. + (context.rank as f64 - 1.) * 0.01f32 as f64
        } else if context.position <= context.first_move_point {
            // v69: the lane ratio is a float quotient (doc 13.186).
            1. + (self.lane / context.max_lane) as f64 * p.outer_coefficient as f64
        } else {
            1.
        };
        let rate = (rate as f64 * factor) as f32;
        // v69: the lane-change acceleration is an unrounded double temporary
        // added to the stored lateral speed (doc 13.186).
        let acceleration = (p.acceleration as f64 * p.base_speed as f64) * dt as f64;
        self.speed = if self.speed < rate {
            ((self.speed as f64 + acceleration) as f32).min(rate)
        } else {
            ((self.speed as f64 - acceleration) as f32).max(rate)
        }
        .max(0.);
        let mut speed =
            apply_additive_modifiers(self.speed, context.modifiers.iter().copied()).clamp(0., 0.6);
        if approximately(speed, 0.) {
            return false;
        }
        if self.target < self.lane {
            speed = (-(speed as f64) * (1. + self.lane as f64)) as f32;
        }
        let mut next = (self.lane as f64 + dt as f64 * speed as f64) as f32;
        next = if self.target < self.lane {
            next.max(self.target)
        } else {
            next.min(self.target)
        };
        next = next.clamp(0., context.max_lane);
        if (next < self.lane && !context.side.can_in) || (next > self.lane && !context.side.can_out)
        {
            return false;
        }
        let moved = next != self.lane;
        self.lane = next;
        if previous_direction == self.continue_direction {
            self.continue_time += dt;
        }
        if approximately(next, self.target) {
            self.speed = 0.;
        }
        moved
    }
}

impl PassingSearch<'_> {
    pub fn target(&self) -> f32 {
        let lane = self.owner.lane;
        const EDGE: f32 = 0.055555559694767_f32;
        const STRAIGHT: f32 = 0.0555555559694767_f32;
        const CORRIDOR: f32 = 0.04444444924592972_f32;
        const CROWD: f32 = 0.1111111119389534_f32;
        assert_eq!(
            self.live.len(),
            self.cached.len(),
            "Live/cached horse indices must align"
        );
        let clear = |candidate: f32, end: f32, bounds: Option<(f32, f32)>| {
            if !(0. ..=self.max_lane).contains(&candidate) || !self.side.reachable(lane, candidate)
            {
                return false;
            }
            let (low, high) = bounds.unwrap_or((candidate - CORRIDOR, candidate + CORRIDOR));
            !self.visible.iter().any(|&i| {
                let row = self.live[i];
                self.owner.position <= row.position
                    && row.position <= end
                    && low <= row.lane
                    && row.lane <= high
            })
        };
        let mut candidates = Vec::new();
        for &index in self.targets {
            for direction in [-1., 1.] {
                let mut edge = index;
                loop {
                    let live = self.live[edge];
                    let cached = self.cached[edge];
                    let mut next: Option<usize> = None;
                    for (j, row) in self.cached.iter().enumerate() {
                        let position_gap = row.position - cached.position;
                        let lane_gap = direction * (row.lane - cached.lane);
                        if (0. ..=3.).contains(&position_gap)
                            && lane_gap > 0.
                            && lane_gap < CROWD
                            && direction * (self.live[j].lane - live.lane) > 0.
                        {
                            // Python max retains the first entry on equal
                            // live lane; do not let a sort choose the last.
                            if next.is_none()
                                || direction * self.live[j].lane
                                    > direction * self.live[next.unwrap()].lane
                            {
                                next = Some(j);
                            }
                        }
                    }
                    match next {
                        Some(i) => edge = i,
                        None => break,
                    }
                }
                let row = self.live[edge];
                let candidate = row.lane + direction * EDGE;
                if clear(candidate, row.position + 0.5, None) {
                    candidates.push(candidate);
                }
            }
        }
        let straight = if self.phase < 2 && self.side.can_in && lane > STRAIGHT {
            lane - STRAIGHT
        } else {
            lane
        };
        let nearest = self
            .targets
            .iter()
            .map(|&i| self.live[i].position)
            .fold(f32::MAX, f32::min);
        // These two operations cannot be regrouped around the rounded lane.
        let bounds = if straight != lane {
            Some(((lane - CORRIDOR) - STRAIGHT, (lane + CORRIDOR) - STRAIGHT))
        } else {
            None
        };
        if clear(straight, nearest + 3., bounds) {
            candidates.push(straight);
        }
        let score = |candidate: f32| {
            let weight = if candidate < lane {
                if self.phase == 0 {
                    100.
                } else if self.phase >= 2 {
                    1.149999976158142_f32
                } else {
                    1.
                }
            } else {
                1.
            };
            let gap=lane-candidate;
            let skill=if (self.lane_preference==1 && gap>0.) || (self.lane_preference==2 && gap<0.) {1.5} else {1.};
            ((gap*weight)*skill).abs()
        };
        let mut best = lane;
        let mut best_score = f32::INFINITY;
        for candidate in candidates {
            let cost = score(candidate);
            if cost < best_score {
                best = candidate;
                best_score = cost;
            }
        }
        best
    }
}

/// Separate manager pass after lateral movement, using the loaded comparison
/// sort's equal-key permutation. It does not clamp the displacement to a rail.
pub fn resolve_overlaps(rows: &mut [Row], force_checks: &mut [bool]) {
    assert_eq!(rows.len(), force_checks.len());
    let mut order: Vec<usize> = (0..rows.len()).collect();
    comparison_sort(&mut order, |&a, &b| {
        (rows[a].lane as i32).cmp(&(rows[b].lane as i32))
    });
    if let Some(&first) = order.first() {
        force_checks[first] = false;
    }
    for pair in order.windows(2) {
        let previous = rows[pair[0]];
        let current = &mut rows[pair[1]];
        if (current.position - previous.position).abs() < 0.4
            && (current.lane - previous.lane).abs() < (0.4_f64 / 18.) as f32
        {
            current.lane = previous.lane + (0.4_f64 / 18.) as f32;
            force_checks[pair[1]] = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::Value;
    #[test]
    fn lane_continuity_preserves_blocked_time_and_skips_direction_changes() {
        let p = MoveParameters { power_coefficient: 0.001, speed_add: 0.3,
            base_speed: 0.02, outer_coefficient: 0.05, acceleration: 1.5 };
        let mut state = MoveState { lane: 0.6, target: 0., speed: 0.,
            continue_time: 0., continue_direction: 0 };
        let step = |s: &mut MoveState, allowed| {
            s.step(MoveContext { position: 100., power: 1000., phase: 0,
                rank: 1, first_move_point: 0., max_lane: 1.3, modifiers: &[],
                side: SideRoom { can_in: allowed, can_out: allowed,
                    in_space: 1., out_space: 1., inner_blocker: None, inner_blocker_index: None } }, 0.0666, p)
        };
        for _ in 0..15 { step(&mut state, true); }
        assert!(state.continue_time < 1.);
        step(&mut state, true);
        assert!(state.continue_time >= 1.);
        let time = state.continue_time;
        assert!(!step(&mut state, false));
        assert_eq!(state.continue_time, time);
        state.target = 1.3;
        step(&mut state, true);
        assert_eq!(state.continue_direction, 1);
        assert_eq!(state.continue_time, time);
        step(&mut state, true);
        assert_eq!(state.continue_time, time + 0.0666);
        state.target = state.lane;
        step(&mut state, true);
        assert_eq!(state.continue_time, 0.);
    }
    fn row(v: &Value) -> Row {
        let a = v.as_array().unwrap();
        Row {
            position: a[0].as_f64().unwrap() as f32,
            lane: a[1].as_f64().unwrap() as f32,
        }
    }
    fn indices(v: &Value, name: &str) -> Vec<usize> {
        v.get_array(name)
            .unwrap()
            .iter()
            .map(|n| n.as_i64().unwrap() as usize)
            .collect()
    }
    fn fixture() -> Value {
        crate::json::parse(include_str!("../tests/fixtures/retail_lane_port.json"))
        .unwrap()
    }
    #[test]
    fn cached_and_live_passing_search_matches_python_bits() {
        let f = fixture();
        let p = f.get("block").unwrap();
        let block = BlockParameters {
            front_distance: p.get_f64("FrontBlockDistanceGap").unwrap() as f32,
            front_lane: p.get_f64("FrontBlockLaneGap").unwrap() as f32,
            side_distance: p.get_f64("SideBlockDistanceGap").unwrap() as f32,
            side_lane: p.get_f64("SideBlockLaneGap").unwrap() as f32,
        };
        for (case, v) in f.get_array("searches").unwrap().iter().enumerate() {
            let owner = row(v.get("owner").unwrap());
            let cached = v
                .get_array("cached")
                .unwrap()
                .iter()
                .map(row)
                .collect::<Vec<_>>();
            let live = v
                .get_array("live")
                .unwrap()
                .iter()
                .map(row)
                .collect::<Vec<_>>();
            let max_lane = v.get_f64("max_lane").unwrap() as f32;
            let side = side_room(owner, max_lane, &cached, block);
            let targets = indices(v, "targets");
            let visible = indices(v, "visible");
            let vision = v.get_f64("vision").unwrap() as f32;
            let actual_visible: Vec<_> = cached
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    is_visible(r.position - owner.position, r.lane - owner.lane, vision)
                })
                .map(|(i, _)| i)
                .collect();
            assert_eq!(actual_visible, visible, "visibility case {case}");
            assert_eq!(
                front_index(owner, &cached, block)
                    .map(|i| i as i64)
                    .unwrap_or(-1),
                v.get_i64("front").unwrap(),
                "front case {case}"
            );
            let target = PassingSearch {
                lane_preference: v.get("lane_preference").and_then(Value::as_i64).unwrap_or(0) as u8,
                owner,
                phase: v.get_i64("phase").unwrap() as i32,
                max_lane,
                live: &live,
                cached: &cached,
                visible: &visible,
                targets: &targets,
                side,
            }
            .target();
            assert_eq!(
                target.to_bits(),
                v.get_i64("target_bits").unwrap() as u32,
                "passing case {case}"
            );
        }
    }
    #[test]
    fn overlap_order_and_flags_match_python_on_small_and_large_fields() {
        for v in fixture().get_array("overlaps").unwrap() {
            let mut rows = v
                .get_array("rows")
                .unwrap()
                .iter()
                .map(row)
                .collect::<Vec<_>>();
            let mut flags = v
                .get_array("flags")
                .unwrap()
                .iter()
                .map(|x| x.as_bool().unwrap())
                .collect::<Vec<_>>();
            resolve_overlaps(&mut rows, &mut flags);
            let expected = v.get_array("expected").unwrap();
            for (i, r) in rows.iter().enumerate() {
                let e = expected[i].as_array().unwrap();
                assert_eq!(r.lane.to_bits(), e[0].as_i64().unwrap() as u32);
                assert_eq!(flags[i], e[1].as_bool().unwrap());
            }
        }
    }

    #[test]
    fn lateral_acceleration_and_permissions_match_python_bits() {
        let fixture = fixture();
        let p = MoveParameters::from_value(fixture.get("move_parameters").unwrap()).unwrap();
        for (case, row) in fixture.get_array("moves").unwrap().iter().enumerate() {
            let n = |key| row.get_f64(key).unwrap() as f32;
            let mut state = MoveState {
                lane: n("lane"),
                target: n("target"),
                speed: n("speed"),
                continue_time: 0.,
                continue_direction: 0,
            };
            let modifiers = row
                .get_array("modifiers")
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap() as f32)
                .collect::<Vec<_>>();
            let side = SideRoom {
                can_in: row.get_bool("can_in").unwrap(),
                can_out: row.get_bool("can_out").unwrap(),
                in_space: 0.,
                out_space: 0.,
                inner_blocker: None,
                inner_blocker_index: None,
            };
            let context = MoveContext {
                position: n("position"),
                power: n("power"),
                phase: n("phase") as i32,
                rank: n("rank") as usize,
                first_move_point: n("first_move"),
                max_lane: n("max_lane"),
                side,
                modifiers: &modifiers,
            };
            let moved = state.step(context, n("dt"), p);
            for (channel, (actual, expected)) in [state.lane, state.target, state.speed]
                .iter()
                .zip(row.get_array("expected_bits").unwrap())
                .enumerate()
            {
                assert_eq!(
                    actual.to_bits(),
                    expected.as_i64().unwrap() as u32,
                    "move case {case} channel {channel}"
                );
            }
            assert_eq!(
                moved,
                row.get_bool("moved").unwrap(),
                "movement flag case {case}"
            );
        }
    }

    #[test]
    fn target_state_retention_cooldown_and_shared_draws_match_python() {
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_lane_state_port.json"))
        .unwrap();
        let p = TargetParameters::from_value(fixture.get("parameters").unwrap()).unwrap();
        let horse = |v: &Value| {
            let a = v.as_array().unwrap();
            Horse {
                point: Row {
                    position: a[0].as_f64().unwrap() as f32,
                    lane: a[1].as_f64().unwrap() as f32,
                },
                speed: a[2].as_f64().unwrap() as f32,
                target: a[3].as_f64().unwrap() as f32,
            }
        };
        for trace in fixture.get_array("traces").unwrap() {
            let mut rng =
                crate::retail_random::RetailRandom::new(trace.get_i64("seed").unwrap() as i32);
            let mut state = TargetState::default();
            for (tick, row) in trace.get_array("rows").unwrap().iter().enumerate() {
                if tick % 13 == 0 {
                    rng.real(-1., 1.);
                }
                let v = row.get("context").unwrap();
                let n = |key| v.get_f64(key).unwrap() as f32;
                let c = TargetContext {
                    lane_preference: v.get("lane_preference").and_then(Value::as_i64).unwrap_or(0) as u8,
                    owner: horse(v.get("owner").unwrap()),
                    phase: n("phase") as i32,
                    max_lane: n("max_lane"),
                    vision: n("vision"),
                    first_move_point: n("first_move"),
                    final_corner_start: v.get("final_corner").unwrap().as_f64().map(|x| x as f32),
                    empty_in_move: v.get_bool("empty_in_move").unwrap(),
                    previous_target: n("previous_target"),
                    fixed_goal: n("fixed_goal"),
                    has_hp: v.get_bool("has_hp").unwrap(),
                    pace_down: v.get_bool("pace_down").unwrap(),
                };
                let cached = row
                    .get_array("cached")
                    .unwrap()
                    .iter()
                    .map(horse)
                    .collect::<Vec<_>>();
                let live = row
                    .get_array("live")
                    .unwrap()
                    .iter()
                    .map(horse)
                    .collect::<Vec<_>>();
                state.collect(&c, &cached, p);
                state.update_targets(&c, &live, fixture.get_f64("dt").unwrap() as f32, p);
                let result = state.target(&c, &live, p, &mut rng);
                assert_eq!(
                    state.targets,
                    indices(row, "targets"),
                    "targets tick {tick}"
                );
                assert_eq!(
                    state.visible,
                    indices(row, "visible"),
                    "visibility tick {tick}"
                );
                assert_eq!(
                    result.to_bits(),
                    row.get_i64("target_bits").unwrap() as u32,
                    "target tick {tick}"
                );
                assert_eq!(
                    state.cooldown.to_bits(),
                    row.get_i64("cooldown_bits").unwrap() as u32,
                    "cooldown tick {tick}"
                );
                assert_eq!(
                    state.extra_outward.map(|x| x.to_bits() as i64),
                    row.get("extra_bits").unwrap().as_i64(),
                    "outward tick {tick}"
                );
                assert_eq!(
                    state.force_check,
                    row.get_bool("force_check").unwrap(),
                    "force tick {tick}"
                );
                assert_eq!(
                    state.force_in,
                    row.get_bool("force_in").unwrap(),
                    "force inward tick {tick}"
                );
                assert_eq!(
                    rng.draw_count,
                    row.get_i64("draws").unwrap() as usize,
                    "draws tick {tick}"
                );
            }
        }
    }
}
