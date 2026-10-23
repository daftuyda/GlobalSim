# Testing

Run the Rust test suite with:

```sh
cargo test --release --locked
```

The library suite passes in full: **155 of 155 tests** with `fast-alloc`, and
all **81 included strict accuracy fixtures pass**. The latest boundary unit
test and three captured fixtures cover the [passing-corridor rounding
correction](CORRIDOR-FIX-20261023.md). The preceding three unit tests and
four captured fixtures cover competition recording order; see the
[practice correction and quarantine](PRACTICE-FIXES-20261022.md).
Two preceding unit tests check rushing section rounding and an absent rushing
section; an added
capture checks its effect on a complete 18-runner race. The preceding 149-test
suite also passed with the default allocator. Four earlier captures cover
restoration to the nearest forward-style runner. See the
[latest batch and remaining discrepancies](NEW-RACES-20261022.md) and the
[pacemaker correction](PACING-FIX-20261021.md); earlier results are retained in
[accuracy](ACCURACY.md).

### Former failures (resolved 2026-10-20)

Earlier releases reported 11 failing unit tests, failing since the first
published commit. None indicated an engine defect; the tests were outdated:

- Two duel tests predated rules the engine adopted deliberately: v6 removed
  duel target retention, and v71 made the 5 m separation boundary inclusive.
  Their expectations were corrected.
- The lane-width check expected decimal `1.2`; widths are basis points times
  `0.0001f` (doc 13.186), which is `1.1999999` for 12000.
- Eight tests compared bits with the reconstructed Python port. The engine
  intentionally rounds base target speed, maximum spurt speed, initial HP, HP
  drain, the HP guts factor, acceleration, lane width, world position, world
  distance correction, lateral movement and the blocking cap differently.
  Reverting each one alone to the Python arithmetic reduced the strict
  fixtures from 69 to between 3 and 68 passing. The blocking cap was the
  exception: the fixtures do not distinguish its two forms. The affected expected
  values in `retail_physics_port`, `retail_hp_port`, `retail_geometry_port`,
  `retail_lane_port`, `retail_solver_port` and `retail_field_port` were
  regenerated from the engine. Inputs, and sections that still match the
  Python port, are unchanged; each fixture's `provenance` field says which
  parts are engine snapshots. These tests now guard against unintended
  numeric changes rather than check agreement with Python.

The v75 release suite had **138 passing tests and 11 failures**.
[v75 release checks](../evidence/v75-release-validation.json).
The earlier `fast-alloc` release suite had **136 passing tests and 11 pre-existing failures**.
The earlier default-allocator accuracy build had 133 passing tests and the same 11 failures.
The failure set is unchanged. Failures cover
numerical reference comparisons, geometry, stamina and spurt behavior,
lateral movement, and duel conditions.
[Default allocator log](../evidence/morning-batch-20261017-default-tests.txt),
[fast allocator log](../evidence/morning-batch-20261017-fast-tests.txt).

Unit tests, persistent-worker checks and [official-race benchmark results](ACCURACY.md)
measure different aspects of the engine and should be considered together.

The earlier Career candidate passed **67 captured regressions** under the earlier
pre-finish checks. The separate practice benchmark recorded matches for
**203,572 of 203,572 captured official races** under its earlier definitions;
that full benchmark has not been rerun for the Career corrections.
[Practice validation](../evidence/new-races-20261017-morning.json).

JSON and binary output agree across 13 requests per frame mode, including
changed skill data, invalid/empty tables, recovery and runner permutations.
The binary helper produces identical packets with both sparse and full frames.
[Current protocol checks](../evidence/morning-batch-20261017-protocol.json).

The v73 prepared library matches 843 runs per frame mode against the ordinary
release worker, plus 24 concurrent runs and handle isolation/recovery checks.
The binary protocol check matches 281 replies per frame mode, including expected
errors. [Library checks](../evidence/parity-native-parity.json),
[protocol checks](../evidence/parity-wire-parity.json).

