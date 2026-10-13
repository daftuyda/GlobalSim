//! Experimental whole-field order. Ports the reconstructed Python manager;
//! matching it does not establish official-game parity.
use super::*;
use crate::retail_lane::{self, Row};

#[derive(Default)]
pub(super) struct TickBuffers {
    before: Vec<(f32, f32, bool)>,
    others: Vec<retail_lane::Horse>,
    advanced: Vec<(usize, f64)>,
    overlaps: Vec<Row>,
    flags: Vec<bool>,
    overlap_order: Vec<usize>,
}

impl RaceField {
    pub(super) fn step_all_retail(&self, dt: f32, indices: &[usize]) {
        let mut buffers = self.retail_tick.borrow_mut();
        let TickBuffers { before: rows, others, advanced, overlaps, flags, overlap_order } = &mut *buffers;
        rows.clear();
        rows.extend(self
            .solvers
            .iter()
            .map(|s| {
                let s = s.borrow();
                (
                    s.pos as f32,
                    s.lane as f32,
                    !s.course.corners.is_empty() && s.course
                        .straights
                        .last()
                        .is_some_and(|r| r.start <= s.pos && s.pos < r.end),
                )
            })
            );
        for &i in indices {
            let mut s = self.solvers[i].borrow_mut();
            s.collect_retail_duel_proximity(&rows, dt);
            let position = s.pos as f32;
            let r = s.retail.as_mut().unwrap();
            if let Some(geometry) = &r.config.geometry {
                r.lane_max = geometry.lane_max_at(position);
            }
            let context = s.retail_lane_context();
            // The pre-update cache stores raw target speed. Sample it directly;
            // current-speed receivers are only needed by the later live query.
            others.clear();
            others.extend(self.solvers.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, cell)| {
                let other = cell.borrow();
                retail_lane::Horse {
                    point: Row { position: other.pos as f32, lane: other.lane as f32 },
                    speed: other.retail.as_ref().map_or(other.current_speed as f32, |r| r.observed_speed),
                    target: other.target_speed as f32,
                }
            }));
            let r = s.retail.as_mut().unwrap();
            r.lane.collect(&context, &others, r.config.lane);
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
        for &i in indices {
            let mut s = self.solvers[i].borrow_mut();
            s.retail.as_mut().unwrap().front_blocker = self
                .front_blocker(i, s.pos, s.lane)
                .map(|(j, gap)| (j, gap as f32));
            s.update_blocking(dt as f64, Some(self));
        }
        advanced.clear();
        advanced.reserve(indices.len());
        for &i in indices {
            let elapsed = self.solvers[i]
                .borrow_mut()
                .step_forward(dt as f64, Some(self));
            advanced.push((i, elapsed));
            let overtake_times = self.retail_overtake_times_for(i, dt);
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
