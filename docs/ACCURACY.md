# Accuracy and coverage

This page describes the v76 engine with the pacemaker and passing-route
corrections committed on 31 October 2026. Official replay comparisons,
implementation regressions and third-party simulator comparisons have separate
denominators. None establishes accuracy for every possible race.

## Evidence priority

Official practice captures are the primary reference for race mechanics and
numerical behavior. Uma.moe exports are supporting evidence for generating
hypotheses and reproducing possible edge cases. When the sources disagree, retain
the behavior supported by official captures. A higher third-party match count
cannot compensate for an official regression.

Changes to race behavior must preserve the matches in the complete frozen active
practice inventory before promotion to the default engine. Keep the original
seeds, runner builds, course data, validator and comparison coverage fixed.
Missing races, changed inputs or weaker checks cannot establish a pass. An
unresolved official case whose output changes remains subject to investigation,
even if both builds still classify it as a failure. A simulator disagreement
alone is not grounds for quarantining an official capture.

The default build remains the basis of official-accuracy work. Experimental
features that fail official fixtures are blocked from promotion. Features that
pass the fixtures still need the full practice comparison; passing 84 cases does
not establish accuracy on the roughly 700,000-capture inventory.

### Official regression gate

`tools/check_official_promotion.py` compares strict fixture reports and frozen
corpus audits. It requires every active practice capture in the supplied
authoritative inventory, unchanged inputs and validation coverage, and zero new
official failures. It also binds each corpus audit to the same engine used for
its fixture check. It does not accept third-party scores as inputs.
Fixture reports also identify the exact compressed capture files; matching case
names alone cannot establish that both engines were checked against the same data.

```sh
python tools/check_official_promotion.py --baseline-fixtures official-audits/baseline/fixtures.json --candidate-fixtures official-audits/candidate/fixtures.json --inventory official-audits/frozen/jobs.jsonl --baseline-practice official-audits/baseline --candidate-practice official-audits/candidate --report official-audits/promotion.json
```

Each corpus audit directory supplies `config.json`, `jobs.jsonl`, `results.jsonl`
and `summary.json`, with receipts for its engine, course data, projections and
validation tools. Use the authoritative inventory frozen from the official
registry, including the archived cohort and existing quarantine membership.
The gate covers exactly that inventory; an independently selected subset must
not be described as a whole-corpus audit. Career and quarantined results are
reported separately and cannot offset active-practice regressions.

Omitting the corpus arguments provides a fixture screen but always leaves the
promotion gate closed. A closed gate exits with status 1 and records its reasons.
Passing this check establishes non-regression on its recorded scope, not universal
accuracy, and does not itself publish or deploy a build.

## Current validated engine

| Check | Scope | Result |
|---|---|---:|
| Rust unit tests | Default build | 167/167 pass |
| Official regression fixtures | Included regression cases; unresolved capture reported separately | 84/84 pass |
| Full active official practice inventory | Frozen 28 October inventory, checked 1 November | 696,777/696,778 match recorded observations |
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

The current whole-practice result is **99.99986% observed agreement** on this
collected inventory. Every current race was executed with full simulation frames.
Previously verified strict passes were reused only when the capture, projected
input and complete output were identical; new or differing outputs received a
fresh strict comparison. This is not a claim that every old comparison was
recomputed, or that unrecorded official frames were observed.

See the [official-practice validation](OFFICIAL-PRIORITY-20261101.md),
[current evidence](../evidence/official-practice-validation-20261101.json),
[earlier validation summary](../evidence/documentation-snapshot-20261031.json),
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

The 1 November audit executed all **696,779 practice captures**: 696,777 of
696,778 active captures matched, and the separately quarantined capture matched
but remains quarantined. The active movement discrepancy has matching raw finish
times and placements, a maximum sampled position difference of 1.221 mm, two HP
differences and two spurt-distance differences. Its input and observations are
included under `tests/fixtures/unresolved` and still fail strict validation.

Career races were not rerun in this audit. Their **410 active discrepancies**
remain a historical result, not a newly verified count.
See the [intake report](NEW-RACES-20261028.md),
[Career coverage](CAREER-VALIDATION.md) and [quarantine policy](DATASET-QUARANTINE.md).

