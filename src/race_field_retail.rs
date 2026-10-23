//! Experimental whole-field order. Ports the reconstructed Python manager;
//! matching it does not establish official-game parity.
use super::*;
use crate::retail_lane::{self, Row};

/// Front blocking includes finishers; the side-time counters do not. Both
/// queries observe the same post-skill positions before any runner moves.
fn blocking_sample(owner: usize, rows: &[(f32, f32, bool)], p: retail_lane::BlockParameters)
    -> (Option<(usize, f32)>, bool)
{
    let (position, lane, _) = rows[owner];
    let mut front = None;
    let mut nearest = f32::INFINITY;
    let mut side = false;
    for (i, &(other_position, other_lane, finished)) in rows.iter().enumerate() {
        if i == owner { continue; }
        let gap = other_position - position;
        let lane_gap = other_lane - lane;
        if gap > 0. && gap < p.front_distance && gap < nearest
            && lane_gap.abs() <= retail_lane::front_lane_limit(gap, p)
        {
            front = Some((i, gap));
            nearest = gap;
        }
        if !finished && gap.abs() < p.side_distance && lane_gap.abs() < p.side_lane
            && (lane_gap < 0. || lane_gap > 0.)
        { side = true; }
    }
    (front, side)
}

/// `blocking_sample` over rows sorted by position (`order`, with `slot[i]`
/// giving row i's place in it). Float subtraction is monotonic, so each scan
/// stops at the first row whose gap cannot qualify. Front ties keep the
/// lowest index, as in the indexed scan.
fn blocking_sample_sorted(owner: usize, rows: &[(f32, f32, bool)], order: &[usize], slot: &[usize],
    p: retail_lane::BlockParameters) -> (Option<(usize, f32)>, bool)
{
    let (position, lane, _) = rows[owner];
    let mut front: Option<(usize, f32)> = None;
    let mut side = false;
    let reach = p.front_distance.max(p.side_distance);
    let mut visit = |i: usize, gap: f32| {
        let (_, other_lane, finished) = rows[i];
        let lane_gap = other_lane - lane;
        if gap > 0. && gap < p.front_distance
            && front.map_or(true, |(j, nearest)| gap < nearest || (gap == nearest && i < j))
            && lane_gap.abs() <= retail_lane::front_lane_limit(gap, p)
        {
            front = Some((i, gap));
        }
        if !finished && gap.abs() < p.side_distance && lane_gap.abs() < p.side_lane
            && (lane_gap < 0. || lane_gap > 0.)
        { side = true; }
    };
    for &i in &order[slot[owner] + 1..] {
        let gap = rows[i].0 - position;
        if gap >= reach { break; }
        visit(i, gap);
    }
    for &i in order[..slot[owner]].iter().rev() {
        let gap = rows[i].0 - position;
        if gap.abs() >= p.side_distance { break; }
        visit(i, gap);
    }
    (front, side)
}

/// Sort runner indices by position, starting from the previous order.
/// Runners rarely swap between ticks, so insertion sort is near linear.
/// Callers that depend on tie order break ties explicitly by index.
fn resort_by_position(order: &mut Vec<usize>, slot: &mut Vec<usize>, rows: &[(f32, f32, bool)]) {
    if order.len() != rows.len() {
        order.clear();
        order.extend(0..rows.len());
    }
    for k in 1..order.len() {
        let i = order[k];
        let mut m = k;
        while m > 0 && rows[order[m - 1]].0 > rows[i].0 {
            order[m] = order[m - 1];
            m -= 1;
        }
        order[m] = i;
    }
    slot.resize(rows.len(), 0);
    for (k, &i) in order.iter().enumerate() { slot[i] = k; }
}

/// Reverse passing membership for the current ordered movement pass. A row
/// changes only after its owner moves, or after a peer-mutating callback.
#[derive(Default)]
struct PassingSnapshot {
    outgoing: Vec<u128>,
    incoming: Vec<u128>,
    behind: Vec<u128>,
}

impl PassingSnapshot {
    fn rebuild(&mut self, field: &RaceField) {
        self.outgoing.resize(field.n, 0);
        self.outgoing.fill(0);
        self.incoming.resize(field.n, 0);
        self.incoming.fill(0);
        self.behind.resize(field.n, 0);
        let mut behind = 0_u128;
        for &i in field.order.borrow().iter().rev() {
            self.behind[i] = behind;
            behind |= 1_u128 << i;
        }
        for (i, cell) in field.solvers.iter().enumerate() {
            self.update(i, &cell.borrow().retail.as_ref().unwrap().lane.targets);
        }
    }