The ordinary release build and fast-allocator PGO build preserve **292 matching
JSON replies per configuration** across captured races, edge cases and worker
isolation checks. Coverage includes changed definitions under the same signature,
omitted/empty/invalid tables and recovery, runner ordering and runtime modes.
The PGO helper is checked separately. Binary output preserves all metadata and
frame values across the same 292 cases, with the helper setting both disabled
and enabled. Expected error replies also match.
[Generic checks](../evidence/prepared-generic-worker.json),
[PGO checks](../evidence/prepared-pgo-worker.json),
[helper checks](../evidence/prepared-helper-worker.json),
[binary checks](../evidence/prepared-protocol-check.json).

The v73 ordinary release build matches **186 raw-wire replies**, including 78 expected errors, covering
Unicode, whitespace, escaped and duplicate keys, malformed requests, numeric
token boundaries and recovery. [Raw-wire results](../evidence/prepared-wire-parity.json).

Passing unit tests cover whole-frame float formatting, blocker-index and
runner-order restoration, exact nested skill-data comparison, and shared parser
cache identity, invalidation and error behavior. Lane and scoring checks also
exercise iterator-based modifiers and repeated rank updates with ties.
Existing tests cover frame cadence, finish samples, dense output, course-cache
identity, numeric precision, Unicode, escapes, parser errors and deferred blocker
checks.

Additional focused coverage includes:

- Shared condition trees with independent sampled windows and activation
  observers, unchanged sampling errors, and separate runtime/table identities.
- Priority and prerequisite indexes preserve observation order, pending removals,
  activation history, cooldown state and RNG state. Empty priority-zero passes
  still flush required bookkeeping; stateful observations continue during cooldown.
  Compact window checks match full visits for inactive and reversed windows,
  non-finite boundaries, removals, random reservations and additional effects.
- Prepared roster facts preserve self exclusions, duplicate skill handling,
  favorite selection and ordered team-stat sums in both runtimes. Sampled-trigger
  indexing preserves expression-node order and parser validation errors.
- The evolving movement roster matches fresh field reads through ordered updates
  and callback invalidation. Blocking samples and passing membership match live
  reference queries at each tested tick, including callbacks that change peer
  targets. Membership tests cover owner exclusions, replacements, duplicates
  and fields up to 128 runners. Acceleration caches refresh for every dependency and
  parameter-object replacement. HP factor caching preserves reinitialization and
  phase boundaries, and the target-speed shortcut preserves clamped results.
- Bitwise geometry interpolation comparisons across all 119 courses, including
  overrun curves, adjacent samples and exact endpoints.
- Specialized number formatting against the standard formatter, frame-text cache
  transitions, changing runner counts, non-finite values and response-buffer reuse.
- Incremental frame encoding matches complete-array encoding for empty arrays,
  changing runner counts and batch boundaries. Persistent helper tests cover
  buffer reuse and abandoned frame sequences.
- Overtaking decisions that skip irrelevant blocker queries and evaluate necessary
  blocker queries exactly once.

The specialized formatter also matches the original formatter across
**162,383,552 numeric comparisons**, including every positive float32 bit pattern
in its optimized magnitude range and additional sampled values. This is exhaustive
coverage of that positive float32 range, not of all float64 values.
[Formatter validation](../evidence/five-ms-format-validation.json).

To compare two persistent workers on your own inputs:

```powershell
python tools/check_worker_parity.py --engine target/release/race_engine_rs_v2.exe --baseline path/to/previous-engine.exe --inputs examples/v75-race.jsonl
```

Add `--isolation` when supplying captured inputs with `skillData` and equipped
speed or acceleration skills. The tool verifies that changed definitions affect
the chosen race and that restoring the original definitions restores its output.

Set `GLOBALSIM_REPLAY_WORKER=1` before launching the check to exercise the optional
helper. Include full-frame requests that need no gate permutation, sparse and
gate-reordered fallbacks, and alternating request modes. The helper changes
encoding concurrency while retaining the same JSON contract.

