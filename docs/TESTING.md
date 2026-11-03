# Testing

Run commands from the repository root. Use an explicit engine path so the
validator checks the intended build. Published validation results and their
scope are described in [Accuracy and coverage](ACCURACY.md).

## Build and unit tests

The Python validation tools require NumPy (`python -m pip install numpy`).

```sh
cargo test --release --locked
cargo build --release --locked
python -m unittest discover -s tests
```

Build release artifacts after the Rust tests so the validation binary has the
intended release configuration. To test the platform allocator instead of the
default mimalloc allocator:

```sh
cargo test --release --locked --no-default-features --lib
```

Unit tests cover physics boundaries, skill state, random-number consumption,
sorting, movement, replay encoding and prepared-race isolation. Implementation
snapshot tests guard against unintended changes; they are distinct from
comparisons with official race observations.

## Captured-race regression fixtures

```sh
python tools/check_accuracy_regressions.py --engine target/release/race_engine_rs_v2.exe --report target/accuracy.json
python tools/audit_validation_coverage.py --report target/coverage.json
```

On Linux or macOS, omit `.exe`. The default fixture directory is
`tests/fixtures/accuracy`. Each compressed fixture contains its input and
expected official observations. Use `--fixtures path/to/fixtures` to select
another fixture collection.

The default checker includes strict event ordering and post-finish observations.
The `--legacy` option selects a narrower comparison and must be reported as a
different scope. Reports bind the engine and exact fixture files used in the
check. A focused fixture collection does not establish whole-corpus accuracy.

## Worker and prepared-library checks

Use JSONL inputs covering varied courses, skills, rosters and seeds. Retain a
separate validated engine as the baseline.

```sh
python tools/check_worker_parity.py --engine path/to/candidate --baseline path/to/baseline --inputs path/to/races.jsonl --isolation --report target/worker-parity.json
python tools/check_replay_protocol.py --engine path/to/candidate --baseline path/to/baseline --inputs path/to/races.jsonl --isolation
python tools/check_seed_sampling.py --engine path/to/candidate --baseline path/to/baseline --inputs path/to/races.jsonl --report target/seed-sampling.json
python tools/check_native_parity.py --library target/release/globalsim.dll --baseline path/to/baseline --inputs path/to/races.jsonl --isolation --report target/native-parity.json
```

Use `libglobalsim.so` or `libglobalsim.dylib` for the native library on other
platforms. These checks cover changing inputs, JSON and binary replay agreement,
serial and concurrent seed batches, separate handles, repeated runs, output
ownership and recovery after invalid requests.

An intentional accuracy correction can change output bytes. Compare that change
with its reference observations before classifying it as a fix or a regression.
Byte equality alone does not establish official accuracy.

## Broader validation

For a mechanics change, preserve the seed, complete runner inputs, course data,
runtime settings and reference observations. Add a focused regression and nearby
boundary cases, then check the affected corpus. Keep unavailable observations
separate from actual matches, and investigate output changes in unresolved cases
as well as previously passing cases.

`tools/check_official_promotion.py` checks fixture reports and full-corpus audit
receipts for complete coverage and new official failures:

```sh
python tools/check_official_promotion.py --baseline-fixtures path/to/baseline/fixtures.json --candidate-fixtures path/to/candidate/fixtures.json --inventory path/to/frozen/jobs.jsonl --baseline-practice path/to/baseline/audit --candidate-practice path/to/candidate/audit --report target/promotion.json
```

Each audit directory supplies `config.json`, `jobs.jsonl`, `results.jsonl` and
`summary.json`, including the engine and data identities. The inventory must
represent the complete intended comparison scope. Omitting corpus arguments
provides a fixture screen but leaves the promotion gate closed. The command
reports its decision and does not publish or deploy a build.

Keep [experimental feature](EXPERIMENTAL.md) results separate from default-build
results. Measure [performance](PERFORMANCE.md) independently using identical
inputs, output modes and build settings.
