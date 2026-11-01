# Accuracy and coverage

This page describes the v76 engine with the pacemaker and passing-route
corrections committed on 31 October 2026. Official replay comparisons,
implementation regressions and third-party simulator comparisons have separate
denominators. None establishes accuracy for every possible race.

## Current validated engine

| Check | Scope | Result |
|---|---|---:|
| Rust unit tests | Default build | 165/165 pass |
| Official regression fixtures | Included captured edge cases | 82/82 pass |
| Official practice regression | 52,096-race cohort audited on 28 October | All complete outputs preserved; all checked official observations match |
| Third-party output regression | 437,000 uma.moe inputs | 436,999 complete outputs unchanged; one discrepancy corrected; no regressions |

An unchanged output preserves its previous result, including any existing
discrepancy. The 437,000-race regression is not a claim that every race matches
uma.moe. The final production build also agrees with the validated candidate
on 62 complete-output checks.

The pacemaker correction resolved a section-boundary discrepancy in the
66,738-race practice intake. That intake then had 66,737 strict matches and
one remaining movement discrepancy. The passing corrections retain that
behavior and refine equal-cost route selection.

See the [validation summary](../evidence/documentation-snapshot-20261031.json),
[pacemaker evidence](../evidence/practice-fix-20261024.json),
[passing-clearance evidence](../evidence/passing-clearance-20261031.json) and
[passing-direction evidence](../evidence/passing-route-direction-20261031.json).

## Official capture inventory

The latest documented inventory snapshot is **28 October 2026**, not a live
count of ongoing collection:

| Capture type | Registered | Active | Quarantined |
|---|---:|---:|---:|
| Practice | 696,779 | 696,778 | 1 |
| Career | 13,174 | 12,289 | 885 |
| Total | 709,953 | 709,067 | 886 |

Practice includes 13,187 archived races outside the newer collection inventory.
The 52,096-race intake covers every field size from 9 through 18 runners. It
checks 97,425,308 runner observations at recorded official frames, plus results,
skills and competition events. Integrity checks also cover full simulator output.

The entire 709,953-race inventory has not been rerun on the current source
revision. Historical audits retain **one active practice discrepancy and 410
active Career discrepancies**. The practice case has matching raw finish times
and placements but a maximum sampled position difference of 1.221 mm and some
stamina/spurt differences. These remain recorded issues, not newly audited totals.
See the [intake report](NEW-RACES-20261028.md),
[Career coverage](CAREER-VALIDATION.md) and [quarantine policy](DATASET-QUARANTINE.md).

## Third-party simulator comparison

The two latest uma.moe collections contain 400,000 sampled races plus 40 targeted
boundary probes. The earlier 240,000-race collection has no equipped skills;
the newer 160,000-race collection includes 32,000 races with equipped skills.

| Collection | Default build | Optional experimental profile |
|---|---:|---:|
| Earlier sampled collection | 213,745/240,000 | 239,967/240,000 |
| New sampled collection | 140,741/160,000 | 159,938/160,000 |
| Targeted boundary probes | 3/40 | 3/40 |

These counts measure agreement with checked third-party exports, not
official-game accuracy. The targeted probes intentionally concentrate known
failures and are reported separately from the sampled panels. Four repeat
controls are included in the 40 probes.

The opt-in `experimental-umamoe` build has higher third-party agreement but
regresses official captures. The default build retains the official-tested
rules. Both profiles preserve the official lead-competition partner cutoff,
which conflicts with some third-party outputs. See the
[experimental results and limits](EXPERIMENTAL.md) and the
[earlier comparison](UMAMOE-MOVEMENT-20261031.md).

## What the checks establish

- **Inputs:** retain the captured seed, runner identities, ordering, stats,
  skills, course and runtime settings. Validate capture linkage and input
  integrity before interpreting a simulation difference.
- **Results:** compare raw finish times, placements and exposed result fields,
  including start delays, running styles, stat orders and spurt distances.
- **Movement:** compare position, speed, HP, lane, rushing state and front-blocker
  identity at recorded official samples, including post-finish samples in the
  strict checker. Position uses float32 equality; encoded channels use the
  replay's wire precision. Unrecorded official ticks cannot be compared.
- **Skills:** compare exposed identities, detail branches, timing, order,
  durations and visual recipient masks. The first ability's visual mask does
  not expose every secondary gameplay recipient or effect value.
- **Competition:** compare exposed event identities, times and sequence order.
  Ordering within a recorded stream does not establish a global order across
  separate streams.
- **Output integrity:** check finite values, identities, structure and temporal
  consistency throughout the simulator's full output. These checks can detect
  invalid output without observing hidden official state.
- **Repeatability:** compare repeated runs, seed order, persistent workers and
  prepared handles. Complete-output equality is an implementation regression
  check, separate from agreement with an official capture.

Earlier audits used different definitions, including pre-finish-only samples
and unordered event comparisons. Their results keep their original scope;
they are not retroactively strict full-replay passes.

## Limitations

Coverage is concentrated in collected configurations and includes cases used to
develop corrections. It is not an independent random sample of all races.
Nonzero Career team-rank bonuses have synthetic coverage but still need official
captures. Career seed/replay discrepancies remain unresolved. Optional later
stamina mechanics and some Conserve Power fallback paths contain approximations.
Custom oversized fields are outside official capture validation.

Results depend on the input data and runtime profile. Use the v76 example's
settings for the documented validated path. [Testing](TESTING.md) describes
reproducible checks; the [documentation index](README.md) retains dated reports
for earlier cohorts and releases.
