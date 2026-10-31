# Source reference

Map of the current production source. Start with the [architecture](ARCHITECTURE.md)
for execution order, [mathematics](MATHEMATICS.md) for equations and
[field mechanics](FIELD-MECHANICS.md) for runner interactions.

## Interfaces and execution

| Module | Responsibility |
|---|---|
| [main.rs](../src/main.rs) | Command-line entry point and process exit handling. |
| [lib.rs](../src/lib.rs) | Request parsing, shared data, field construction, output metadata and worker execution. |
| [api.rs](../src/api.rs) | PreparedRace setup and fresh per-seed state; owned Simulation results. |
| [ffi.rs](../src/ffi.rs) | Thread-confined C handles, packet buffers, ownership and recoverable errors. |
| [seed_sampling.rs](../src/seed_sampling.rs) | Validated seed batches, prepared setup reuse, persistent threads and ordered replies. |
| [geometry_registry.rs](../src/geometry_registry.rs) | Registration and lookup of separately supplied course geometry. |
| [json.rs](../src/json.rs) | JSON parsing, numeric representation and serialization. |

## Runners, fields and skills

| Module | Responsibility |
|---|---|
| [race_solver_builder.rs](../src/race_solver_builder.rs) | Runner stats, Career bonuses, skill definitions, triggers and effect values. |
| [race_solver.rs](../src/race_solver.rs) | Runner state, skill lifecycle, conventional movement and competition decisions. |
| [race_field.rs](../src/race_field.rs) | Roster facts, ranks, recipients, pacemaker selection and coordinated updates. |
| [race_solver_retail.rs](../src/race_solver_retail.rs) | Detailed runtime updates, precision-sensitive movement and integration. |
| [race_field_retail.rs](../src/race_field_retail.rs) | Ordered detailed field passes, cached/live observations and overlap resolution. |
| [horse_types.rs](../src/horse_types.rs) | Runner attributes, strategies, aptitudes and shared types. |
| [condition_parser.rs](../src/condition_parser.rs) | Condition-expression parsing, composition and immutable expression caches. |
| [activation_conditions.rs](../src/activation_conditions.rs) | Static/dynamic predicates, stateful observations and sampled conditions. |
| [activation_sample_policy.rs](../src/activation_sample_policy.rs) | Distance windows and lottery policies for sampled activation. |
| [region.rs](../src/region.rs) | Eligible distance intervals and region operations. |

## Physics and field mechanics

| Module | Responsibility |
|---|---|
| [retail_initialization.rs](../src/retail_initialization.rs) | Ordered seeded initialization, start delays, skill lotteries and opening values. |
| [retail_physics.rs](../src/retail_physics.rs) | Detailed speed, acceleration, HP, blocking and mixed-precision formulas. |
| [retail_lane.rs](../src/retail_lane.rs) | Visibility, cached/live passing targets, body clearance, route ties and lateral movement. |
| [retail_position_keep.rs](../src/retail_position_keep.rs) | Detailed pacing modes, selected targets, gaps and check timers. |
| [retail_spurt.rs](../src/retail_spurt.rs) | Last-spurt candidate construction, affordability, sorting and retries. |
| [retail_states.rs](../src/retail_states.rs) | Rushing and downhill states with their ordered random draws. |
| [retail_geometry.rs](../src/retail_geometry.rs) | Course transforms, interpolation and world-distance correction. |
| [retail_geometry_assets.rs](../src/retail_geometry_assets.rs) | Embedded course-geometry asset selection. |
| [retail_random.rs](../src/retail_random.rs) | Detailed-runtime random streams and integer/floating draw paths. |
| [retail_sort.rs](../src/retail_sort.rs) | Comparison sorting and tie behavior used by the detailed runtime. |
| [hp_policy.rs](../src/hp_policy.rs) | Stamina policy, consumption, recovery and conventional spurt planning. |
| [lane_selection.rs](../src/lane_selection.rs) | Conventional lane selection and lateral calculations. |
| [overtaking.rs](../src/overtaking.rs) | Overtake targets, timing and recorded passes. |
| [rushing.rs](../src/rushing.rs) | Conventional rushing state and duration decisions. |
| [conserve_power.rs](../src/conserve_power.rs) | Supported accumulation/release ledger and fallback coverage. |
| [random_gen.rs](../src/random_gen.rs) | Conventional Rule30 random generator and derived streams. |
| [starting_grid.rs](../src/starting_grid.rs) | Stall coordinates, gate identity and custom grid layout. |
| [course_data.rs](../src/course_data.rs) | Course metadata, conditions and speed-threshold modifiers. |
| [race_parameters.rs](../src/race_parameters.rs) | Parameter loading and typed runtime settings. |
| [stadium_score.rs](../src/stadium_score.rs) | Recorded leading-style and rushing observations. |

