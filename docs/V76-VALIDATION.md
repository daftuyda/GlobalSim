# Version 76 validation

Version 76 reduces repeated setup and field-query work while retaining full
replay output. The final optimized build matches the previous engine's complete
binary output across all 470,350 frozen race inputs, with no changed outputs or
execution errors. This preserves the existing 469,142 strict matches and 1,208
Career discrepancies. Quarantine membership is unchanged.

## Stateful precondition correction

The original optimization reused a compiled skill when its main condition was
stateless. It did not check whether the precondition retained mutable state.
An `is_hp_empty_onetime==1` precondition remembers previous HP: sharing that
observer between races allowed the first run to activate a skill and suppressed
it on later runs of the identical input and seed.

The controlled one-runner case uses skill 999999 with `always==1` as its main
condition and the HP-empty precondition. The expected event occurs at
118.81246948242188 seconds. The uncorrected rewrite loses this event on reused
prepared runs and serial seed batches. Its finish time happens to remain
unchanged in this reproducer, so finish-only validation would miss the defect.
This is a synthetic valid-input test, not evidence of a failed official capture.

Compiled skill data is now cached only when both the main condition and
precondition are stateless, and no sampled triggers were used. Stateful
preconditions are rebuilt for each race. The main-condition state flag remains
separate so cooldown evaluation order does not change.

Both new Rust tests fail before the fix and pass afterward. They compare complete
replays across repeated and changed seeds, prepared setup reuse, and reused
worker threads. The same reproducer also passes through the final PGO worker.

## Evidence and limits

- [Release checks and corpus identity](../evidence/v76-release-validation.json)
- [Captured regression fixtures](../evidence/v76-accuracy-fixtures.json)
- [Seed sampling, output modes and recovery](../evidence/v76-seed-sampling.json)
- [Prepared library isolation and equivalence](../evidence/v76-native-parity.json)
- [HP-empty precondition reproduction after correction](../evidence/v76-stateful-precondition.json)
- [Paired timing trials](../evidence/v76-performance.json)

The ordinary release build passes 160 Rust tests with each allocator, all 81
captured fixtures and the repeated-seed reproducer. The final PGO build passes
1,995 worker sampling comparisons, prepared-library equivalence/isolation checks,
and the full corpus comparison. The Python validation suite passes 27 tests.
The corpus comparator verifies original reference hashes and exact input hashes
before comparing complete binary output hashes against the previously audited
engine. Unchanged outputs reuse that engine's strict observation verdicts.

The performance comparison uses the same compiler, portable PGO settings, fast
allocator, training inputs and pinned CPU for both builds. Full-frame worker
latencies improve by 4.5%, 5.4% and 7.6% on the measured 9-, 12- and 18-runner
workloads. Nine-runner prepared numeric timing is effectively unchanged.
See [performance](PERFORMANCE.md) for distances, timing scope and all modes.

These checks establish regression agreement for the tested inputs and interfaces.
They do not resolve the existing Career discrepancies or establish universal
equivalence with the official game.
