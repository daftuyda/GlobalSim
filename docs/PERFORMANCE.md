# Performance

## Measured latency

The published v76 benchmark provides a release baseline. These figures have not
been remeasured for the current source revision.

| Workload | Binary worker | Prepared Rust: numeric replay | Prepared Rust: binary packet |
|---|---:|---:|---:|
| 9 runners, 1600 m | 4.619 ms | 3.890 ms | 3.836 ms |
| 12 runners, 1200 m | 5.356 ms | 4.524 ms | 4.690 ms |
| 18 runners, 1600 m | 10.681 ms | 8.988 ms | 9.154 ms |

Values are mean individual-race latencies with every replay frame retained.
The benchmark used Windows, an Intel Core i7-14700K, Rust 1.97.1, portable
profile-guided optimization (PGO) and the default mimalloc allocator. Trials ran
on logical CPU 2 at above-normal priority. GlobalSim does not set process
affinity or priority itself. An ordinary release build uses different compiler
settings from this PGO build.

Worker timing includes request transfer, parsing, simulation, binary encoding
and complete reply transfer. Client encoding and decoding are excluded. Each
workload cycles through 30 seeds for 240 measured requests after five warmups.

Prepared timing includes fresh seed state, simulation, owned results and
cleanup; binary mode also includes packet encoding. Input preparation and pipe
transfer are excluded. Each workload has four trials of 180 measured races,
with five warmups per trial. See the [prepared API](IN-PROCESS.md) for ownership
and integration details.

Hardware, system load, roster, course and equipped skills affect latency.
Measure parallel throughput and results-only requests separately from full
replay latency.

## Benchmark a local build

Run these commands from the repository root. The included example provides a
complete race request; use a representative JSONL workload for your application.

```sh
cargo build --release --locked
python tools/benchmark.py --engine target/release/race_engine_rs_v2.exe --inputs examples/v77-race.jsonl --full-frames --repeats 30
python tools/benchmark_replay.py --engine target/release/race_engine_rs_v2.exe --inputs examples/v77-race.jsonl --repeats 30 --report target/replay-benchmark.json
```

Omit `.exe` on Linux or macOS. The first benchmark measures JSON worker latency;
the second compares full-frame JSON and binary replies. Both accept
`--baseline path/to/engine` for a paired comparison with a separate build.

Measure prepared Rust calls with the included benchmark example:

```sh
cargo run --release --locked --example benchmark_prepared -- examples/v77-race.jsonl 6 numeric
cargo run --release --locked --example benchmark_prepared -- examples/v77-race.jsonl 6 binary
```

The repetition argument selects batches of 30 seeds, so `6` measures 180 races
after five warmups. Preparation time is reported separately. This example
accepts one race request; use input with `retainAllFrames: true` when measuring
full replays.

For independent worker throughput:

```sh
python tools/benchmark_throughput.py --engine target/release/race_engine_rs_v2.exe --inputs examples/v77-race.jsonl --workers 1 2 4 --report target/throughput.json
```

Keep inputs, runtime settings, output mode and build configuration consistent
when comparing measurements. [Testing](TESTING.md) covers correctness checks.

## Build and output options

The default `fast-alloc` feature enables mimalloc and requires a C toolchain.
Add `--no-default-features` to use the platform allocator. The
[allocator license](../licenses/mimalloc.txt) and
[binding license](../licenses/mimalloc-rust.txt) are included.

For profile-guided compilation of the worker and prepared library:

```sh
rustup component add llvm-tools-preview
python tools/build_pgo.py --inputs examples/v77-race.jsonl --features fast-alloc --train-binary --train-native --output target/pgo/race_engine_rs_v2.exe --library-output target/pgo/globalsim.dll --benchmark-output target/pgo/benchmark_prepared.exe
```

On Linux or macOS, omit `.exe` and use `libglobalsim.so` or `libglobalsim.dylib`
for the library output. Replace the example with representative courses,
rosters, skills and frame settings for production training; repeat `--inputs`
to combine files. `--train-native` exercises prepared calls three times per
input. Add `--native` only when deployment supports the build CPU's features.

JSON is the default worker output. Set `GLOBALSIM_OUTPUT=binary-v1` for the
[lossless binary protocol](BINARY-REPLAY.md). Set `GLOBALSIM_REPLAY_WORKER=1` to
overlap JSON encoding with simulation for eligible full-frame requests.
Prepared calls expose numeric results or explicitly selected serialization.

For one setup across seeds, send a worker request with `seeds` and optionally
`threads`; `omitFrames` selects results-only output. See
[seed sampling](INTERFACE.md#seed-sampling) for the request fields.