## Historical v74 captured-race validation

Full dataset validation covers **203,572 races**, 2,210,585 runners and 266,951,869 compared
pre-finish runner/frame samples. It checks raw finish times, ordering, skill
starts, detail choices, recorded durations, visual recipients, competition
events, movement and recorded runner state. **All 203,572 races pass every check**.
These are regression checks, not a guarantee for unseen races.
See [validation evidence](../evidence/new-races-20261017-morning.json) and the
[comparison definitions](ACCURACY.md).

Sixty-three included regression fixtures cover pacemaker restoration, healer-history
timing for both runner update orders, an exact shared slope boundary,
front-runner competition eligibility, cached leader identity at the 150 m
threshold, invitations to finished duel partners and entry into groups that
remain active past the finish line. They also cover bonuses queued on a skill's
final tick, inclusive and exclusive front-block boundaries, and leader-gap
duration rounding. Further cases cover final-corner lottery eligibility,
overlapping corner/straight events, one-time straight entry, cornerless courses,
cached overtake state, continuous rank history and activations before a running
skill's final ordered turn. Pacemaker cases cover in-range and detached forward
groups, restoration at section boundaries and after selection ends, and the
exact pace-up gap that must not consume a lottery draw. Further fixtures cover
refreshing the handed-over group leader before forward-group restoration and
an empty visible corridor under stacked vision debuffs. Additional fixtures cover
an overtake on the final-corner entry tick, a competition partner beyond the
entry cutoff, post-finish lane movement affecting an unfinished runner, and
a one-float visibility-cone boundary that changes passing direction. Six further
fixtures cover a late gate-skill activation preserving bad-start state, duel
warmup completing after the entrant finishes, opposite spurt HP rounding
boundaries, and section-counter resets after isolated or consecutive pacemaker
refreshes. Three further fixtures cover course/ground speed-stat rounding on
two courses and a counterexample that preserves the wider spurt guts product.
Three new cases cover a returning Runaway retaining the pacemaker incumbent,
HP-percentage truncation before a skill trigger, and a control preserving the
full course-section precision. An additional case covers an inward passing
corridor boundary. Three later captures distinguish the corrected arithmetic:
evaluate the complete shifted bound in float64, then round it once to float32
before the inclusive lane comparison. The boundary unit test also distinguishes
a blocker exactly on that edge from one at the adjacent float below it.
Checks include finish times, skill starts, competition
events, visual recipient masks and sampled movement; recent fixtures also
check recorded skill durations. The engine receives only pre-race inputs;
expected official observations are checked separately.
[Fixture results](../evidence/morning-batch-20261017-regressions.json).

```powershell
python tools/check_accuracy_regressions.py --engine target/release/race_engine_rs_v2.exe
```

The checker now defaults to **strict observation validation**. It uses exact
float32 timestamps, rejects duplicate or out-of-order simulated timestamps,
checks runner counts and every supplied replay sample including post-finish
movement, and compares skill/competition event order within the exposed streams.
It also checks recorded durations, duplicate recipient observations, and extra
recipient events when the fixture contains a complete recipient list. Fixtures
with supplementary result fields, rushing state and blocker identities check
those too. Reports list missing observation coverage explicitly.

The historical pre-finish check remains available with `--legacy`. Before the
subsequent event-order corrections and fixture additions, all 67
fixtures pass it, while 63 pass the strict check. Four older fixtures still
have time-zero skill-event ordering differences; movement and competition
event sequences match their supplied observations. Three official Career
fixtures cover adjusted-stat rounding and approximately equal guts/wisdom
ranks. The separate 4,442-race Career audit has 4,031 strict matches, with
410 startup discrepancies and one later runtime discrepancy.
See [Career validation](CAREER-VALIDATION.md) for scope and test evidence.

Dense engine output contains additional ticks between official samples; this
checker does not claim identical recording cadence or compare hidden ticks.
Global ordering between separate event streams and unexposed presentation
result fields remain outside its scope. Some older fixtures contain only
selected recipient observations or omit duration/state/result observations.
`--fixtures DIRECTORY` selects another directory of captured fixture files.

