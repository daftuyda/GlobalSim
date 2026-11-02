# Testing

Run commands from the repository root. Use a release build and an explicit
engine path so the validator checks the binary you intend to ship.

## Current validation

The [consolidated validation summary](VALIDATION-SUMMARY-20261103.md) was reviewed
on 3 November from the completed reports. It separates release tests, official
observations, third-party reports, detailed traces and rejected experiments.

[Version 77 release validation](V77-VALIDATION.md) records the fresh default-build
checks and the previously completed full practice audit.

The default build passes **167 Rust tests**
and **84 official captured-race regression fixtures**. The full practice audit
matches **696,777/696,778 active official captures** from the frozen 28 October
inventory. One movement case remains unresolved. All current races were executed;
prior strict passes were reused only for identical captures, inputs and complete
outputs. Changed outputs received fresh strict comparisons.

The earlier passing-route refinement preserved all 52,096 complete outputs in
its official regression cohort.
Across 437,000 third-party inputs, it preserved 436,999 complete outputs and
corrected one discrepancy without regressions.

The Python validation suite also passes all **39 tests**, including twelve
promotion-gate tests for missing captures, altered inputs, reduced coverage,
quarantine changes, changed unresolved cases and stale validation receipts.

The optional experimental profile has its own results and known official-fixture
failures. Use the [experimental validation instructions](EXPERIMENTAL.md) when
building with `experimental-umamoe`; the default-build accuracy results above do
not describe that profile.

These are separate checks. An unchanged previously failing output remains a
failure against its reference. See [accuracy and coverage](ACCURACY.md) and the
[current official evidence](../evidence/official-practice-validation-20261101.json).

## Build and unit tests

```sh
cargo test --release --locked
cargo build --release --locked
python -m unittest discover -s tests
```

Build release artifacts after the Rust tests so a test library's panic
configuration does not replace the binary intended for validation or deployment.

The default allocator is mimalloc. To check the platform-allocator configuration:

```sh
cargo test --release --locked --no-default-features --lib
```

Unit tests cover physics boundaries, skill state, RNG consumption, sorting,
passing, replay encoding and prepared-race isolation. Some numeric fixtures
are implementation snapshots, as recorded in their provenance fields; those
guard against changes rather than independently prove official behavior.

## Official regression fixtures

```sh
python tools/check_accuracy_regressions.py --engine target/release/race_engine_rs_v2.exe --report target/accuracy.json
python tools/audit_validation_coverage.py --report target/coverage.json
```

On Linux or macOS, omit `.exe` from the engine path. The default fixture
directory is `tests/fixtures/accuracy`. Each compressed fixture contains its
input and expected official observations. The default comparison includes
strict ordering and post-finish observations; `--legacy` selects the earlier
pre-finish comparison and must be reported separately.
Each report includes `fixturesSha256`, binding the exact compressed fixture files
used in that run. Promotion comparisons require matching fixture receipts.

The known unresolved official movement case is retained separately. This command
is expected to report a strict failure and exit with status 1:

```sh
python tools/check_accuracy_regressions.py --engine target/release/race_engine_rs_v2.exe --fixtures tests/fixtures/unresolved --report target/unresolved-accuracy.json
```

Its presence outside the passing regression directory does not exclude it from
the complete active-practice audit. Report the unresolved result separately;
do not describe the 84 passing regression fixtures as every included capture.

Recent regressions cover pacemaker retention at section boundaries, inward
corridor rounding, competition partner cutoffs and event order. Lane unit tests
also cover equal-cost routes, loaded body clearance, same-target ties, the
straight fallback, and outward obstacle edges on opposite sides of a runner.

## Worker and prepared-library checks

Use JSONL inputs covering varied courses, skills, rosters and seeds. The
baseline should be a separately retained validated engine.

```sh
python tools/check_worker_parity.py --engine path/to/candidate --baseline path/to/baseline --inputs path/to/races.jsonl --isolation --report target/worker-parity.json
python tools/check_replay_protocol.py --engine path/to/candidate --baseline path/to/baseline --inputs path/to/races.jsonl --isolation
python tools/check_seed_sampling.py --engine path/to/candidate --baseline path/to/baseline --inputs path/to/races.jsonl --report target/seed-sampling.json
python tools/check_native_parity.py --library target/release/globalsim.dll --baseline path/to/baseline --inputs path/to/races.jsonl --isolation --report target/native-parity.json
```

Use `libglobalsim.so` or `libglobalsim.dylib` for the native library on other
platforms. Worker checks exercise changing inputs and recovery. Protocol checks
compare metadata and numeric frames across JSON and binary modes. Seed sampling
covers serial/concurrent batches and results-only requests. Native checks cover
repeated seeds, separate handles, output ownership, recovery and concurrency.

For accuracy fixes, expect intentional output changes only in diagnosed cases.
A byte-equality checker will flag those too; classify each change against its
reference instead of treating every changed output as a regression or a fix.

## Capture validation

The wider capture dataset is maintained separately from this repository.
Freeze the input inventory, preserve original seeds and capture linkage, and
record engine settings and data identity. Keep quarantine explicit and report
active, quarantined and total counts independently.

Distinguish input-integrity failures, engine failures, observation discrepancies
and unavailable observations. A coverage gap is not a pass. Retain full simulator
output for diagnosis, even when the official replay only exposes sparse samples.
Detailed replay captures can explain differences hidden by ordinary exports.

For a candidate correction:

1. Reproduce the original case and identify its first differing observation.
2. Add a focused regression and nearby boundary/control cases.
3. Run the official fixture panel, then the affected frozen cohorts.
4. Compare complete baseline and candidate outputs; investigate every change.
5. Check worker/prepared equivalence when shared state or execution order changes.
6. Measure latency separately using identical inputs, output modes and build settings.

Before promoting a behavior change, run the complete frozen active-practice
comparison and the [official regression gate](ACCURACY.md#official-regression-gate).
An improvement on one official case or on uma.moe cannot offset a new official
failure. The gate remains closed if full practice evidence is missing, even when
all 84 focused regression fixtures pass. Use the authoritative inventory from the capture
registry; a smaller selected cohort does not establish whole-corpus accuracy.

Third-party simulations support diagnosis but do not override a conflicting
official capture. See [third-party comparison](UMAMOE-MOVEMENT-20261031.md),
[validation coverage](VALIDATION-AUDIT-20261022.md) and
[capture integrity](VALIDATION-HARDENING-20261022.md).

## Historical release checks

The v76 release passed 160 Rust tests, 27 Python tests and 81 official fixtures.
Its optimization preserved complete outputs across 470,350 frozen official
captures, including existing Career discrepancies. It also passed 1,995 seed
sampling comparisons and prepared-library checks. Those are release-specific
results, not fresh measurements of every later correction.
See [v76 validation](V76-VALIDATION.md) and [historical reports](README.md#historical-reports).
