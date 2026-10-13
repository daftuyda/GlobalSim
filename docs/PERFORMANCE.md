# Performance

A persistent worker reuses immutable course data and geometry between races.
Skill definitions are read directly from each request's parsed JSON. Movement
is recorded as compact numeric snapshots, with JSON objects created only for
frames included in the response. Tick, live-runner and lane-search buffers reuse storage,
and skill construction borrows race parameters. ASCII JSON tokens are read
directly from the input buffer, and strings that need no escaping are written
in one operation. Overtaking checks defer unnecessary blocker scans.
Simulation steps, RNG and race mechanics are unchanged.

## Local measurements

Thirty nine-runner races with skills, repeated five times after five warmup
requests in a persistent Windows worker on an Intel Core i7-14700K:

| Output | Mean latency | 95th percentile | Throughput |
|---|---:|---:|---:|
| Standard replay frames | 15.3 ms/race | 19.4 ms | 65.4 races/second |
| Every simulation frame | 27.0 ms/race | 34.7 ms | 37.1 races/second |

Measurements include worker request parsing, simulation, response serialization
and pipe transfer. Input preparation and caller-side result parsing are excluded.
These are sequential worker measurements, not aggregate multi-worker throughput.
Latency varies with hardware, machine load, runner count and skill workload.

The full 13,187-race corpus produces byte-identical dense output to the v71
recorded reference outputs. Another 270 replies cover both output modes and request
isolation, including expected invalid-request errors. See
[full validation](../evidence/full-validation.json) and
[measurement details](../evidence/performance.json).

## Reproduce

The optional benchmark tool uses Python's standard library. From the repository
root, after building the release executable:

```powershell
python tools/benchmark.py --engine target/release/race_engine_rs_v2.exe --inputs examples/v71-race.jsonl --repeats 50
```

Add `--full-frames` for dense movement output. Add `--baseline PATH` to compare
another executable; every measured reply must match exactly. Supply a JSON-lines
file containing representative races with skills for workload-specific results.
The included example has nine runners without skills, so it is only a quick
check rather than the workload used in the table.

The first request includes worker initialization and is reported separately
from the warmed average. Keep the worker alive across requests to benefit from
cached data. Changing the skill table takes effect on the next request; caching
never substitutes a previous request's skill definitions.
