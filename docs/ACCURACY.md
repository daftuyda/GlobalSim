# Benchmark results and coverage

## Version 76

The frozen corpus contains 470,350 official captures. The final optimized engine
produces byte-identical complete binary replies for every input compared with
the preceding audited engine. Each reconstructed input must also match its
frozen input hash. This preserves all strict observation outcomes, including
known discrepancies; it is not 470,350 official-race passes.

| Group | Strict matches | Total | Unresolved |
|---|---:|---:|---:|
| Practice, including quarantine | 457,989 | 457,989 | 0 |
| Career, including quarantine | 11,153 | 12,361 | 1,208 |
| Active captures, both types | 469,141 | 469,551 | 410 |
| Quarantined captures, both types | 1 | 799 | 798 |

The last two rows partition the first two. One practice capture remains
quarantined even though it now passes; quarantine was not changed for this
release. All 81 captured regression fixtures pass. The synthetic HP-empty
precondition test is separate and is not counted as an official capture.

See [v76 validation](V76-VALIDATION.md) and
[machine-readable results](../evidence/v76-release-validation.json).

## Historical Version 75

The latest batch contains 95,062 additional official races: 90,283 practice
and 4,779 Career races. Strict validation records **94,566 matches and 496
unresolved cases**, with no audit errors. The startup event-order correction
resolved 4,681 cases without regressions in that batch.

Of the unresolved cases, **492 are quarantined for captured-seed/replay
mismatches**. This leaves **94,566 strict matches out of 94,570 active races**
and four active discrepancies. Quarantine changes dataset selection, not the
historical result or the simulator's pass count. Replay-derived seed matches
are diagnostic evidence only.

The stored corpus contains 342,913 races at this snapshot; 342,421 remain active.
The older 247,851 races were not rerun for this batch. All 69 included strict
regression fixtures pass. See [batch validation](DISCREPANCIES-20261020.md),
[remaining cases](UNRESOLVED-FOLLOWUP-20261020.md), and
[quarantine policy](DATASET-QUARANTINE.md).

## Historical v74 benchmark

Version 74 was evaluated against **203,572 captured official
Global races**. **All 203,572 match every audited observation**, including finish
times, placements, sampled movement, recorded skill events and competition
events. This is observed 1:1 agreement with the tested captures at their recorded
precision and cadence, not a claim of equivalence across every possible race.

| Metric | Result |
|---|---:|
| Races matching all core checks | 203,572/203,572 |
| Races with every finish time exact | 203,572/203,572 |
| Winner agreement | 203,572/203,572 |
| Exact placement agreement | 2,210,585/2,210,585 |
| Races with every compared movement sample exact | 203,572/203,572 |
| Races with all skill starts exact | 203,572/203,572 |
| Matched skill starts | 23,395,637/23,395,637 |
| Worst observed finish-time error | 0 s |
| Unresolved execution errors | 0 |

Core checks comprise finish times, winner, placements, skill starts and the
position, speed, HP and lane samples defined below.
[Validation results](../evidence/new-races-20261017-morning.json).

## Captured-race coverage

| Field size | Races | Matches across all audited observations |
|---|---:|---:|
| 9 runners | 129,086 | 129,086 |
| 12 runners | 29,888 | 29,888 |
| 14 runners | 6,041 | 6,041 |
| 15 runners | 24,443 | 24,443 |
| 16 runners | 7,558 | 7,558 |
| 18 runners | 6,556 | 6,556 |

The datasets contain distinct official replay captures. Each race uses its
original seed, complete runner inputs and the skill definitions corresponding
to the capture.

Coverage includes 2,210,585 runner observations across 105 courses and
21 race distances from 1000 to 3600 m, on turf and dirt.

These captures informed the corrections and also serve as regression coverage.
They share some configurations and are not an independent, uniformly sampled
estimate of accuracy across the game. Custom oversized fields are outside
this benchmark.

## Additional observations

The audit compares **266,951,869 pre-finish runner/frame samples** for position,
speed, HP, lane, rushing state and front-blocker identity. All 2,210,585 start
delays, running styles, guts orders, wisdom orders and last-spurt start distances
match.

All 23,395,637 skill records have exact recorded durations and visual recipient
masks, with no missing or extra casts. Recorded competition-event identities
and times match in all 203,572 races under the earlier multiset comparison;
that check does not establish ordering among simultaneous events.

Recorded durations and visual recipients describe replay events; they do not
expose every gameplay receiver or its complete lifetime. Full-frame engine
output is retained for comparisons, while official observations are limited
to the capture's sampled frames. Post-finish movement is outside the core
pre-finish denominator.

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

The stricter regression audit includes post-finish samples and event-stream
order. All 67 fixtures pass the earlier checks; 63 pass the strict checks, with
four remaining time-zero skill-order differences. The 203,572-race result above
uses the earlier comparison definitions; it is not a strict replay-order pass.

A separate cohort of 4,442 official Career races has 4,031 strict observation
matches. The remaining cases comprise 410 startup discrepancies and one later
runtime discrepancy. All captured team ranks are zero, so nonzero rank bonuses
still require official validation. See [Career validation](CAREER-VALIDATION.md)
and [strict validation](TESTING.md).

## Latest practice intake

An additional **39,837 official practice races** have been audited: **39,834**
match the exposed observations under the practice definitions above. One
corner-boundary skill discrepancy was corrected; one pacing discrepancy and
two small position differences remain. The previous 203,572-race benchmark
remains a separate result on its recorded engine; it has not been rerun for
this intake. See [new race findings](NEW-RACE-FINDINGS-20261018.md).
