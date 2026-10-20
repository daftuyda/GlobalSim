# Performance

The prepared Rust API averages **4.62 ms per seed with every frame retained**
on the measured nine-runner, 1600 m workload. Including complete binary packet
encoding averages **5.05 ms**. These measurements use a portable profile-guided
release build with the fast allocator and reuse race preparation across seeds.

## Prepared Rust calls

| Workload | Output | v74 baseline | Optimized | Change in mean time |
|---|---|---:|---:|---:|
| 9 runners, 1600 m | Owned numeric replay | 4.74 ms | 4.62 ms | 2.6% lower |
| 9 runners, 1600 m | Binary packet in memory | 4.98 ms | 5.05 ms | 1.4% higher |
| 18 runners, 1200 m | Owned numeric replay | 10.26 ms | 9.89 ms | 3.6% lower |
| 18 runners, 1200 m | Binary packet in memory | 10.23 ms | 10.00 ms | 2.2% lower |

Both builds used the same compiler settings and profile-training inputs. Each
initial comparison used four alternating trials per build and output mode, with
480 measured races and five warmups per trial. The nine-runner binary case also
received six trials of 720 measured races per build because its initial means
were inconsistent. Its reported mean includes **all** initial and follow-up
trials; there is no demonstrated speedup for that case.

Trials cycle through the same 30 seeds. Timing includes fresh seed state,
simulation, every frame, results and cleanup; binary mode also includes encoding
the complete packet. One-time preparation, pipe transfer and application analysis
are excluded. Both builds ran on the same performance core at above-normal
process priority while background collection continued. The engine does not set
CPU affinity or process priority automatically.

The engine caches inactive skill runs between trigger boundaries and combines
visibility and blocking queries over the cached field. Stateful skill observers,
pending effects and callbacks retain their existing evaluation order. These
changes reduce simulation work; they do not reduce replay detail.

The optimized build matches the baseline byte for byte on **203,572 race
inputs**, including every replay frame, result and event. All 63 captured
regression fixtures also pass.

These two workloads do not establish a universal latency or a guaranteed
percentage improvement. See the [measurements and validation](../evidence/v74-skill-movement-performance.json)
for every trial and the [prepared API guide](IN-PROCESS.md) for integration,
ownership, threading and benchmark commands.

## Binary worker and Python binding

The following interface measurements precede v73.

| Full-frame workload | Baseline worker | Prepared worker | Prepared Python binding |
|---|---:|---:|---:|
| 9 runners, 1600 m Monte Carlo | 5.82 ms | 5.85 ms | 5.72 ms |
| 9 runners, varied races | 6.40 ms | 6.48 ms | 6.50 ms |
| 18 runners, varied races | 19.84 ms | 19.97 ms | 19.15 ms |

All columns use portable profile-guided builds with the fast allocator. Thirty
inputs per group, seven repetitions and five warmup requests, with execution
order rotating and reversing. Monte Carlo reuses one prepared roster across
30 seeds; the varied groups retain 30 separate preparations. Every binary packet
matches the preceding engine byte for byte.

Worker timing includes parsing, simulation, capture, encoding and pipe transfer.
Python binding timing includes simulation, capture, encoding and copying the
entire packet into Python-owned bytes. Request encoding and response decoding
are excluded. Preparation is separate: the first library preparation, including
loading, took 26.5 ms in this run; subsequent preparations varied by input, mostly
4–9 ms. [Paired measurements](../evidence/prepared-interface-performance.json).

These paired measurements and the Rust-only trials use different harnesses.
Their absolute means should not be treated as an isolated measurement of Python
overhead. For Monte Carlo, the binding's median was 5.10 ms, with occasional
slower calls raising its mean. Constraining all benchmark processes to performance
cores produced a 5.51 ms binding mean; it did not reliably meet 5 ms, so the
engine does not set affinity automatically.
[Affinity experiment](../evidence/prepared-affinity-performance.json).

The tests ran on Windows, Intel Core i7-14700K, Rust 1.97.1. Hardware, system
load, roster, course and equipped skills affect latency. The small nine-runner
case does not establish sub-5 ms performance for larger fields or longer races.

## Build and output options

```powershell
cargo build --release --locked --features fast-alloc
```

Omit `--features fast-alloc` to use the platform allocator. The optional mimalloc
allocator requires a C toolchain; its [license](../licenses/mimalloc.txt) and
[binding license](../licenses/mimalloc-rust.txt) are included. Allocator gains
were measured on Windows.

For profile-guided compilation of the worker and prepared library:

```powershell
rustup component add llvm-tools-preview
python tools/build_pgo.py --inputs path/to/profile-races.jsonl --features fast-alloc --train-binary --train-native --output target/pgo/race_engine_rs_v2.exe --library-output target/pgo/globalsim.dll --benchmark-output target/pgo/benchmark_prepared.exe
```

Use representative courses, rosters, skills and frame settings. Repeat `--inputs`
to combine files. Training guides compiler code generation; it does not change
race rules or fit outcomes. `--train-native` exercises prepared calls three times
per input. Add `--native` only when deployment supports the build CPU's features.

JSON remains the default worker output. Set `GLOBALSIM_OUTPUT=binary-v1` for the
[lossless binary protocol](BINARY-REPLAY.md). Set `GLOBALSIM_REPLAY_WORKER=1` to
overlap JSON encoding with simulation for eligible full-frame requests. Binary
output does not use that helper. Prepared calls bypass environment output
settings and expose numeric results or explicitly selected serialization.

The JSON and binary interfaces preserve numeric precision, timestep, RNG draws,
event order, results and frames. Binary Monte Carlo replies average 0.54 MB,
compared with 1.57 MB in JSON in the earlier output-format comparison.
[Output-format evidence](../evidence/architecture-performance.json).

Independent seeds can also run concurrently, using separate prepared handles on
separate threads or separate persistent workers. Report batch throughput and
individual request latency separately. The existing worker-pool benchmark is
`tools/benchmark_throughput.py`.

See [Testing](TESTING.md) for regression coverage and [Accuracy](ACCURACY.md) for
the limits of the engine's reconstruction.
