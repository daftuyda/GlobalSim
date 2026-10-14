# Testing

Run the Rust test suite with:

```sh
cargo test --release --locked
```

Both default and `fast-alloc` release suites have **125 passing tests and 11 pre-existing failures**.
The failure set is unchanged. Failures cover
numerical reference comparisons, geometry, stamina and spurt behavior,
lateral movement, and duel conditions.
[Default allocator log](../evidence/parity-default-tests.txt),
[fast allocator log](../evidence/parity-fast-tests.txt).

Unit tests, persistent-worker checks and [official-race benchmark results](ACCURACY.md)
measure different aspects of the engine and should be considered together.

The current accuracy build passes **23 captured regressions** and all audited
observations across **47,664 official races**. The optimized build reproduces
every complete packet from the scored build. There are no observed regressions.
[Current validation](../evidence/parity-validation-20261014.json).

The current prepared library matches 843 runs per frame mode against the ordinary
release worker, plus 24 concurrent runs and handle isolation/recovery checks.
The binary protocol check matches 281 replies per frame mode, including expected
errors. [Library checks](../evidence/parity-native-parity.json),
[protocol checks](../evidence/parity-wire-parity.json).

The ordinary release build and fast-allocator PGO build preserve **292 matching
JSON replies per configuration** across captured races, edge cases and worker
isolation checks. Coverage includes changed definitions under the same signature,
omitted/empty/invalid tables and recovery, runner ordering and runtime modes.
The PGO helper is checked separately. Binary output preserves all metadata and
frame values across the same 292 cases, with the helper setting both disabled
and enabled. Expected error replies also match.
[Generic checks](../evidence/prepared-generic-worker.json),
[PGO checks](../evidence/prepared-pgo-worker.json),
[helper checks](../evidence/prepared-helper-worker.json),
[binary checks](../evidence/prepared-protocol-check.json).

The current ordinary release build matches **186 raw-wire replies**, including 78 expected errors, covering
Unicode, whitespace, escaped and duplicate keys, malformed requests, numeric
token boundaries and recovery. [Raw-wire results](../evidence/prepared-wire-parity.json).

Passing unit tests cover whole-frame float formatting, blocker-index and
runner-order restoration, exact nested skill-data comparison, and shared parser
cache identity, invalidation and error behavior. Lane and scoring checks also
exercise iterator-based modifiers and repeated rank updates with ties.
Existing tests cover frame cadence, finish samples, dense output, course-cache
identity, numeric precision, Unicode, escapes, parser errors and deferred blocker
checks.

Additional focused coverage includes:

- Shared condition trees with independent sampled windows and activation
  observers, unchanged sampling errors, and separate runtime/table identities.
- Priority and prerequisite indexes preserve observation order, pending removals,
  activation history, cooldown state and RNG state. Empty priority-zero passes
  still flush required bookkeeping; stateful observations continue during cooldown.
  Compact window checks match full visits for inactive and reversed windows,
  non-finite boundaries, removals, random reservations and additional effects.
- Prepared roster facts preserve self exclusions, duplicate skill handling,
  favorite selection and ordered team-stat sums in both runtimes. Sampled-trigger
  indexing preserves expression-node order and parser validation errors.
- The evolving movement roster matches fresh field reads through ordered updates
  and callback invalidation. Blocking samples and passing membership match live
  reference queries at each tested tick, including callbacks that change peer
  targets. Membership tests cover owner exclusions, replacements, duplicates
  and fields up to 128 runners. Acceleration caches refresh for every dependency and
  parameter-object replacement. HP factor caching preserves reinitialization and
  phase boundaries, and the target-speed shortcut preserves clamped results.
- Bitwise geometry interpolation comparisons across all 119 courses, including
  overrun curves, adjacent samples and exact endpoints.
- Specialized number formatting against the standard formatter, frame-text cache
  transitions, changing runner counts, non-finite values and response-buffer reuse.
- Incremental frame encoding matches complete-array encoding for empty arrays,
  changing runner counts and batch boundaries. Persistent helper tests cover
  buffer reuse and abandoned frame sequences.
- Overtaking decisions that skip irrelevant blocker queries and evaluate necessary
  blocker queries exactly once.

