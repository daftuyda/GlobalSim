# GlobalSim

### High-fidelity race simulation for Umamusume Global

**Version 73**

GlobalSim is a detailed race simulation engine written in Rust, modeling the
interactions that decide a race—from starting gates and positioning to skill
activation, stamina management, and the final sprint.

Built for race analysis, strategy exploration, and simulation-driven tools,
GlobalSim produces reproducible results from seeded inputs, with detailed
trajectories, finishing orders, and race events.

## Race mechanics

- **Runner attributes:** speed, stamina, power, guts, wisdom, mood, and aptitudes.
- **Course physics:** distance, surface, ground conditions, slopes, corners, and track geometry.
- **Field interactions:** positioning, pacing, overtaking, lane selection, blocking, and competition.
- **Stamina and race phases:** consumption, recovery, rushing, Conserve Power, and last-spurt planning.
- **Skill simulation:** activation conditions, timing, effect values, duration, cooldowns, and recipients.
- **Detailed output:** finish times, placements, movement frames, and skill and competition events.

## Improvements in v73

Skill effects preserve queued bonuses through their final update. Front-blocking
boundaries and leader-gap duration scaling retain the precision needed for
consistent behavior in edge cases. The prepared API provides complete replay
frames in under 5 ms on the measured nine-runner workload.
See [performance measurements](docs/PERFORMANCE.md).

## Measured fidelity

Results across **47,664 captured official Global races**:

| Metric | Result |
|---|---:|
| Races matching every audited replay check | **47,664/47,664** |
| Races with every finish time matching exactly | **47,664/47,664** |
| Winner agreement | **47,664/47,664** |
| Exact placement agreement | **435,508/435,508** |
| Worst observed finish-time difference | **0 ms** |

These measurements describe the tested races, not guaranteed accuracy for every
race configuration. GlobalSim is an unofficial implementation and does not claim
1:1 equivalence with the game. See [benchmark methodology and coverage](docs/ACCURACY.md).

## Getting started

Build with a Rust toolchain, Cargo, and a native linker:

```sh
cargo build --release --locked
```

Run the included example on Windows:

```powershell
Get-Content examples/v73-race.jsonl | .\target\release\race_engine_rs_v2.exe
```

On Linux or macOS:

```sh
./target/release/race_engine_rs_v2 < examples/v73-race.jsonl > results.jsonl
```

The example includes the simulation settings and skill data used by the
benchmarked configuration. Adapt its course, runners, skills, and seed to
explore other races.

## Integration

By default, GlobalSim accepts one race specification per line of JSON and returns one JSON
result per race. A persistent worker supports repeated requests, making it
suitable for batch simulations and integration into analysis tools.

The [prepared Rust and C API](docs/IN-PROCESS.md) reuses race setup across seeds
and provides direct access to numeric replay frames for low-latency integration.

Use a fixed seed and identical inputs for reproducible comparisons. Detailed
movement frames and event records support analysis beyond the finishing order.
See the [input/output reference](docs/INTERFACE.md) for settings, result fields,
and error handling.

## Documentation

- [Race mathematics](docs/MATHEMATICS.md)
- [Positioning, movement, and field mechanics](docs/FIELD-MECHANICS.md)
- [Skills and activation rules](docs/SKILLS.md)
- [Architecture and simulation order](docs/ARCHITECTURE.md)
- [Input/output reference](docs/INTERFACE.md)
- [Parameter reference](docs/PARAMETERS.md)
- [Source reference](docs/SOURCE-INDEX.md)
- [Performance and benchmarking](docs/PERFORMANCE.md)
- [Benchmark methodology and coverage](docs/ACCURACY.md)
