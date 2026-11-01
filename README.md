# GlobalSim

### High-fidelity race simulation for Umamusume Global

**Version 76**

GlobalSim is a fast, detailed race simulation engine written in Rust. It models
runner attributes, course geometry, positioning, skill activation, stamina and
the final sprint to produce reproducible races from seeded inputs.

Designed for race analysis, strategy exploration and Monte Carlo workloads,
GlobalSim offers complete movement replays, race events and finishing results
through a persistent worker or a prepared library API.

## Race mechanics

- **Runner attributes:** speed, stamina, power, guts, wisdom, mood and aptitudes.
- **Course physics:** distance, surface, ground conditions, slopes, corners and track geometry.
- **Field interactions:** pacing, overtaking, lane selection, blocking and competition.
- **Stamina:** consumption, recovery, rushing, Conserve Power and last-spurt planning.
- **Skills:** activation conditions, timing, effect values, duration, cooldowns and recipients.
- **Replay output:** finish times, placements, movement frames, skills and competition events.

## Performance and fidelity

Prepared races reuse course, roster and skill setup across seeds. Persistent
workers support concurrent seed batches, full replays and results-only output.
Every seed receives independent mutable race and activation state.

The current engine includes refined pacemaker retention and passing-route
selection. Its latest validation records:

| Check | Result |
|---|---:|
| Official practice regression cohort | **52,096/52,096** matching recorded observations |
| Included official accuracy fixtures | **82/82** passing |
| Rust unit tests | **165/165** passing |

These results describe the tested configurations and recorded observations.
GlobalSim is an unofficial implementation and does not claim universal 1:1
equivalence. [Accuracy and coverage](docs/ACCURACY.md) distinguishes official
captures, third-party comparisons and known limitations.
[Performance](docs/PERFORMANCE.md) reports measured latency and benchmark scope.

An optional [experimental uma.moe profile](docs/EXPERIMENTAL.md) improves agreement
with exported third-party races. It has known differences from official captures
and requires an explicit build feature.

## Getting started

Build with a Rust toolchain, Cargo, a native linker and a C compiler:

```sh
cargo build --release --locked
```

The default build uses mimalloc. Add `--no-default-features` to use the platform
allocator instead.

Run the included v76 example on Windows:

```powershell
Get-Content examples/v76-race.jsonl | .\target\release\race_engine_rs_v2.exe
```

On Linux or macOS:

```sh
./target/release/race_engine_rs_v2 < examples/v76-race.jsonl > results.jsonl
```

Start from this example to retain the runtime settings and skill table used by
the validated configuration. Adapt its course, runners, skills and seed to
explore other races. Omitting those settings selects a different runtime profile.

## Integration

The worker accepts one race specification per line of JSON and returns one
result per race. A `seeds` array runs one setup across multiple seeds; `threads`
controls concurrency and `omitFrames` selects results-only output.

The [prepared Rust and C API](docs/IN-PROCESS.md) reuses race setup and exposes
numeric replay frames directly. A Python binding and a
[lossless binary worker interface](docs/BINARY-REPLAY.md) support analysis tools.
See the [input/output reference](docs/INTERFACE.md) for request fields and errors.

## Documentation

- [Race mathematics](docs/MATHEMATICS.md)
- [Positioning, movement and field mechanics](docs/FIELD-MECHANICS.md)
- [Skills and activation rules](docs/SKILLS.md)
- [Architecture and simulation order](docs/ARCHITECTURE.md)
- [Parameter reference](docs/PARAMETERS.md)
- [Source reference](docs/SOURCE-INDEX.md)
- [Performance and benchmarking](docs/PERFORMANCE.md)
- [Accuracy and coverage](docs/ACCURACY.md)
- [Testing](docs/TESTING.md)
- [Complete documentation index](docs/README.md)
