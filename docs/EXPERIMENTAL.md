# Experimental profiles

GlobalSim includes optional Cargo features for alternative race behavior and
third-party simulator compatibility. They use the same worker and prepared
library interfaces as the default build, but can produce different race results.
Some combinations have known accuracy regressions. Default-build validation
results do not apply to these profiles; use the default build for normal use.

## Features

| Cargo feature | Behavior |
|---|---|
| `experimental-umamoe` | Aggregate profile for uma.moe comparisons; enables `experimental-gap-refresh` and `experimental-duel-hp`. |
| `experimental-gap-refresh` | Alternative opening-phase lane-search behavior. |
| `experimental-duel-hp` | Alternative duel-stamina calculation. |
| `experimental-pacemaker-staging` | Alternative timing of pacemaker restoration. |
| `experimental-spot-cutoff` | Alternative evaluation of lead-competition entry eligibility. |
| `experimental-spot-entry-floor` | Alternative precision for lead-competition entry distances. |
| `experimental-pacing-log` | Alternative logarithm implementation for pacing lotteries. |

Each component can be enabled independently. Only gap refresh and duel-stamina
arithmetic belong to the `experimental-umamoe` aggregate. The remaining features
require explicit selection. These compile-time settings apply to both the
worker and the prepared library, across every seed; request-level runtime
settings still select the applicable simulation path.

## Build and run

Build the aggregate in a separate target directory:

```sh
cargo test --release --locked --features experimental-umamoe --target-dir target/experimental-umamoe
cargo build --release --locked --features experimental-umamoe --target-dir target/experimental-umamoe
```

To select an individual component:

```sh
cargo test --release --locked --features experimental-pacing-log --target-dir target/experimental-pacing-log
cargo build --release --locked --features experimental-pacing-log --target-dir target/experimental-pacing-log
```

Pass a comma-separated list to `--features` to combine components. Use the worker
or library from that target directory's `release` folder. Build release
artifacts after running tests so the test library's panic configuration does
not replace the library intended for use.

For example, run the aggregate worker on Windows:

```powershell
Get-Content examples/v77-race.jsonl | .\target\experimental-umamoe\release\race_engine_rs_v2.exe
```

On Linux or macOS:

```sh
./target/experimental-umamoe/release/race_engine_rs_v2 < examples/v77-race.jsonl
```

The standard regression checker also accepts an experimental build:

```sh
python tools/check_accuracy_regressions.py --engine target/experimental-umamoe/release/race_engine_rs_v2.exe --report target/experimental-umamoe/accuracy.json
```

Omit `.exe` on Linux or macOS. A profile may pass its unit tests while failing
captured-race regressions; the checker retains its strict comparisons and
reports those failures. See [Testing](TESTING.md) and [Accuracy](ACCURACY.md).

## Attribution

The experimental pacing logarithm's range reduction and split constants are
adapted from SunPro's `log10f` implementation. Its copyright, permission notice
and source attribution are retained in [pacing-log.txt](../licenses/pacing-log.txt)
and [NOTICE](../NOTICE).
