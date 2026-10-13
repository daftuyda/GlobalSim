# Performance audit

The optimizations measured here reduce mean latency by **25% for standard nine-runner races**, **18% for dense nine-runner output**, and **11% for both eighteen-runner modes**, compared with the preceding optimized build. At this stage, all **13,187 captured races** retained identical output and official metrics.

Subsequent accuracy corrections resolve the four remaining differing races.
The tables below describe the optimization build; current correctness results
are in [Accuracy](ACCURACY.md), and the cost of the corrections is measured in
[Performance](PERFORMANCE.md).

## Changes implemented

- **Reuse parsed skill definitions.** Compare the exact incoming root-level `skillData` text with the cached value before parsing it again. A match shares the immutable parsed tree and makes decoded-table identity checking constant-time. Changed text uses the original token parser and full content comparison.
- **Encode whole replay frames.** Write directly from contiguous numeric runner arrays, removing the remaining per-runner boxes and per-frame generic maps. One scratch array handles requested runner order and blocker-index remapping across frames.
- **Update duel proximity in place.** Keep surviving timer entries, avoid cloning the map during target selection, and select nearby/eligible/ready candidates without temporary vectors.
- **Reuse remaining tick storage.** Precompute running-style groups, reuse lane-overlap ordering, score passing candidates as they are visited, and pass lane modifiers through iterators.

The request and response formats, timestep, RNG sequence, candidate tie order, field update order and floating-point operations remain unchanged. Whole-frame encoding is now part of the validated engine.

## Paired latency

The benchmark uses 30 captured races per field-size group, five warmups and five repetitions in persistent workers. Engine order alternates. It includes parsing, simulation, response encoding and pipe transfer; caller JSON encoding and decoding are excluded. These are release builds on an Intel Core i7-14700K with Windows and Rust 1.97.1.

| Workload | Previous optimized | Latest | Reduction | Latest p95 |
|---|---:|---:|---:|---:|
| 9 runners, standard | 20.62 ms | 15.55 ms | 24.6% | 21.62 ms |
| 9 runners, dense | 33.24 ms | 27.16 ms | 18.3% | 35.08 ms |
| 18 runners, standard | 57.86 ms | 51.53 ms | 10.9% | 75.01 ms |
| 18 runners, dense | 85.44 ms | 75.65 ms | 11.5% | 107.46 ms |

All 600 timed replies match for each executable. CPU-heavy background work continued throughout the audit, so compare paired engines within this table rather than comparing absolute timings with earlier sessions. Nine-runner races average 1,660 m and 174 equipped skills; eighteen-runner races average 2,220 m and 301 skills.

A staged build with only parsed-table reuse measured 16.53 ms for nine-runner standard output, versus 20.62 ms before and 15.55 ms with all changes. This identifies repeated JSON work as the largest demonstrated saving in that workload; the replay and tick changes provide additional benefit.

## Where worker time goes

Phase measurements come from a separate instrumented build. They include cleanup after response writing and exclude input transfer/read, so their totals differ from caller response latency. Each group has 150 measurements and every profiled reply matches the ordinary executable.

| Phase | 9 standard | 9 dense | 18 standard | 18 dense |
|---|---:|---:|---:|---:|
| Parse request JSON | 0.114 ms | 0.134 ms | 0.197 ms | 0.207 ms |
| Order runners by gate | 0.060 ms | 0.070 ms | 0.121 ms | 0.129 ms |
| Reuse or replace decoded skill definitions | 0.001 ms | 0.001 ms | 0.001 ms | 0.001 ms |
| Read race settings and geometry | 0.215 ms | 0.136 ms | 0.596 ms | 0.149 ms |
| Initialize stats and seeded state | 0.345 ms | 0.406 ms | 0.670 ms | 0.683 ms |
| Build runners and skill conditions | 1.284 ms | 1.490 ms | 3.180 ms | 3.256 ms |
| Set up field and runtime | 0.082 ms | 0.090 ms | 0.136 ms | 0.147 ms |
| Run simulation ticks | 10.452 ms | 12.049 ms | 42.474 ms | 43.795 ms |
| Assemble results and events | 0.156 ms | 0.177 ms | 0.315 ms | 0.314 ms |
| Select and wrap numeric replay frames | 0.048 ms | 0.040 ms | 0.115 ms | 0.057 ms |
| Restore requested runner order / metadata | 0.002 ms | 0.002 ms | 0.002 ms | 0.002 ms |
| Release solver state | 0.042 ms | 0.047 ms | 0.097 ms | 0.089 ms |
| Release parsed request | 0.048 ms | 0.052 ms | 0.087 ms | 0.086 ms |
| Serialize response JSON | 0.578 ms | 7.095 ms | 1.445 ms | 17.517 ms |
| Write and flush response | 0.187 ms | 1.947 ms | 0.400 ms | 5.472 ms |
| Release response buffers | 0.187 ms | 0.479 ms | 0.449 ms | 1.221 ms |
| **Worker processing including cleanup** | **13.80 ms** | **24.22 ms** | **50.29 ms** | **73.13 ms** |

Instrumented and ordinary mean latency differed by at most 2.0% in these groups. Small phases are approximate and sensitive to scheduling. Dense replay still spends substantial time formatting numbers and transferring the response.

## Inside the simulation

These sampled estimates partition standard-mode simulation time above. One tick in 32 is sampled with rotating offsets, timer overhead is corrected, and children are normalized within their parent phase. They are not additional costs.

