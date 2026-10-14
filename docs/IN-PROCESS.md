# Prepared race API

Use the prepared API when many seeds share the same course, roster, skills and
settings. It decodes those inputs once and returns each race directly to the
calling application. The simulation still creates fresh RNG streams, sampled
triggers, runners, timers, activation observers and results for every seed.

`retainAllFrames: true` retains every simulation frame. The default frame
selection is the same as the worker interface. Preparing a different roster,
skill table or setting requires a new preparation; only the seed varies during
`run` calls.

## Rust

The library crate is `globalsim`, in the `race_engine_rs` Cargo package. A local
dependency can be declared as:

```toml
[dependencies]
globalsim = { package = "race_engine_rs", path = "../GlobalSim", features = ["fast-alloc"] }
```

The optional `fast-alloc` feature selects the library's global allocator. An
application that supplies its own global allocator should omit this feature.

```rust
use globalsim::PreparedRace;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // One conventional race request, with retainAllFrames set as desired.
    let input = std::fs::read_to_string("race.json")?;
    let mut race = PreparedRace::new(&input)?;
    for seed in [100003, 107922, 115841] {
        let result = race.run(seed)?;
        let frames = result.frames();
        for index in 0..frames.len() {
            let (time, runners) = frames.frame(index);
            // Each runner: [position, speed, HP, lane, rushing mode, front blocker].
            std::hint::black_box((time, runners));
        }
    }
    Ok(())
}
```

Each `Simulation` owns its result; borrowed frame views remain valid while it
lives. Keeping a result does not prevent running the next seed. `write_json`
replaces a supplied `String` with a conventional JSON reply. `into_binary`
consumes the result and replaces supplied metadata and packet buffers with a
[binary replay](BINARY-REPLAY.md), reusing their capacity.

`PreparedRace` is confined to its creating thread through its Rust types. Create
one preparation per worker thread. Skill definitions are scoped to each run, so
independent handles can use different tables concurrently.

## Shared library and Python

```powershell
cargo build --release --locked --features fast-alloc
```

This builds the command-line engine and `globalsim.dll` on Windows,
`libglobalsim.so` on Linux, or `libglobalsim.dylib` on macOS. The current performance
and library regression measurements were run on Windows.

The [Python binding](../tools/native_engine.py) uses the standard-library `ctypes`
module. Add `GlobalSim/tools` to your application's module path:

```python
import json
from native_engine import NativeRace

with open("race.json", encoding="utf-8") as source:
    spec = json.load(source)
spec["retainAllFrames"] = True

with NativeRace("target/pgo/globalsim.dll", spec) as race:
    for seed in [100003, 107922, 115841]:
        replay = race.run(seed)
        time, runner_values = replay.frame(0)
```

`run_wire(seed)` returns the complete binary packet as owned `bytes`; `run(seed)`
also decodes its metadata and exposes a read-only frame view. Returned packets
and views remain valid after later runs or after closing the native handle.
NumPy consumers can use the [numeric buffer view](BINARY-REPLAY.md#reading-a-reply).

Use a handle on its creating thread and close it there, preferably with `with`.
Separate threads can create independent handles. `prepare(spec)` replaces the
handle's race. A failed preparation clears the preceding race; a failed run
clears its preceding reply. Recoverable errors raise `ValueError` and permit a
subsequent valid preparation or seed. Retail seeds must fit a signed 32-bit integer.

## C interface

Declarations are in [globalsim.h](../include/globalsim.h). The exported functions
use the C calling convention:

| Function | Operation |
|---|---|
| `globalsim_new` | Allocate a handle on the calling thread |
| `globalsim_prepare` | Decode a JSON request from a pointer and byte length |
| `globalsim_run` | Simulate an independent seed and encode its binary packet |
| `globalsim_data`, `globalsim_size` | Borrow the packet pointer and length |
| `globalsim_error` | Borrow a NUL-terminated UTF-8 error description |
| `globalsim_free` | Release a handle on its creating thread |

Status codes are `0` for success, `-1` for a recoverable error and `-2` for an
incorrect thread. Pass valid live handles and valid input buffers; never access
the same handle concurrently or free it twice. Borrowed packet and error pointers
expire at the next prepare, run or free call for their handle. Copy the packet
before then if it must outlive that call. The Python binding performs this copy.

As with other native libraries, an unrecoverable allocation failure or engine
panic terminates the host process. The worker interface provides process isolation
when that is required by the application.

## Build and measure

Train on representative inputs for both worker and prepared paths:

```powershell
python tools/build_pgo.py --inputs path/to/profile-races.jsonl --features fast-alloc --train-binary --train-native --output target/pgo/race_engine_rs_v2.exe --library-output target/pgo/globalsim.dll --benchmark-output target/pgo/benchmark_prepared.exe
```

Measure the Python library binding against a persistent binary worker:

```powershell
python tools/benchmark_native.py --library target/pgo/globalsim.dll --baseline target/pgo/race_engine_rs_v2.exe --inputs path/to/race.json --mc-seeds 30 --repeats 10
```

This includes binary encoding and copying all reply bytes into Python. Preparation
is reported separately, decoding is excluded, and every returned packet must
match the worker byte for byte.

The [Rust benchmark](../examples/benchmark_prepared.rs) exercises 30 seeds per
repetition after five warmup races. Supply one race with `retainAllFrames: true`:

```powershell
target/pgo/benchmark_prepared.exe path/to/race.json 10 frames
target/pgo/benchmark_prepared.exe path/to/race.json 10 binary
```

`frames` times simulation with owned numeric frames and result cleanup; `binary`
also includes complete packet encoding. Neither includes process transport or
application analysis. Consult [Performance](PERFORMANCE.md) for measured results
and the separate preparation cost.
