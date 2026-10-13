# Architecture and simulation order

## Entry point and data

[`main.rs`](../src/main.rs) loads the embedded course and skill tables, reads
one JSON object per line, parses runners, builds a `RaceField`, runs it, and
serializes observations. The request parser shares the preceding `skillData`
tree when its incoming text matches exactly. Changed text is parsed normally;
decoded skill definitions are reused only when the complete parsed table
matches, including floating-point bits. Changed definitions are decoded before
installation; absent `skillData` selects the bundled table. Embedded course and parameter JSON is parsed once,
and immutable course geometry is reused by course identity. Each request has
its own seed and runner state. ASCII request tokens are read directly from the
input buffer; Unicode input uses the character-aware parser.

Tick, rank and lane buffers reuse their storage. Running-style scoring groups
are prepared once, duel proximity entries are updated in place, and candidate
selection avoids temporary vectors. Blocking checks iterate over
runner state directly, preserving field order and the distinction between
cached and live positions. Skill conditions borrow immutable race parameters
during construction. Overtaking queries defer blocker scans until gap and
speed checks establish that blocker state can affect the decision.

[`race_solver_builder.rs`](../src/race_solver_builder.rs) converts raw stats,
race conditions and skill definitions into one solver per runner.
[`race_field.rs`](../src/race_field.rs) supplies roster facts, ranks, gaps,
nearby runners, recipients and coordinated updates.
[`race_solver.rs`](../src/race_solver.rs) owns each runner's movement, HP,
timers, conditions, active effects and competition state.

## Two execution paths

The default path uses the conventional solver, predominantly `f64`, Rule30
randomness and its own pacing, lane and last-spurt calculations. It remains
the path used when `experimentalRetailRuntime` is omitted.

The reconstructed path enables `experimentalRetailRuntime`. It uses shared
retail-style randomness, mixed-precision physics, cached state and ordered field
passes through [`race_solver_retail.rs`](../src/race_solver_retail.rs) and
[`race_field_retail.rs`](../src/race_field_retail.rs). Separate switches select
startup, geometry and section behavior. These are not interchangeable with the
default solver. The examples deliberately name the selected path.

For the benchmark profile use `examples/v72-race.jsonl`. Its switches do
not enable the separate startup option. The reconstructed field tick proceeds in this order:

1. Collect duel proximity, geometry width and lane caches
   from the current field.
2. Prepare each runner's tick, including runtime state and effect expiry.
3. Process skill priority 1 for the field, then priority 0; dispatch effects.
4. Refresh front blockers and blocking state.
5. Advance each runner longitudinally in the supplied order and immediately
   assign that runner's overtaking timers from the partially updated field.
6. Resolve lateral overlaps, prepare new duel participants, then latch active
   Spot Struggle cutoffs before evaluating field-condition exits.
7. Update eligible runners' field conditions using their effective elapsed time.
8. Refresh the field, flush effects and publish activation and healer history
   for the next tick.

The default field also separates longitudinal, lateral and field-condition
passes. Do not replace either manager with “fully update runner A, then B”:
that exposes mixed-time positions and changes blocking and activation results.

## Clock, observations and completion

Simulation uses a 0.0666-second step, not exactly 1/15 second. The reconstructed
path keeps float32 clock/state behavior; startup may perform a zero-time pass.
Start delays can leave only part of a tick available for movement. Finish
crossings are interpolated between adjacent simulation ticks.

`main.rs` records compact numeric snapshots and events. Selected frames are
serialized directly from contiguous numeric runner samples, while retaining
dense finish samples. Normally the
first 17 effective frames, every 16th frame and the final frame are retained;
the effective index accounts for a synthetic gate frame the original server
may prepend. Per-runner finish samples are preserved separately.
`retainAllFrames: true` disables thinning for analysis.

This binary produces simulation JSON. Encoding the game's `race_scenario`,
persisting results, granting rewards, and playback/UI are outside this package.
Post-finish movement is distinct from the recorded finish time and rank.

## Numerical reproducibility

Keep the input JSON, runtime settings, engine version, data and platform
consistent when comparing runs. Matching the seed alone is
insufficient. A change in runner order, a conditional random draw, skill table,
sort tie or float32 rounding can alter subsequent outcomes.

The default solver uses compensated sums for some additive effects. The
reconstructed physics uses ordered float32 receivers. Its `root` evaluates a
double square root then casts back to float32. Reassociating expressions or
changing receiver order can change a branch even when the equations look
algebraically identical. See the bit-vector tests bundled with the source.