The specialized formatter also matches the original formatter across
**162,383,552 numeric comparisons**, including every positive float32 bit pattern
in its optimized magnitude range and additional sampled values. This is exhaustive
coverage of that positive float32 range, not of all float64 values.
[Formatter validation](../evidence/five-ms-format-validation.json).

To compare two persistent workers on your own inputs:

```powershell
python tools/check_worker_parity.py --engine target/release/race_engine_rs_v2.exe --baseline path/to/previous-engine.exe --inputs examples/v73-race.jsonl
```

Add `--isolation` when supplying captured inputs with `skillData` and equipped
speed or acceleration skills. The tool verifies that changed definitions affect
the chosen race and that restoring the original definitions restores its output.

Set `GLOBALSIM_REPLAY_WORKER=1` before launching the check to exercise the optional
helper. Include full-frame requests that need no gate permutation, sparse and
gate-reordered fallbacks, and alternating request modes. The helper changes
encoding concurrency while retaining the same JSON contract.

## Full captured-race validation

All **47,664 races** match the audited official observations: raw finishes,
winners, all 435,508 placements, all 4,497,700 skill starts and every compared
replay sample. Recorded skill durations, detail choices, visual recipient masks
and competition events also match. Binary PGO replies preserve all metadata
and every generated frame from the scored build. These are regression checks,
not a guarantee for unseen races.
See [validation evidence](../evidence/parity-validation-20261014.json) and the
[comparison definitions](ACCURACY.md).

Twenty-three included regression fixtures cover pacemaker restoration, healer-history
timing for both runner update orders, an exact shared slope boundary,
front-runner competition eligibility, cached leader identity at the 150 m
threshold, invitations to finished duel partners and entry into groups that
remain active past the finish line. They also cover bonuses queued on a skill's
final tick, inclusive and exclusive front-block boundaries, and leader-gap
duration rounding. Checks include finish times, skill starts, competition
events, visual recipient masks and sampled movement; the new fixtures also
check recorded skill durations. The engine receives only pre-race inputs;
expected official observations are checked separately.
[Fixture results](../evidence/parity-regressions.json).

```powershell
python tools/check_accuracy_regressions.py --engine target/release/race_engine_rs_v2.exe
```

The Rust suite also checks slope boundary inclusion, expiry without re-entry,
crossing a boundary without landing exactly on it, the inclusive 150 m competition
threshold, and HP and separation exits for duels after the finish. Parallel corpus timings
are validation-run timings; use the separate paired [performance measurements](PERFORMANCE.md)
for latency comparisons.

Focused unit tests additionally check queued-bonus caps, cooldown cleanup and
receiver expiry, front-block edges one float apart, and duration multiplier
rounding and clamps.

## Binary protocol checks

```powershell
cargo test --release --locked --features fast-alloc
python -m unittest discover -s tests -p test_replay_protocol.py
python tools/check_replay_protocol.py --engine target/pgo/race_engine_rs_v2.exe --baseline path/to/reference-engine.exe --inputs path/to/races.jsonl --isolation
```

Rust checks retain float64 bits, signed zero, nonfinite values, sparse selection,
runner permutations, blocker indices and recovery after errors. Reader checks
cover partial reads, consecutive packets, malformed lengths and versions,
truncation and bounded reads. The conventional JSON number formatter is unchanged.

## Prepared library checks

Both generic and profile-guided libraries preserve 876 outcomes each across
repeated and alternate seeds, dense and sparse frames, changed skill tables,
runner permutations and expected errors. Separate checks interleave handles
with different definitions, retain old output views across later runs, reject
cross-thread handle use, and run 24 races concurrently with independent handles.
Failed preparation clears the old race; failed runs clear the old reply and
permit recovery. [Generic API checks](../evidence/prepared-generic-api.json),
[PGO API checks](../evidence/prepared-pgo-api.json).

The current prepared PGO library check uses 270 captured inputs and generated
variants, comparing every selected frame value and all metadata with the release
worker. Rust unit tests additionally compare prepared initialization with the original path across
signed seed boundaries, checking exact float bits and subsequent RNG draws.

```powershell
python tools/check_native_parity.py --library target/pgo/globalsim.dll --baseline path/to/reference-engine.exe --inputs path/to/races.jsonl --isolation
```