    fn update(&mut self, owner: usize, targets: &[usize]) {
        let mut mask = 0_u128;
        for &local in targets {
            if local >= self.outgoing.len().saturating_sub(1) { continue; }
            let target = local + usize::from(local >= owner);
            mask |= 1_u128 << target;
        }
        let mut changed = self.outgoing[owner] ^ mask;
        self.outgoing[owner] = mask;
        while changed != 0 {
            let target = changed.trailing_zeros() as usize;
            self.incoming[target] ^= 1_u128 << owner;
            changed &= changed - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PassingSnapshot;

    #[test]
    fn passing_membership_keeps_owner_exclusion_and_replacements() {
        for n in [1, 2, 9, 18, 64, 65, 128] {
            let mut snapshot = PassingSnapshot {
                outgoing: vec![0; n], incoming: vec![0; n], behind: vec![0; n],
            };
            let mut lists = vec![Vec::new(); n];
            for step in 0..n * 5 {
                let owner = step % n;
                lists[owner] = match step % 5 {
                    0 => (0..n.saturating_sub(1)).collect(),
                    1 => vec![0, 0, n, usize::MAX],
                    2 => vec![],
                    _ => vec![n.saturating_sub(2)],
                };
                snapshot.update(owner, &lists[owner]);
                for target in 0..n {
                    let expected = lists.iter().enumerate().fold(0_u128, |mask, (i, list)| {
                        if i != target && list.contains(&(target - usize::from(target > i))) {
                            mask | (1_u128 << i)
                        } else { mask }
                    });
                    assert_eq!(snapshot.incoming[target], expected, "n={n} step={step} target={target}");
                }
            }
        }
    }
}

#[derive(Default)]
pub(super) struct TickBuffers {
    before: Vec<(f32, f32, bool)>,
    roster: Vec<retail_lane::Horse>,
    advanced: Vec<(usize, f64)>,
    overlaps: Vec<Row>,
    flags: Vec<bool>,
    overlap_order: Vec<usize>,
    by_position: Vec<usize>,
    slot: Vec<usize>,
    before_order: Vec<usize>,
    before_slot: Vec<usize>,
    passing: PassingSnapshot,
}

impl RaceField {
    pub(super) fn step_all_retail(&self, dt: f32, indices: &[usize]) {
        let mut buffers = self.retail_tick.borrow_mut();
        let TickBuffers { before: rows, roster, advanced, overlaps, flags, overlap_order, by_position, slot,
            before_order, before_slot, passing } = &mut *buffers;
        rows.clear();
        roster.clear();
        for cell in &self.solvers {
            let s = cell.borrow();
            rows.push((
                s.pos as f32,
                s.lane as f32,
                !s.course.corners.is_empty() && s.course.straights.last()
                    .is_some_and(|r| r.start <= s.pos && s.pos < r.end),
            ));
            // Every lane cache sees the same pre-movement field. Capture each
            // runner once, retaining the original owner-excluded roster order.
            roster.push(retail_lane::Horse {
                point: Row { position: s.pos as f32, lane: s.lane as f32 },
                speed: s.retail.as_ref().map_or(s.current_speed as f32, |r| r.observed_speed),
                target: s.target_speed as f32,
            });
        }
        // Lane caches scan only nearby runners when positions can be ordered.
        let before_sorted = rows.iter().all(|row| row.0.is_finite());
        if before_sorted { resort_by_position(before_order, before_slot, rows); }
        let any_straight = rows.iter().any(|row| row.2);
        for &i in indices {
            let mut s = self.solvers[i].borrow_mut();
            s.collect_retail_duel_proximity(&rows, any_straight, dt);
            let position = s.pos as f32;
            let r = s.retail.as_mut().unwrap();
            if let Some(geometry) = &r.config.geometry {
                r.lane_max = geometry.lane_max_at(position);
            }
            let context = s.retail_lane_context();
            let r = s.retail.as_mut().unwrap();
            r.lane.collect_roster(&context, roster, i, r.config.lane,
                before_sorted.then_some((&before_order[..], &before_slot[..])));
        }
        for &i in indices {
            self.solvers[i]
                .borrow_mut()
                .prepare_retail_tick(dt, Some(self), false);
        }
        for priority in [1, 0] {
            for &i in indices {
                self.solvers[i]
                    .borrow_mut()
                    .process_skill_priority(Some(priority), Some(self));
            }
        }
        self.flush_skill_effects();
        // Skills have now finished their whole-field pass. Keep live rows
        // current in movement order: later runners must see earlier moves.
        rows.clear();
        for (row, cell) in roster.iter_mut().zip(&self.solvers) {
            let s = cell.borrow();
            *row = s.retail_live_horse();
            rows.push((s.pos as f32, s.lane as f32, s.pos >= s.course.distance));
        }
        // Positions are finite during a race; otherwise keep the indexed scan.
        let sorted = rows.iter().all(|row| row.0.is_finite());
        if sorted { resort_by_position(by_position, slot, rows); }
        for &i in indices {
            let block = self.retail_block.unwrap();
            let (front, side) = if sorted {
                blocking_sample_sorted(i, rows, by_position, slot, block)
            } else {
                blocking_sample(i, rows, block)
            };
            #[cfg(test)] assert_eq!((front, side), blocking_sample(i, rows, block));
            let mut s = self.solvers[i].borrow_mut();
            #[cfg(test)] {
                assert_eq!(front, self.front_blocker(i, s.pos, s.lane).map(|(j, gap)| (j, gap as f32)));
                let (inner, outer) = self.side_room(i, s.pos, s.lane);
                assert_eq!(side, inner || outer);
            }
            s.retail.as_mut().unwrap().front_blocker = front;
            s.update_retail_blocking(dt as f64, side);
        }
        let indexed_passing = self.n <= 128;
        if indexed_passing { passing.rebuild(self); }
        advanced.clear();
        advanced.reserve(indices.len());
        for &i in indices {
            let (elapsed, dirty) = {
                let mut s = self.solvers[i].borrow_mut();
                let elapsed = s.step_forward_retail_with_roster(dt, Some(self), Some(roster),
                    before_sorted.then_some((&before_order[..], &before_slot[..])));
                roster[i] = s.retail_live_horse();
                if indexed_passing { passing.update(i, &s.retail.as_ref().unwrap().lane.targets); }
                (elapsed, s.retail.as_ref().unwrap().live_roster_dirty)
            };
            if dirty {
                for (row, cell) in roster.iter_mut().zip(&self.solvers) {
                    *row = cell.borrow().retail_live_horse();
                }
                if indexed_passing { passing.rebuild(self); }
            }
            advanced.push((i, elapsed));
            let overtake_times = if indexed_passing {
                let s = self.solvers[i].borrow();
                let targeted = passing.incoming[i] & passing.behind[i] != 0;
                let has_targets = !s.retail.as_ref().unwrap().lane.targets.is_empty();
                let advance = |value: f64, enabled: bool| if enabled { (value as f32 + dt) as f64 } else { 0. };
                let result = (advance(s.overtake_target_time, targeted),
                    advance(s.overtake_target_no_order_up_time, has_targets),
                    advance(s.overtake_target_no_order_down_time, has_targets));
                #[cfg(test)] assert_eq!(result, self.retail_overtake_times_for(i, dt));
                result
            } else { self.retail_overtake_times_for(i, dt) };
            let mut s = self.solvers[i].borrow_mut();
            (
                s.overtake_target_time,
                s.overtake_target_no_order_up_time,
                s.overtake_target_no_order_down_time,
            ) = overtake_times;
        }
        let rows = overlaps;
        rows.clear();
        rows.extend(self
            .solvers
            .iter()
            .map(|s| {
                let s = s.borrow();
                Row {
                    position: s.pos as f32,
                    lane: s.lane as f32,
                }
            })
            );
        flags.clear();
        flags.extend(self
            .solvers
            .iter()
            .map(|s| s.borrow().retail.as_ref().unwrap().lane.force_check)
            );
        retail_lane::resolve_overlaps_with_order(rows, flags, overlap_order);
        for (i, row) in rows.iter().enumerate() {
            let mut s = self.solvers[i].borrow_mut();
            s.lane = row.lane as f64;
            s.retail.as_mut().unwrap().lane.force_check = flags[i];
        }
        // Isolated hypothesis: commit new duel participants before evaluating
        // exits, so a same-tick join can sustain its existing partners.
        for &(i, elapsed) in advanced.iter() {
            if elapsed > 0. {
                self.solvers[i].borrow_mut().prepare_retail_duel_start(elapsed, self);
            }
        }
        for &(i, elapsed) in advanced.iter() {
            if elapsed > 0. { self.solvers[i].borrow_mut().latch_active_spot_cutoff(); }
        }
        for &(i, elapsed) in advanced.iter() {
            if elapsed > 0. {
                self.solvers[i]
                    .borrow_mut()
                    .step_field_conditions(elapsed, Some(self));
            }
        }
        self.update();
        self.flush_skill_effects();
        self.cache_skill_activations();
    }
}
