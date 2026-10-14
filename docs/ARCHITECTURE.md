# Architecture and simulation order

## Entry point and data

[`lib.rs`](../src/lib.rs) loads the embedded course and skill tables, reads
one JSON object per line, parses runners, builds a `RaceField`, runs it, and
serializes observations. The request parser shares the preceding `skillData`
tree when its incoming text matches exactly. Changed text is parsed normally;
decoded skill definitions are reused only when the complete parsed table
matches, including floating-point bits. Changed definitions are decoded before
installation; absent `skillData` selects the bundled table. Embedded course and parameter JSON is parsed once,
and immutable course geometry is reused by course identity. Geometry stores
precomputed interpolation differences and quaternion sign choices; each query
retains the original interpolation, normalization and rounding order. Each request has
its own seed and runner state. ASCII request tokens are read directly from the
input buffer; Unicode input uses the character-aware parser.

Runner construction shares a request-local prepared roster containing style
counts, skill and character membership, favorite-style information and team stat
sums. Self exclusions and team sums retain their original runner order and
numeric rounding. This data is rebuilt for each request; each seed has its own
mutable simulation state.

Tick, rank and lane buffers reuse their storage. Rank-band thresholds are
calculated once from the fixed field size, retaining the original rounded cutoffs. Running-style scoring groups
are prepared once, duel proximity entries are updated in place, and candidate
selection avoids temporary vectors. After skills run, a shared field snapshot
supplies front blockers and side-blocked timers in one pass per runner. Front
blocking includes finishers; side-blocked timers exclude them. Other queries keep
their original observation point and distinction between cached and live positions. Skill conditions borrow immutable race parameters
during construction. Overtaking queries defer blocker scans until gap and
speed checks establish that blocker state can affect the decision. Reordering
runners by gate moves their parsed records without copying the runner trees.

Condition parsing shares immutable expression trees in a per-thread cache of
up to 4,096 expressions. Cache identity includes the exact expression, condition
table and runtime mode. Each application supplies its own sampled windows and
constructs fresh predicates, prerequisite latches and activation observers.
Already sampled windows pass directly into runner construction.

During retail runner construction, pre-sampled triggers are indexed by skill ID,
alternative index and condition or precondition column. Each lookup supplies
fresh sample records to the parser in their original expression-node order.
Missing, mismatched and extra-node validation is unchanged. This index belongs
to one runner's construction and is not shared across seeds.

Condition predicates retain their evaluation order, including continuity and
HP-transition observations during cooldown. Prerequisite latches and activation
observers are updated in their existing passes. Running
skills also track whether additional activations are pending, avoiding repeated
scans when there is nothing to apply.

Pending skills have separate indexes for each priority pass and for unsatisfied
prerequisite latches. Entries retain the full scan's visitation order, and indexes
are rebuilt after removals. Completing a prerequisite removes it from the watch
list. Compact trigger metadata dismisses inactive details without reading their
full skill records. Stateful observers, random reservations, removals and pending
additional effects retain the full path. Skipped details still advance their turn
in the ordered additional-effect pass. Priority-zero bookkeeping runs even when
its candidate list is empty.

Each runner caches its base minimum speed by exact float32 guts bits. The
acceleration base is cached by every input that affects it and by physics-parameter
identity; start-dash additions and ordered skill modifiers remain live. The HP
policy caches its initialization-time guts factor and refreshes it on
reinitialization. Front-blocker reductions remain live calculations. Idle pacing
controllers skip peer-context construction once both position-keep windows have
ended, while active modes and rushing transitions follow the full update.

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

For the benchmark profile use `examples/v73-race.jsonl`. Its switches do
not enable the separate startup option. The reconstructed field tick proceeds in this order:

1. Capture the pre-movement roster, then collect duel proximity, geometry width
   and lane caches. Each lane cache excludes its owner while retaining roster
   order and cached visibility.
2. Prepare each runner's tick, including runtime state and effect expiry.
3. Process skill priority 1 for the field, then priority 0; dispatch effects.
4. Capture the post-skill roster and refresh front blockers and blocking state.
5. Advance each runner in the supplied order, replace its live roster entry and
   immediately assign overtaking timers. Reverse passing-target membership uses
   bit masks for fields of up to 128 runners, with direct scans for larger fields.
   Membership follows the same ordered movement pass. Callbacks or effects that
   can mutate peers refresh the complete roster and passing index. Later runners
   see earlier runners' updates within the tick.
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

`lib.rs` records numeric snapshots and events. Synchronous capture stores all
timestamps and runner samples in two contiguous buffers. Sparse selection uses
indices into those buffers and retains dense finish samples. Selected frames
can be serialized as JSON or emitted as exact binary records. The frame serializer reuses exact formatted lane and speed
text only when the floating-point bits match the preceding value for that runner;
this cache lasts for one frame array. Common integer fields and supported widened
float32 values use specialized formatting with the same JSON text as the general
formatter. Other numbers retain the general path.

The worker reuses its response buffer across requests, replacing its contents
for every reply. Buffers larger than 16 MiB are released after writing. Normally the
first 17 effective frames, every 16th frame and the final frame are retained;
the effective index accounts for a synthetic gate frame the original server
may prepend. Per-runner finish samples are preserved separately.
`retainAllFrames: true` disables thinning for analysis.

With JSON output and `GLOBALSIM_REPLAY_WORKER=1`, eligible full-frame requests send completed
snapshots to a persistent encoding thread in batches of 128. Its bounded input
channel holds up to four messages. The helper uses the same frame serializer
and returns a complete typed replay array before the response is emitted.
Simulation, random draws and response order stay on the main worker thread.
Requests requiring gate remapping and requests using sparse frames retain the
synchronous path. The helper's replay buffer retains at most 16 MiB between
requests, separately from the main response buffer. The default worker uses
one thread.

`GLOBALSIM_OUTPUT=binary-v1` writes length-prefixed metadata and little-endian
float64 frame records on one thread. JSON remains the default. The optional
`fast-alloc` Cargo feature selects mimalloc; the normal build uses the platform
allocator. Neither option changes simulation rules.

This binary produces simulation results and replay observations. Encoding the game's `race_scenario`,
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

## Prepared library execution

The CLI entry point in [main.rs](../src/main.rs) calls `run_worker` in
[lib.rs](../src/lib.rs). [api.rs](../src/api.rs) exposes `PreparedRace` and owned
`Simulation` results. It reuses parsed inputs, immutable skill tables, geometry,
roster facts and decoded initialization rules across seeds. Each run creates
fresh RNG streams, sampled triggers, mutable solvers, timers and observers.

Prepared skill tables are installed in a thread-local scope for the duration of
the run and restored afterward. The worker retains its existing installed-table
path. [ffi.rs](../src/ffi.rs) exports thread-confined C handles and length-delimited
binary replies. Numeric Rust views borrow owned results; the Python wrapper
copies native reply bytes before another run can reuse the buffer.
See [Prepared API](IN-PROCESS.md) for lifetimes, threading and usage.
