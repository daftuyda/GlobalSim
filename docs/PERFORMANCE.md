# Performance

See the [performance audit](PERFORMANCE-AUDIT.md) for a measured breakdown of
request processing, simulation ticks, replay output and further optimization options.

A persistent worker reuses immutable course data and geometry between races.
Identical `skillData` text shares an immutable parsed tree between requests.
Changed text is parsed normally, and decoded definitions are reused only when
the complete parsed content matches. Whole replay frames serialize directly from
contiguous numeric samples. Tick, rank, live-runner and lane-search buffers reuse
storage; duel and passing candidates are selected without temporary vectors.
Numeric skill IDs are parsed once, movement modifiers use iterators, and maximum
spurt speed is computed when the spurt plan changes. These optimizations preserve
simulation steps, RNG, update order and floating-point addition order.

## Local measurements

Thirty races per field-size group, repeated five times after five warmup
requests in persistent Windows workers on an Intel Core i7-14700K. The engine
order alternates for each request:

The accuracy corrections have a small measured effect on latency:

| Workload | Before accuracy corrections | Current | Change |
|---|---:|---:|---:|
| 9 runners, standard frames | 10.31 ms | 10.27 ms | -0.4% |
| 9 runners, every frame | 17.33 ms | 17.28 ms | -0.3% |
| 18 runners, standard frames | 38.80 ms | 38.82 ms | +0.04% |
| 18 runners, every frame | 51.42 ms | 52.44 ms | +2.0% |

Measurements include worker request parsing, simulation, response serialization
and pipe transfer. Input preparation and caller-side result parsing are excluded.
All 600 timed replies per executable match for these benchmark inputs. The
dense eighteen-runner workload measured 2% slower; the other means changed by
less than 0.5%. These small differences come from one paired run. Compare paired
engines within the same run rather than absolute timings from earlier sessions.
Latency varies with hardware, machine load, course, runner count and skills.
These measurements do not establish aggregate multi-worker throughput.

The full **13,187-race corpus** now matches official finish times, skill starts
and every compared replay sample. Four races are corrected, while the other
13,183 dense replies remain byte-identical. Another **292 replies** preserve
both output modes and request isolation, including expected invalid-request
errors. See [full validation](../evidence/accuracy-validation.json) and
[measurement details](../evidence/accuracy-performance.json). The preceding
[performance audit](PERFORMANCE-AUDIT.md) records optimization savings, detailed
phase timings and the additional raw-wire parser checks.

## Reproduce

The optional benchmark tool uses Python's standard library. From the repository
root, after building the release executable:

```powershell
python tools/benchmark.py --engine target/release/race_engine_rs_v2.exe --inputs examples/v72-race.jsonl --repeats 50
```

Add `--full-frames` for dense movement output. Add `--baseline PATH` to compare
another executable; every measured reply must match exactly. Supply a JSON-lines
file containing representative races with skills for workload-specific results.
The included example has nine runners without skills, so it is only a quick
check rather than the workload used in the table.

The first request includes worker initialization and is reported separately
from the warmed average. Keep the worker alive across requests to benefit from
cached data. The main workload reuses a 730-definition skill table. Changing its
text requires parsing again; changing its content can also require decoding.
The [audit](PERFORMANCE-AUDIT.md) includes changed-table, Unicode and alternating
formatting workloads. Changing or omitting the skill table takes effect on the
next request. The caches compare actual text and content, including float bits,
rather than trusting `dataSignature`.