## Third-party simulator comparison

The latest narrowed experiment combines only the whole-metre entry cutoff and
pacing logarithm. It improves three sampled uma.moe comparisons and ten targeted
probes, but the complete official audit records two new failures and no fixes.
It remains disabled by default. Its secondary audit required a continuation after
validation-process errors; two affected cases remain excluded from qualified
counts. See the [complete scoped assessment](OFFICIAL-PRIORITY-20261101.md).
The historical measurements below retain their original denominators and profiles.

The two latest uma.moe collections contain 400,000 sampled races plus 40 targeted
boundary probes. The earlier 240,000-race collection has no equipped skills;
the newer 160,000-race collection includes 32,000 races with equipped skills.

| Collection | Default build | Gap + duel profile | All follow-up features |
|---|---:|---:|---:|
| Earlier sampled collection | 213,745/240,000 | 239,967/240,000 | 239,981/240,000 |
| New sampled collection | 140,741/160,000 | 159,938/160,000 | 159,966/160,000 |
| Targeted boundary probes | 3/40 | 3/40 | 40/40 |
| Wave 4 exact-seed checks | 13/33 | 19/33 | 33/33 |

These counts measure agreement with checked third-party exports, not
official-game accuracy. The targeted probes intentionally concentrate known
failures and are reported separately from the sampled panels. Four repeat
controls are included in the 40 probes.

The opt-in `experimental-umamoe` build has higher third-party agreement but
regresses official captures. The default build retains the official-tested
rules. The original aggregate preserves the official lead-competition partner
cutoff. Three additional opt-in features improve third-party agreement further,
including a participant-timing rule that conflicts with an official capture.
The separate `experimental-pacing-log` option also reproduces captured numerical
lottery thresholds without changing the default arithmetic.
The full follow-up combination matches 399,947/400,000 sampled third-party races
but passes only 72/84 current official regression fixtures. Ordinary report matches do not establish
full internal-state equivalence. See the
[experimental results and limits](EXPERIMENTAL.md) and the
[earlier comparison](UMAMOE-MOVEMENT-20261031.md).

The Wave 5 rushing-reference correction improves two observed internal reference
histories while preserving all 400,040 complete outputs in the frozen third-party
audit. The sampled match count above is unchanged, with 53 sampled discrepancies
remaining. Eight fresh exact-seed captures repeat existing failures and are not
added to the sampled denominator. A broader counter-reset trial fixed eight
sampled cases but broke 51 others and was rejected. See the
[Wave 5 validation record](../evidence/experimental-wave5-20261101.json).

An additional 21 targeted capture pairs match 18 ordinary reports. The expanded
detailed audit covers 67 captures: 55 match every recorded binary frame as well
as their reports, 50 match reference-and-mode histories through 15 seconds, and
54 match full-race pacing decisions. These scopes overlap but are not equivalent;
recorded binary frames still omit intervening remote ticks. No additional
report-matching movement failures were found. The new counter variants regress
controls and remain excluded. See the
[control and frame audit](../evidence/experimental-wave5-controls-20261101.json).

Seventeen controlled wisdom captures support the optional pacing-log calculation,
including ten predictions recorded before collecting the new replays. Across the
expanded 85-capture panel, it corrects 551 threshold values in eleven captures:
complete pacing-decision agreement rises from **62/85 to 73/85**. Ordinary reports
and recorded binary frames remain **73/85**; reference-and-mode histories through
15 seconds remain **68/85**. All 85 fresh complete outputs are unchanged.

The 400,040-input regression retains every report pass/fail result, with **53
sampled discrepancies** remaining. It preserves 400,039 complete outputs. One
isolated output changes a lane sample and a spurt-start annotation; it does not
repeat when either build replays the preceding 34,003-race sequence. A new remote
control supports the normally reproduced spurt timing. The anomalous output is
retained for investigation, so complete-output equivalence across the entire
audit is not claimed. Default and combined experimental builds pass 167 Rust
tests each and retain their respective **82/82** and **72/82** official-fixture
results. See the [numerical validation record](../evidence/experimental-pacing-log-20261101.json).

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
