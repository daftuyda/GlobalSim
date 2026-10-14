# Benchmark results and coverage

GlobalSim v73 has been evaluated against **47,664 captured official Global
races**. All **47,664 match every audited observation**, including finish times,
placements, sampled movement, recorded skill events and competition events.
These figures describe the tested captures at their recorded precision, not
accuracy across every possible race configuration.

| Metric | Result |
|---|---:|
| Races matching all core checks | 47,664/47,664 (100% observed) |
| Races with every finish time exact | 47,664/47,664 |
| Winner agreement | 47,664/47,664 |
| Exact placement agreement | 435,508/435,508 |
| Races with every compared movement sample exact | 47,664/47,664 |
| Races with all skill starts exact | 47,664/47,664 |
| Matched skill starts | 4,497,700/4,497,700 |
| Worst observed finish-time error | 0 ms |
| Execution errors | 0 |

Core checks comprise finish times, winner, placements, skill starts and the
position, speed, HP and lane samples defined below.
[Validation results](../evidence/parity-validation-20261014.json).

## Captured-race coverage

| Dataset | Races | Core matches | Matches including details and competition events |
|---|---:|---:|---:|
| Initial benchmark | 13,187 | 13,187 | 13,187 |
| Course 10602, 1600 m collection | 14,775 | 14,775 | 14,775 |
| First additional course set | 2,819 | 2,819 | 2,819 |
| Latest additional course set | 16,883 | 16,883 | 16,883 |

The datasets contain distinct official replay captures. Each race uses its
original seed, complete runner inputs and the skill definitions corresponding
to the capture.

The initial benchmark contains 125,215 runner observations: 12,405 races have
nine runners, and 782 have 12, 14, 16 or 18 runners. The other 34,477 races have
nine runners on 16 turf courses at 1600 or 1800 m, spanning 77 distinct rosters
and 341 learned skill IDs. The latest 16,883-race set covers all 16 courses,
13 rosters and 295 learned skill IDs. Four newly available skill definitions
were present in its data but unused by its rosters.

These captures informed the corrections and also serve as regression coverage.
They share some configurations and are not an independent, uniformly sampled
estimate of accuracy across the game. Custom oversized fields are outside
this benchmark.

## Additional observations

No mismatch remains in the audited observations on this dataset. The full set
provides **51,649,161 pre-finish runner/frame samples**, matching position, speed,
HP, lane, rushing state and front-blocker identity. All 435,508 start delays,
running styles, guts orders, wisdom orders and last-spurt start distances match.

All **4,497,700 skill records** match their checked starts, detail choices,
recorded durations and visual recipient masks. All **351,046 competition
events** match. Recorded durations and visual recipients describe replay events;
they do not expose every gameplay receiver or its complete lifetime.

The optimized build reproduces all 47,664 audited output packets, including
every generated frame and all metadata. This verifies agreement between builds
beyond the official replay's sampled observations.

Post-finish checks cover runners whose movement changed in the latest
corrections. All 37 compared samples match, with no regressions. These samples
are outside the core pre-finish denominator.
[Post-finish checks](../evidence/parity-postfinish-validation.json).

## What exact means

- `finishExact`: the entire vector of decoded official raw finish times equals
  the simulated `finishTimeRaw` vector, without a tolerance.
- Winner: simulated finish-time ordering selects the official winner.
- Placement: corresponding identities in sorted official and simulated orders
  match; denominator is runners.
- Skill starts: multiset of runner, skill ID and timestamp rounded to four
  decimal places (0.1 ms). `skillExact` compares the full multiset; matched-start
  recall alone does not measure recipient, value or duration correctness.
- Frame exactness: official replay sample times matched to simulation times
  rounded to five decimals; positions compared as float32; speed, HP and lane
  compared through the audit's wide wire quantization (scales 100,1,10000).
  Samples after either official or simulated finish are excluded. It is not
  proof of all hidden state or all unsampled ticks matching.
- Competition events: recorded runner, event kind and float32 timestamp match
  for Spot Struggle, dueling and Conserve Power starts. These are checked in
  addition to the core metrics.
- Recorded skill durations: float32 duration multiplied by 10000 in wider
  arithmetic and truncated to integer wire units; time-zero events use `-1`.
  No one-unit tolerance is applied.
- Visual recipients: the recorded mask matches the first ability's selected
  targets, with the replay's time-zero sentinel. Secondary gameplay targets
  are not all visible in this field.

## Limitations

Conserve Power fallbacks and optional later stamina mechanics include
approximations. Results also depend on the selected runtime settings and course
and skill data. Coverage is not uniform across course, roster or skill
combinations, and the results do not establish population-wide accuracy.

GlobalSim is an unofficial implementation and does not claim complete 1:1
equivalence with the game. See [testing](TESTING.md) for the current test status.