Run the validator mutation tests and rank checks with:

```powershell
python -m unittest discover -s tests -p test_accuracy_regressions.py
python tools/check_career_team_rank.py --engine target/release/race_engine_rs_v2.exe --report target/career-ranks.json --native-inputs target/career-inputs.jsonl
```

The rank check compares 48 synthetic career races against equivalent pre-mood
stat inputs and checks invalid ranks and worker recovery. It covers 9- and
18-runner fields. The generated inputs also exercise ranked prepared-library
runs with learned skills and overcap wisdom using `check_native_parity.py`.
This is implementation consistency testing, not official career-race accuracy.
[Rank, worker and prepared-API results](../evidence/career-team-rank-validation-20261018.json).

The Rust suite also checks slope boundary inclusion, expiry without re-entry,
crossing a boundary without landing exactly on it, the inclusive 150 m competition
threshold, and HP and separation exits for duels after the finish. Parallel corpus timings
are validation-run timings; use the separate paired [performance measurements](PERFORMANCE.md)
for latency comparisons.

Focused unit tests additionally check queued-bonus caps, cooldown cleanup and
receiver expiry, front-block edges one float apart, and duration multiplier
rounding and clamps. Adjacent-float visibility checks exercise both sides of the
captured cone boundary. Post-finish movement checks cover signed step speed,
side permissions, target clamping and exclusion of normal lane modifiers.

## Binary protocol checks

```powershell
cargo test --release --locked --features fast-alloc
python -m unittest discover -s tests -p test_replay_protocol.py
python tools/check_replay_protocol.py --engine target/pgo/race_engine_rs_v2.exe --baseline path/to/reference-engine.exe --inputs path/to/races.jsonl --isolation
```

Rust checks retain float64 bits, signed zero, nonfinite values, sparse selection,
runner permutations, blocker indices and recovery after errors. Reader checks
cover partial reads, consecutive packets, malformed lengths and versions,
truncation and bounded reads. The conventional JSON number formatter is unchanged.

## Prepared library checks

Both generic and profile-guided libraries preserve 876 outcomes each across
repeated and alternate seeds, dense and sparse frames, changed skill tables,
runner permutations and expected errors. Separate checks interleave handles
with different definitions, retain old output views across later runs, reject
cross-thread handle use, and run 24 races concurrently with independent handles.
Failed preparation clears the old race; failed runs clear the old reply and
permit recovery. [Generic API checks](../evidence/prepared-generic-api.json),
[PGO API checks](../evidence/prepared-pgo-api.json).

The v73 prepared PGO library check uses 270 captured inputs and generated
variants, comparing every selected frame value and all metadata with the release
worker. Rust unit tests additionally compare prepared initialization with the original path across
signed seed boundaries, checking exact float bits and subsequent RNG draws.

```powershell
python tools/check_native_parity.py --library target/pgo/globalsim.dll --baseline path/to/reference-engine.exe --inputs path/to/races.jsonl --isolation
```

## Latest intake

The expanded [validation coverage audit](VALIDATION-AUDIT-20261022.md) adds
all-tick output integrity, official skill branch comparisons, correlated branch
and recipient checks, retained replay-surface inventories and explicit coverage
status. Its 464-race panel passes, alongside all 73 regression fixtures. A pass
means agreement on the checked observations; coverage gaps remain explicit.

The new 39,837-race practice batch has 39,834 observed matches after a natural
corner-expiry correction. All five new Career captures pass, bringing that
cohort to 4,031 strict matches out of 4,442. The previous 203,572 practice
captures were not rerun for this intake. The prepared library agrees with the
worker on 90 sequential runs and 24 concurrent runs, including the new corner
fixture and three Career fixtures.
[Findings and capture priorities](NEW-RACE-FINDINGS-20261018.md),
[validation evidence](../evidence/new-races-20261018-combined.json).