| Tick work | 9 runners | 18 runners |
|---|---:|---:|
| Skill conditions, activation passes and effects | 3.32 ms | 10.76 ms |
| Runner movement and overtaking | 3.28 ms | 14.61 ms |
| Surrounding runners and lane context | 0.74 ms | 3.73 ms |
| Timers, course events, expiry and cooldown preparation | 0.88 ms | 2.48 ms |
| Front blockers and blocking state | 0.63 ms | 3.32 ms |
| Lane overlap resolution | 0.10 ms | 0.36 ms |
| Duels, spot struggle and field conditions | 0.45 ms | 4.36 ms |
| Ranks, pacemaker and activation caches | 0.63 ms | 1.79 ms |
| Active-runner/rank bookkeeping | 0.07 ms | 0.21 ms |
| Scoring, events and finish detection | 0.08 ms | 0.27 ms |
| Numeric replay snapshots | 0.28 ms | 0.59 ms |

## Allocation counts

| Workload | Previous optimized | Latest | Reduction |
|---|---:|---:|---:|
| 9 runners, standard | 71,637 | 35,360 | 50.6% |
| 9 runners, dense | 89,398 | 36,489 | 59.2% |
| 18 runners, standard | 151,014 | 95,124 | 37.0% |
| 18 runners, dense | 187,314 | 96,593 | 48.4% |

For standard nine-runner races, request parsing drops from about 26,230 to 1,167 allocations. Tick processing drops from 23,345 to 13,479. Dense replay construction drops from 19,272 to 1,286 allocations with nine runners and from 39,247 to 1,707 with eighteen runners.

Counts include allocation and reallocation calls, measured separately from latency; they are not peak memory or allocator-time measurements. The parsed-data cache retains the latest parsed table and the last successfully installed definitions, with shared ownership when they match. Repeated requests no longer duplicate the table tree.

## Cache behavior and less favorable workloads

The cache checks actual text and parsed content, never `dataSignature`. Numeric token boundaries, escaped keys, duplicate keys, Unicode and exact error behavior remain covered. A malformed request does not replace the last parsed cache entry; a table that fails decoding does not replace installed definitions. Omitting `skillData` selects the bundled table.

Whitespace, key ordering or escape changes inside the table can require reparsing even when the decoded definitions are equivalent. The ASCII path compares raw bytes directly; Unicode input preserves the existing character-based parser and comparison path. Cold workers and continually changing tables get less benefit from reuse.

| Additional nine-runner standard workload | Previous optimized | Latest | Reduction |
|---|---:|---:|---:|
| Changed definition metadata every request | 21.18 ms | 18.45 ms | 12.9% |
| Unicode request text with repeated definitions | 20.39 ms | 15.77 ms | 22.7% |
| Alternating table whitespace without content changes | 18.11 ms | 17.57 ms | 2.9% |

## Remaining opportunities

1. **Skill-condition construction and evaluation.** Runner setup still allocates about 10,973 times for nine runners and 22,497 for eighteen. Separate immutable parsed condition structure from per-runner state, and investigate indexed cooldown/activation lookups. Prerequisite latches, random reservations and additional activation counters must remain private to each runner.
2. **Live-runner and lane queries.** Movement repeatedly scans peers and reconstructs observations. Reuse valid data or iterate cached candidate indices while preserving visibility, iteration order and earlier runners’ live updates. In duel selection, investigating iteration over the existing proximity keys could avoid scanning unrelated peers.
3. **Remaining tick allocations.** Timers, skill history and rank observers still allocate. Use the measured phase counts to prioritize concrete lifetimes rather than assuming that every collection is an expensive allocation.
4. **Dense JSON formatting and transfer.** Whole-frame encoding removes construction overhead but still emits the same numeric text and payload. Specialized formatting for common integral fields could be tested against exact bytes. Binary or reduced-detail output would require an optional interface change.

These are candidates, not promised savings. Approximate math, coarser timesteps or changed RNG/field ordering would alter behavior. Parallel workers can improve throughput across independent races without changing one race’s ordered updates.

## Validation and reproduction

- **13,187/13,187** dense replies match the preceding optimized engine and recorded reference, with zero execution errors and unchanged official metrics: 13,183 exact races and 13,185 correct winners. This is the existing captured corpus.
- **292** regression/isolation replies match, including changed/omitted/invalid tables, recovery, runner ordering, runtime modes and fields of 1–30 runners.
- **186** additional raw-wire replies match, including 78 expected error replies. These cover Unicode, whitespace, escaped and duplicate keys, malformed requests and numeric-token boundaries.
- Rust suite: **84 passed, 11 failed**. The same 11 existing failures remain; new parser-cache tests and updated replay, lane and style-leader checks pass.

[Measurements and per-race summaries](../evidence/performance-audit-next.json), [full corpus results](../evidence/full-validation-next.json), [worker checks](../evidence/next-worker-parity.json), [raw-wire checks](../evidence/next-wire-parity.json), and [Rust test log](../evidence/next-cargo-test.txt). The [preceding audit evidence](../evidence/performance-audit-optimized.json) supplies the allocation baseline.

Build and benchmark commands are in [Performance](PERFORMANCE.md); regression commands are in [Testing](TESTING.md). Source: [request handling and replay](../src/main.rs), [JSON cache and serializer](../src/json.rs), [skill and duel processing](../src/race_solver.rs), [movement](../src/race_solver_retail.rs), [lanes](../src/retail_lane.rs), and [scoring](../src/stadium_score.rs).