## Replay output

| Module | Responsibility |
|---|---|
| [replay_frames.rs](../src/replay_frames.rs) | Contiguous numeric frames, thinning, finish samples and runner/blocker remapping. |
| [binary_replay.rs](../src/binary_replay.rs) | Lossless GSIMRP01 packet encoding. |
| [replay_worker.rs](../src/replay_worker.rs) | Optional JSON encoding helper for eligible full-frame requests. |

## Current accuracy boundaries

- `RaceField::update_pacemaker` retains the incumbent until normal selection or restoration replaces it.
- `PassingSearch` keeps cached target identity separate from live obstacle routes and evaluates exact-cost ties with body clearance and obstacle-side direction.
- The inward corridor calculation stores each complete shifted bound as float32 before comparison.
- Prepared skill reuse preserves fresh mutable preconditions and observers for every seed.

## Tools and bindings

| Tool | Purpose |
|---|---|
| [native_engine.py](../tools/native_engine.py) | Python binding for the prepared native API. |
| [replay_protocol.py](../tools/replay_protocol.py) | Binary packet reading, frame views and output comparison. |
| [check_accuracy_regressions.py](../tools/check_accuracy_regressions.py) | Strict official-fixture comparison. |
| [check_binary_accuracy.py](../tools/check_binary_accuracy.py) | Binary replay observation checking. |
| [audit_validation_coverage.py](../tools/audit_validation_coverage.py) | Fixture observation and branch coverage inventory. |
| [validation_inputs.py](../tools/validation_inputs.py) | Captured input preflight and integrity checks. |
| [validation_contract.py](../tools/validation_contract.py) | Input-aware output structure and consistency checks. |
| [validation_runner.py](../tools/validation_runner.py) | Bounded engine execution and recovery. |
| [check_worker_parity.py](../tools/check_worker_parity.py) | Persistent worker comparisons and isolation. |
| [check_replay_protocol.py](../tools/check_replay_protocol.py) | JSON/binary agreement and protocol checks. |
| [check_seed_sampling.py](../tools/check_seed_sampling.py) | Seed batches, output modes, concurrency and error handling. |
| [check_native_parity.py](../tools/check_native_parity.py) | Native/worker agreement, ownership and handle isolation. |
| [check_career_team_rank.py](../tools/check_career_team_rank.py) | Synthetic team-rank equivalence and validation. |
| [build_pgo.py](../tools/build_pgo.py) | Profile-guided compiler training and builds. |
| [train_native.py](../tools/train_native.py) | Prepared-library profile training. |
| [benchmark.py](../tools/benchmark.py) | JSON worker latency. |
| [benchmark_replay.py](../tools/benchmark_replay.py) | Paired JSON/binary worker latency. |
| [benchmark_native.py](../tools/benchmark_native.py) | Prepared Python binding and worker latency. |
| [benchmark_throughput.py](../tools/benchmark_throughput.py) | Independent worker throughput. |

The [C header](../include/globalsim.h) defines exported handles and pointer
lifetimes. [prepared_race.rs](../examples/prepared_race.rs) demonstrates Rust
integration; [benchmark_prepared.rs](../examples/benchmark_prepared.rs)
measures owned numeric frames or binary packets.

## Data and regression cases

- [Course definitions](../data/course_data.json) and [course geometry](../data/course_geometry).
- [Runtime parameters](../data/global_parameters.json) and [parameter reference](PARAMETERS.md).
- [Validated-profile skill table](../data/v70-skill-data.json) and [legacy fallback](../data/skill_data.json).
- [Starting grids](../data/starting_grids.json).
- [Official regression fixtures](../tests/fixtures/accuracy) and [test guide](TESTING.md).

Use [v76-race.jsonl](../examples/v76-race.jsonl) for a complete request with
the validated runtime settings and skill table. File-version names identify
data snapshots; they are separate from the engine release version.
