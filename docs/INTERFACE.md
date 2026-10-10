# JSON-lines interface

The binary reads stdin until EOF and emits one compact JSON reply for each
nonempty input line, flushing after each reply. Multiline pretty-printed JSON
is not a request. No command-line mode flag is needed. Use the provided
[default example](../examples/race.jsonl) or
[v70 example](../examples/v70-race.jsonl).

The example includes the skill table and settings used for the benchmark.
`examples/v70-settings.json` lists switches separately;
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
`fans` (0), `careerWins` (0), and `skills` (empty).
Skills can be IDs or objects such as `{"skillId":"123456","level":1,"isUnique":false}`.
Use real IDs from the skill table. Nonzero `teamRank` is rejected by the
experimental runtime because that bonus integration is unvalidated.

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

## Reply

`results` retains runner-array identity and contains the finish/rank metadata;
`frames` contains time-indexed runner observations. `skillEvents`,
`skillDetailEvents`, `skillTargetEvents`, `additionalTargetEvents`,
`competitionEvents` and `conservePowerEvents` expose supported events.
`playerLastSpurtTime` and `playerHorseIndex` describe player telemetry.
Capability/version fields describe output behavior, not official-game parity.
Consult the serialization block in [`main.rs`](../src/main.rs) for every field.

Normal output includes `framesDownsampled`, `gateFrameNeeded` and preserved
finish samples. Callers encoding replay data must not thin frames a second
time. Use `finishTimeRaw` for numerical comparisons where provided; rounded
display times and encoded scenario values are separate products.

Recoverable parse/build errors return `{"error":"..."}` and processing
continues. The worker is **not a hardened validation boundary**: some invalid
inputs, such as an unknown course reaching `get_course`, can panic. Release
builds use panic=abort. Validate user inputs, bound request sizes, check process
exit and restart the worker when integrating it.
