//! Reconstructed HorseLastSpurtCalculator (340c5c0/340ca00/340cc30).
//! The 30m search, two independent retry limits, comparator and RNG are part
//! of the behavior: do not substitute the normal solver's algebraic estimate.
use crate::retail_random::RandomSource;
use crate::retail_sort::comparison_sort;

#[derive(Debug, Clone, Copy)]
pub struct Plan {
    pub start: f32,
    pub speed: f32,
    pub result: i32,
}

pub struct Input {
    pub distance: f32,
    pub position: f32,
    pub hp: f32,
    pub max_speed: f32,
    pub base_speed: f32,
    pub wisdom: f32,
}

#[derive(Clone, Copy)]
pub struct SearchParameters {
    pub buffer: f32,
    pub latest_from_goal: f32,
    pub speed_delta: f32,
}
impl Default for SearchParameters {
    fn default() -> Self {
        Self {
            buffer: 60.,
            latest_from_goal: 100.,
            speed_delta: 0.1,
        }
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    start: f32,
    speed: f32,
    time: f32,
}

fn sort_candidates(candidates: &mut [Candidate]) {
    // The comparator truncates the difference, not the individual times.
    comparison_sort(candidates, |a, b| {
        (((a.time - b.time) * 10000.) as i32).cmp(&0)
    });
}

pub fn choose(
    input: Input,
    parameters: SearchParameters,
    consumption: impl Fn(f32) -> f32,
    rng: &mut impl RandomSource,
) -> Plan {
    let remaining = ((input.distance - input.position) - parameters.buffer).max(0.);
    let base_cost = consumption(input.base_speed);
    let finish = |start, speed, result| Plan {
        start,
        speed,
        result,
    };
    if input.hp < (remaining / input.base_speed) * base_cost {
        return finish(input.distance, input.max_speed, 4);
    }
    if input.hp >= ((remaining as f64 / input.max_speed as f64) * consumption(input.max_speed) as f64) as f32 {
        return finish(input.position, input.max_speed, 1);
    }
    let latest = input.distance - parameters.latest_from_goal;
    let candidate = |speed: f32| {
        let cost = consumption(speed);
        let mut sprint_distance = remaining;
        while sprint_distance > 0. {
            let slow_time = (remaining - sprint_distance) / input.base_speed;
            let fast_time = sprint_distance / speed;
            // Store the normal-pace cost as a float before adding the wider
            // sprint cost. Rounding both products, or neither product, changes
            // feasibility at captured HP boundaries and can select a different
            // 30 m plan. Keep the final sum and comparison at double precision.
            let slow_cost = slow_time * base_cost;
            let needed = slow_cost as f64 + fast_time as f64 * cost as f64;
            let start = (input.distance - parameters.buffer) - sprint_distance;
            if input.hp as f64 >= needed && start < latest {
                return Some(Candidate {
                    start,
                    speed,
                    time: slow_time + fast_time,
                });
            }
            sprint_distance -= 30.;
        }
        None
    };
    let lower = input.base_speed.min(input.max_speed);
    let mut speed = input.max_speed;
    let mut found = false;
    for _ in 0..101 {
        if candidate(speed).is_some() {
            found = true;
            break;
        }
        speed -= parameters.speed_delta;
        if speed <= lower {
            return finish(input.distance, input.max_speed, 8);
        }
    }
    if !found {
        return finish(input.distance, input.max_speed, 8);
    }
    let mut candidates = Vec::new();
    for _ in 0..101 {
        if let Some(row) = candidate(speed) {
            candidates.push(row);
        }
        speed -= parameters.speed_delta;
        if speed <= lower {
            break;
        }
    }
    if candidates.is_empty() {
        return finish(input.distance, input.max_speed, 8);
    }
    sort_candidates(&mut candidates);
    let chance = 15. + 0.05f32 * input.wisdom;
    for candidate in &candidates {
        if (rng.integer(0, 100) as f32) < chance {
            return finish(input.position.max(candidate.start), candidate.speed, 2);
        }
    }
    let last = candidates.last().unwrap();
    finish(input.position.max(last.start), last.speed, 2)
}

pub struct SpurtState {
    pub pending: bool,
    pub count: usize,
    pub start: f32,
    pub speed: f32,
    pub result: i32,
}
impl Default for SpurtState {
    fn default() -> Self {
        Self {
            pending: false,
            count: 0,
            start: -1.,
            speed: 0.,
            result: 8,
        }
    }
}
impl SpurtState {
    pub fn update(&mut self, position: f32, calculate: impl FnOnce() -> Plan) -> bool {
        if self.count == 0 || self.pending {
            let plan = calculate();
            self.count += 1;
            if self.count == 1 || matches!(plan.result, 1 | 2) {
                self.result = plan.result;
                self.speed = plan.speed;
                if self.count == 1 || self.start < 0. {
                    self.start = if matches!(plan.result, 1 | 2) {
                        plan.start
                    } else {
                        -1.
                    };
                }
            }
            self.pending = false;
        }
        matches!(self.result, 1 | 2) && self.start > 0. && position >= self.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retail_physics::Parameters;
    use crate::retail_random::RetailRandom;

    #[test]
    fn search_matches_python_plans_and_shared_draw_counts() {
        let raw = crate::json::parse(include_str!("../data/global_parameters.json"))
        .unwrap();
        let physics = Parameters::from_value(raw.get("parameters").unwrap()).unwrap();
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_physics_port.json"))
        .unwrap();
        for (i, row) in fixture
            .get_array("spurt_vectors")
            .unwrap()
            .iter()
            .enumerate()
        {
            let n = |key| row.get_f64(key).unwrap() as f32;
            let mut rng = RetailRandom::new(row.get_i64("seed").unwrap() as i32);
            let input = Input {
                distance: n("distance"),
                position: n("position"),
                hp: n("hp"),
                max_speed: n("maximum"),
                base_speed: n("base"),
                wisdom: n("wisdom"),
            };
            let plan = choose(
                input,
                SearchParameters::default(),
                |speed| {
                    physics.hp_per_second(
                        speed,
                        physics.course_speed(n("distance")),
                        n("ground"),
                        n("guts"),
                        2,
                        1.,
                    )
                },
                &mut rng,
            );
            assert_eq!(
                plan.start.to_bits(),
                row.get_i64("start_bits").unwrap() as u32,
                "start vector {i}"
            );
            assert_eq!(
                plan.speed.to_bits(),
                row.get_i64("speed_bits").unwrap() as u32,
                "speed vector {i}"
            );
            assert_eq!(
                plan.result,
                row.get_i64("result").unwrap() as i32,
                "result vector {i}"
            );
            assert_eq!(
                rng.draw_count,
                row.get_i64("draws").unwrap() as usize,
                "draws vector {i}"
            );
        }
    }

    #[test]
    fn recovery_retry_preserves_a_valid_start_and_failed_retry_keeps_plan() {
        let mut state = SpurtState::default();
        assert!(!state.update(1400., || Plan {
            start: 1600.,
            speed: 22.,
            result: 2
        }));
        state.pending = true;
        assert!(!state.update(1500., || Plan {
            start: 1500.,
            speed: 24.,
            result: 1
        }));
        assert_eq!((state.start, state.speed, state.count), (1600., 24., 2));
        state.pending = true;
        assert!(state.update(1700., || Plan {
            start: 2000.,
            speed: 30.,
            result: 4
        }));
        assert_eq!(
            (state.start, state.speed, state.result, state.count),
            (1600., 24., 1, 3)
        );
        assert!(state.update(1800., || panic!("unexpected repeated spurt calculation")));
    }

    #[test]
    fn real_first_pass_retry_limit_rejects_a_distant_feasible_speed_without_rolls() {
        let mut rng = RetailRandom::new(1);
        let plan = choose(
            Input {
                distance: 1000.,
                position: 0.,
                hp: 950.,
                max_speed: 50.,
                base_speed: 1.,
                wisdom: 1000.,
            },
            SearchParameters::default(),
            |speed| speed * speed,
            &mut rng,
        );
        assert_eq!(plan.result, 8);
        assert_eq!(rng.draw_count, 0);
    }

    #[test]
    fn comparator_near_ties_and_large_equal_partitions_keep_framework_behavior() {
        let mut rows = vec![
            Candidate {
                start: 0.,
                speed: 20.,
                time: 50.00011,
            },
            Candidate {
                start: 1.,
                speed: 21.,
                time: 50.00009,
            },
        ];
        sort_candidates(&mut rows);
        assert_eq!(
            rows.iter().map(|v| v.start as i32).collect::<Vec<_>>(),
            vec![0, 1]
        );
        rows.push(Candidate {
            start: 2.,
            speed: 22.,
            time: 49.9,
        });
        sort_candidates(&mut rows);
        assert_eq!(
            rows.iter().map(|v| v.start as i32).collect::<Vec<_>>(),
            vec![2, 1, 0]
        );
        let mut rows: Vec<_> = (0..17)
            .map(|i| Candidate {
                start: i as f32,
                speed: 20.,
                time: 50.,
            })
            .collect();
        sort_candidates(&mut rows);
        assert_eq!(
            rows.iter().map(|v| v.start as i32).collect::<Vec<_>>(),
            vec![0, 14, 13, 12, 11, 10, 9, 15, 8, 6, 5, 4, 3, 2, 1, 7, 16]
        );
    }

    #[test]
    fn second_pass_lottery_is_strict_and_has_its_own_retry_limit() {
        struct Reject {
            draws: usize,
        }
        impl RandomSource for Reject {
            fn integer(&mut self, lo: i32, hi: i32) -> i32 {
                assert_eq!((lo, hi), (0, 100));
                self.draws += 1;
                99
            }
            fn real(&mut self, _: f32, _: f32) -> f32 {
                panic!("Spurt must use integer lotteries")
            }
        }
        let mut rng = Reject { draws: 0 };
        let plan = choose(
            Input {
                distance: 2000.,
                position: 1300.,
                hp: 15000.,
                max_speed: 30.,
                base_speed: 13.,
                wisdom: 1000.,
            },
            SearchParameters::default(),
            |speed| speed * speed,
            &mut rng,
        );
        assert_eq!(plan.result, 2);
        assert_eq!(rng.draws, 101);
        assert!(plan.speed >= 19.999 && plan.speed <= 30.);
        assert!(plan.start < 1900.);
        assert!((plan.start - 1300.) * 13. + (1940. - plan.start) * plan.speed <= 15000.01);
    }
}
