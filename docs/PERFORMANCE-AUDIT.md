# Performance audit

The prepared API removes repeated input decoding and process transport when
many seeds share one race setup. Direct Rust calls average 4.74 ms with every
frame retained, or 4.81 ms including binary packet encoding. The paired Python
binding result is 5.72 ms versus 5.85 ms for the current binary worker.
[Performance scopes and measurements](PERFORMANCE.md).

## Prepared execution

`PreparedRace` owns decoded skill definitions, parsed race inputs, geometry and
seed-independent initialization rules. Roster facts are reused after the first
run. Initialization still samples every trigger and performs every lottery in
the same field-wide pass order for each seed. Solver conditions, timers,
observers, mutable runners, RNG streams and results are rebuilt independently.

Skill tables are scoped to the calling thread during a prepared run, so handles
with different definitions can be interleaved or run on separate threads without
overwriting one another. Returned Rust results own their frames. The C API
exposes a complete binary packet; the Python binding copies it into owned bytes.

The command-line worker keeps its existing request/response contract. Its
latency was roughly unchanged by this refactor. The direct API requires caller
integration and amortizing preparation across seeds. It is useful for Monte Carlo
workloads; preparing anew for every seed forfeits that benefit.

This implements the previously identified direct-call and reusable-input
approach. Runner/condition construction is still performed per seed, so further
immutable preparation may be possible, provided sampled windows, green effects,
observers and random draw ordering stay independent. That further speedup has
not been measured.

## Allocation and transport experiments

The earlier paired output-format comparison reduced full-frame Monte Carlo
worker latency from 8.89 ms to 7.95 ms with JSON, or 6.25 ms with binary output.
It evaluated the storage and allocator changes below. Those values belong to
that comparison, not the newer direct-API timing harness.
[Output-format measurements](../evidence/architecture-performance.json).

## Approaches evaluated

| Approach | Evidence | Outcome |
|---|---|---|
| Contiguous replay storage | Generic-build paired comparison | Roughly neutral for JSON latency; removes per-frame allocation and boxing and supports direct numeric output. |
| Lossless binary replay | All metadata and every frame scalar checked | About 0.54 MB instead of 1.57 MB for the Monte Carlo reply; implemented as an optional interface. |
| Faster allocator | Same-source generic comparisons | About 7–10% lower JSON latency across the measured workloads; implemented as `fast-alloc`. |
| Performance-core affinity | Paired pinned/unpinned workers | Small, workload-dependent improvement; no automatic affinity policy added. |
| Borrowed cooldown keys and reusable pacemaker list | Separate PGO candidate | Reduced allocation sites but did not reliably lower the full-frame Monte Carlo binary time; remains an experiment. |
| Multiple independent workers | Complete full-frame batches | About 663 races/second with four workers and 1,087 with eight; individual latency is reported separately. |

[Storage experiment](../evidence/architecture-storage-experiment.json),
[allocator experiment](../evidence/architecture-allocator-experiment.json),
[affinity experiment](../evidence/architecture-affinity-experiment.json),
[cooldown/list experiment](../evidence/architecture-memory-experiment.json),
[batch results](../evidence/architecture-throughput.json).

## Allocation profile

Instrumented system-allocator builds counted allocations and reallocations over
30 warmed nine-runner Monte Carlo requests. Counts describe memory operations,
not elapsed-time shares or peak resident memory. The first request is excluded.

| Phase | Per-frame storage | Contiguous storage |
|---|---:|---:|
| Parse request and settings | 2,137 | 2,137 |
| Seeded initialization | 2,693 | 2,693 |
| Build runners and conditions | 4,593 | 4,593 |
| Set up runtime | 754 | 754 |
| Simulation ticks and capture | 11,225 | 10,083 |
| Results and cleanup | 3,871 | 2,718 |
| JSON output and cleanup | 54 | 54 |

Contiguous storage removes about 2,300 allocations per race. Its growing buffers
can request more cumulative bytes during a first allocation sequence; fewer
allocations alone do not prove lower memory use or lower latency. Binary output
also avoids the per-runner numeric text caches needed by JSON.
[Allocation measurements](../evidence/architecture-allocations.json).

