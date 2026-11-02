# JSON-lines interface

JSON is the default output. The optional [binary replay interface](BINARY-REPLAY.md)
accepts the same requests and returns exact numeric frames with lower output cost.

The binary reads stdin until EOF and emits one compact JSON reply for each
nonempty input line, flushing after each reply. Multiline pretty-printed JSON
is not a request. No command-line mode flag is needed. Use the provided
[default example](../examples/race.jsonl) or
[benchmark example](../examples/v77-race.jsonl).

The example includes the skill table and settings used for the benchmark.
`examples/v77-settings.json` lists switches separately;
`data/v70-skill-data.json` contains that table. The built-in `data/skill_data.json`
is the legacy fallback: omitting `skillData` does not select the audited table.

## Required fields

| Field | Meaning |
|---|---|
| `seed` | Integer seed; reconstructed initialization requires signed 32-bit |
| `courseId` | Key in bundled `data/course_data.json` |
| `groundCondition` | Good, Yielding, Soft, Heavy |
| `weather` | Sunny, Cloudy, Rainy, Snowy |
| `season` | Spring, Summer, Autumn, Winter, Sakura |
| `horses` | Nonempty array of runner objects, in reproducible order |

Each runner requires numeric `speed`, `stamina`, `power`, `guts`, `wisdom`;
`strategy` (use canonical Nige, Senkou, Sasi, Oikomi, Oonige); and
`distanceAptitude`, `surfaceAptitude`, `strategyAptitude` (S through G).
Canonical strategy spelling matters for output metadata even where parsing
accepts aliases/case variants.

Optional runner fields include `mood` (default 0, normally -2 through +2),
`gateIndex` (default array index), `popularity` (0), `teamId` (0), `charaId` (0),
`fans` (0), `careerWins` (0), `teamRank` (0), and `skills` (empty).
Skills can be IDs or objects such as `{"skillId":"123456","level":1,"isUnique":false}`.
Use real IDs from the skill table. `teamRank` accepts nonnegative 32-bit integers;
zero means no team bonus. The race bonus is +50 for ranks 1–5, +30 for 6–10,
+20 for 11–15, +10 for 16–20, and zero from 21 onward. It applies to each
base stat after overcap halving and before motivation, including skill-lottery
wisdom and team-stat skill values. Supply the captured rank rather than adding
the bonus to raw stats yourself. The separate `singleMode` bonus remains +400
at the adjusted-stat stage. Both JSON workers and the prepared API support ranks.
Nonzero ranks have synthetic regression coverage; official career capture
validation, including exact tier boundaries, remains outstanding.

## Optional request fields

| Field | Default / purpose |
|---|---|
| `time` | midday; also morning, evening, night, none |
| `grade` | g1; see `parse_grade` for supported names |
| `isDirtGrade` | Optional boolean override |
| `playerHorseIndex` | 0; integer within roster, controls player telemetry |
| `wisdomChecks` | true |
| `singleMode` | false; career adjusted-stat bonus |
| `skillData` | Replacement table in the same shape as bundled skill data |
| `dataSignature` | Echoed in output for caller bookkeeping |
| `traceConservePower` | Optional boolean diagnostic timeline |
| `retainAllFrames` | false; true returns dense frames |
| `experimentalRetailInitialization` | false; reconstructed initialization |
| `experimentalRetailSections` | false; reconstructed sections, implies initialization |
| `experimentalRetailRuntime` | false; reconstructed whole runtime, implies initialization |
| `experimentalRetailStartup` | false; requires reconstructed runtime |
| `experimentalRetailGeometry` | false; requires reconstructed runtime and standard grid |
| `experimentalLaterStaminaMechanics` | false; enables additional approximate mechanics |
| `experimentalGrid` | Optional `{"columns":20,"rowSpacing":3.0}` for 21–120 runners |

Flags select simulation behavior. Geometry requires a
bundled geometry asset for that course. Oversized grids are custom behavior
with no official oversized-field benchmark. Defaults apply when a setting is omitted from the request.

## Seed sampling

The worker accepts three sampling keys. They are removed before the request is
read as a race.

| Field | Default / purpose |
|---|---|
| `seeds` | Absent. A non-empty array: one reply per entry, in order, each identical to the request sent alone with that `seed` |
| `threads` | 1. Threads that share one `seeds` batch (1–256; 0 uses every core) |
| `omitFrames` | false. true omits `frames`, `finishFrames`, `rawFrameCount`, `framesDownsampled` and `gateFrameNeeded` and skips capturing them. Binary packets then carry zero frames |

A batch reply that fails, including a seed outside the retail 32-bit range, is
an `{"error":...}` reply in that seed's place, so a reader always receives one
reply per seed. A malformed sampling key returns one error reply. The worker
also keeps the most recent setup (the request minus `seed`) prepared once it
repeats, so lone requests that change only `seed` reuse it too. Every reply
matches the per-request path byte for byte.

## Reply

`results` retains runner-array identity and contains the finish/rank metadata;
`frames` contains time-indexed runner observations. `skillEvents`,
`skillDetailEvents`, `skillTargetEvents`, `additionalTargetEvents`,
`competitionEvents` and `conservePowerEvents` expose supported events.
`playerLastSpurtTime` and `playerHorseIndex` describe player telemetry.
Capability/version fields describe output behavior, not official-game parity.
Consult the serialization block in [`lib.rs`](../src/lib.rs) for every field.

Normal output includes `framesDownsampled`, `gateFrameNeeded` and preserved
finish samples. Callers encoding replay data must not thin frames a second
time. Use `finishTimeRaw` for numerical comparisons where provided; rounded
display times and encoded scenario values are separate products.

`finishTimeRaw` is the interpolated elapsed crossing time. `finishTime` is the
enclosing simulation tick end. Neither field applies the `FinishTimeCoef`
parameter; consumers requiring a display-clock conversion must handle it
separately.

Recoverable parse/build errors return `{"error":"..."}` and processing
continues. The worker is **not a hardened validation boundary**: some invalid
inputs, such as an unknown course reaching `get_course`, can panic. Release
builds use panic=abort. Validate user inputs, bound request sizes, check process
exit and restart the worker when integrating it.
