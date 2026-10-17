# GlobalSim

### High-fidelity race simulation for Umamusume Global

**Version 74**

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

## Improvements in v74

Version 74 refines pacing, overtaking, lane selection, skill timing, course
transitions, and final-sprint planning. Expanded coverage includes 203,572
official race captures across 105 courses and fields of 9 to 18 runners.

## Measured fidelity

Results across **203,572 captured official Global races**:

| Metric | Result |
|---|---:|
| Races matching every audited replay check | **203,572/203,572** |
| Races with every finish time matching exactly | **203,572/203,572** |
| Winner agreement | **203,572/203,572** |
| Exact placement agreement | **2,210,585/2,210,585** |
| Worst observed finish-time difference | **0 ms** |

These measurements describe the tested captures at their recorded precision and
cadence. GlobalSim is an unofficial implementation and does not claim 1:1
equivalence with the game across every race configuration.
See [benchmark methodology and coverage](docs/ACCURACY.md).

## Getting started

Build with a Rust toolchain, Cargo, and a native linker:

```sh
cargo build --release --locked
```

Run the included example on Windows:

```powershell
Get-Content examples/v74-race.jsonl | .\target\release\race_engine_rs_v2.exe
```

On Linux or macOS:

```sh
./target/release/race_engine_rs_v2 < examples/v74-race.jsonl > results.jsonl
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
