# Testing

Run the Rust test suite with:

```sh
cargo test --release --locked
```

The most recent recorded run has **85 passing tests and 11 failures**. The failure
set is unchanged from the preceding optimized build. Failures
cover numerical reference comparisons, geometry, stamina and spurt behavior,
lateral movement, and duel conditions. See the [test output](../evidence/accuracy-cargo-test.txt)
for individual assertions.

The release build and included example requests run successfully. Test results
and [official-race benchmark results](ACCURACY.md) measure different aspects
of the engine and should be considered together.

All **122 captured-race regression cases** pass against the v71 release build,
covering the pacing and duel changes alongside previously matching races.
See [regression results](../evidence/v71-regression-verification.json) and
[example and unit-test results](../evidence/v71-local-verification.json).

The accuracy corrections preserve byte-identical replies for 135
captured-race and edge cases, plus 11 persistent-worker isolation cases, in both
replay modes: **292 matching replies**. Coverage includes changed definitions
under an unchanged signature, omitted/empty/invalid tables and recovery, runner
ordering, runtime modes and fields of 1–30 runners. Expected invalid-request
errors are also compared. [Worker parity results](../evidence/accuracy-worker-parity.json).

The preceding parser validation matched **186 raw-wire replies**, including 78 expected errors, covering
Unicode, whitespace, escaped and duplicate keys, malformed requests, numeric
token boundaries and recovery. [Raw-wire results](../evidence/next-wire-parity.json).

Passing unit tests cover whole-frame float formatting, blocker-index and
runner-order restoration, exact nested skill-data comparison, and shared parser
cache identity, invalidation and error behavior. Lane and scoring checks also
exercise iterator-based modifiers and repeated rank updates with ties.
Existing tests cover frame cadence, finish samples, dense output, course-cache
identity, numeric precision, Unicode, escapes, parser errors and deferred blocker
checks. The parser changes also have an earlier 9,258-input comparison recorded
in the [performance checks](../evidence/performance.json).

To compare two persistent workers on your own inputs:

```powershell
python tools/check_worker_parity.py --engine target/release/race_engine_rs_v2.exe --baseline path/to/previous-engine.exe --inputs examples/v72-race.jsonl
```

Add `--isolation` when supplying captured inputs with `skillData` and equipped
speed or acceleration skills. The tool verifies that changed definitions affect
the chosen race and that restoring the original definitions restores its output.

## Full captured-race validation

All **13,187 races** now match official raw finish times, winners, placements,
skill starts and every compared replay sample, with no execution errors.
The four previously differing races are corrected; the other **13,183 dense
replies remain byte-identical**. This is the existing captured corpus, not an
independent holdout. See [full results](../evidence/accuracy-validation.json)
and the [comparison definitions](ACCURACY.md).

Four included regression fixtures cover pacemaker restoration, healer-history
timing for both runner update orders, and an exact shared slope boundary. They
check finish times, skill starts, healer-targeting recipient masks and sampled
movement. The engine receives only pre-race inputs; expected official
observations are checked separately. All four fail on the preceding build and
pass with the corrections. [Fixture results](../evidence/accuracy-regressions.json).

```powershell
python tools/check_accuracy_regressions.py --engine target/release/race_engine_rs_v2.exe
```

The Rust suite also checks slope boundary inclusion, expiry without re-entry,
and crossing a boundary without landing exactly on it. Parallel corpus timings
are validation-run timings; use the separate paired [performance measurements](PERFORMANCE.md)
for latency comparisons.
