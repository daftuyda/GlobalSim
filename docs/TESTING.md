# Testing

Run the Rust test suite with:

```sh
cargo test --release --locked
```

The most recent recorded run has **79 passing tests and 11 failures**. Failures
cover numerical reference comparisons, geometry, stamina and spurt behavior,
lateral movement, and duel conditions. See the [test output](../evidence/cargo-test.txt)
for individual assertions.

The release build and included example requests run successfully. Test results
and [official-race benchmark results](ACCURACY.md) measure different aspects
of the engine and should be considered together.

All **122 captured-race regression cases** pass against the v71 release build,
covering the pacing and duel changes alongside previously matching races.
See [regression results](../evidence/v71-regression-verification.json) and
[example and unit-test results](../evidence/v71-local-verification.json).

Performance changes also preserve byte-identical output across 122 reference
races in both replay modes, plus 13 cases covering changed, omitted and invalid
skill tables, runner ordering, runtime modes and fields of 1–30 runners.
All 270 replies match both the original v71 build and the preceding optimized
build, including the expected replies for invalid requests.
Additional unit tests cover frame cadence, finish samples, dense output,
course-cache identity, numeric precision, Unicode, escapes, parser errors and
deferred blocker checks.
The parser also matches the preceding implementation on 9,258 varied inputs. [Performance checks](../evidence/performance.json).

## Full captured-race validation

All **13,187 races** produce byte-identical dense output to the recorded v71
reference outputs. Official replay checks retain **13,183 exact races** and **13,185 correct
winners**, with the same four non-exact races and no execution errors.
Checks cover finish times, skill activations and replay samples for every race. This is the existing captured
corpus, not an independent holdout.

The simultaneous comparison with the local original executable matched 13,186
out of 13,187 replies. The remaining case matched in a fresh run and all 100
repeated checks. Its cause remains undetermined; the optimized reply matched
the recorded reference throughout. See [full results](../evidence/full-validation.json).
