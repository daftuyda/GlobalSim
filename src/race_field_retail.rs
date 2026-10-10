//! Experimental whole-field order. Ports the reconstructed Python manager;
//! matching it does not establish official-game parity.
use super::*;
use crate::retail_lane::{self, Row};

impl RaceField {
    pub(super) fn step_all_retail(&self, dt: f32, indices: &[usize]) {
        let rows: Vec<_> = self
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
            .collect();
        for &i in indices {
            let mut s = self.solvers[i].borrow_mut();
            s.collect_retail_duel_proximity(&rows, dt);
            let position = s.pos as f32;
            let r = s.retail.as_mut().unwrap();
            if let Some(geometry) = &r.config.geometry {
                r.lane_max = geometry.lane_max_at(position);
            }
            let context = s.retail_lane_context();
            let mut others = s.retail_other_horses(Some(self));
            // The pre-update cache stores raw target speed; the later live
            // passing query includes current-speed receivers.
            for (other, j) in others.iter_mut().zip((0..self.n).filter(|&j| j != i)) {
                other.target = self.solvers[j].borrow().target_speed as f32;
            }
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
        let mut advanced = Vec::with_capacity(indices.len());
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
        let mut rows: Vec<_> = self
            .solvers
            .iter()
            .map(|s| {
                let s = s.borrow();
                Row {
                    position: s.pos as f32,
                    lane: s.lane as f32,
                }
            })
            .collect();
        let mut flags: Vec<_> = self
            .solvers
            .iter()
            .map(|s| s.borrow().retail.as_ref().unwrap().lane.force_check)
            .collect();
        retail_lane::resolve_overlaps(&mut rows, &mut flags);
        for (i, row) in rows.iter().enumerate() {
            let mut s = self.solvers[i].borrow_mut();
            s.lane = row.lane as f64;
            s.retail.as_mut().unwrap().lane.force_check = flags[i];
        }
        // Isolated hypothesis: commit new duel participants before evaluating
        // exits, so a same-tick join can sustain its existing partners.
        for &(i, elapsed) in &advanced {
            if elapsed > 0. {
                self.solvers[i].borrow_mut().prepare_retail_duel_start(elapsed, self);
            }
        }
        for &(i, elapsed) in &advanced {
            if elapsed > 0. { self.solvers[i].borrow_mut().latch_active_spot_cutoff(); }
        }
        for (i, elapsed) in advanced {
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