A representative seed attributed most remaining tick allocations to runner
preparation, skills and rank/pacemaker handling. Cooldown key cloning and the
pacemaker's temporary list were tested separately. Their removal did not produce
a reliable Monte Carlo latency gain after enabling the faster allocator, which
is why the timing result determines selection rather than allocation count alone.

## Computation versus transport

The reference JSON phase profile used the system allocator and per-frame
storage. It separates the main costs for the same full-frame Monte Carlo workload:

| Phase | Reference worker time |
|---|---:|
| Request parsing | 0.084 ms |
| Settings and geometry | 0.044 ms |
| Seeded initialization | 0.259 ms |
| Runner and condition construction | 0.620 ms |
| Field/runtime setup | 0.065 ms |
| Ticks, including numeric frame capture | 4.670 ms |
| Results | 0.111 ms |
| Frame selection/wrapping | 0.028 ms |
| JSON serialization | 0.844 ms |
| Writing and flushing | 0.972 ms |
| Reply cleanup | 0.172 ms |

These are reference measurements, not a fresh phase profile of the fast allocator
or binary output. Timers omit some caller-visible transfer and bookkeeping costs
and add instrumentation overhead; their sum is not a substitute for complete
request latency. [Reference phase measurements](../evidence/latency-profile.json).

Within sampled reference field work, movement/overtaking accounted for about
42%, conditions/skills 23%, preparation 10%, caches 9%, and the remaining field
coordination about 16%. These are sampled field shares rather than shares of the
whole request. There is no single arithmetic function consuming most race time.

## Language and architecture choices

**A wholesale language rewrite is not currently supported by the measurements.**
Rust already exposes LLVM optimization, vectorization, LTO, CPU targeting and
profile-guided code generation. Changing to C++ or another native language may
help a particular kernel if it produces better machine code, but it does not
automatically remove allocations, repeated preparation or JSON/pipe work.
A controlled kernel comparison would be needed before assigning a speedup to a
language change. [Rust compiler options](https://doc.rust-lang.org/rustc/codegen-options/index.html).

Further architectural candidates include:

1. **More reusable runner construction.** The direct API already reuses input
   and initialization decoding. Determine which remaining builder calculations
   can safely be shared across seeds before adding another cache.
2. **Compact hot state and condition evaluation.** Group frequently accessed
   fields and investigate indexed skill state or a compact condition program.
   Measure cache behavior and dynamic-call costs before undertaking a broad rewrite.
3. **SIMD across independent work.** Independent seed lanes or coordinate
   operations are candidates. Floating-point reductions and same-race update
   dependencies must retain their original order; unrestricted fast-math can
   change results. [LLVM vectorization](https://llvm.org/docs/Vectorizers.html).
4. **GPU batches.** Large batches could be explored, but the branch-heavy skill
   system, ordered runner interactions and transfer of complete replays require
   substantial redesign. A GPU speedup for one nine-runner race has not been
   demonstrated. NVIDIA recommends minimizing host/device transfers and batching
   small transfers. [CUDA guidance](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/).

The engine retains its timestep, event order and precision. The current changes
do not trade accuracy for speed or claim new official-game parity.

## Validation

All 13,187 captured races pass complete JSON byte comparison for the generic,
fast-allocator PGO and PGO-helper configurations. Binary PGO replies preserve all
metadata and every frame scalar, accounting for JSON's zero/null normalization.
The prepared library passes the same full-frame corpus comparison. The captured official metrics also remain unchanged. The Rust suite has
118 passing tests and the same 11 pre-existing failures. Binary protocol tests
cover exact float bits, selected frames, errors, truncation and partial reads.

[Full validation](../evidence/prepared-validation.json),
[protocol checks](../evidence/prepared-protocol-check.json),
[build settings](../evidence/prepared-build.json), [testing](TESTING.md).

The current optimized build also reproduces all 17,594 recently audited race
outputs after the competition corrections. The paired numeric-frame benchmark
remains 4.68 ms before and after those corrections.
[Accuracy validation](../evidence/fidelity-validation-20261014.json),
[paired timings](../evidence/fidelity-performance.json).
