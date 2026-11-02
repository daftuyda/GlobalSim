# Validation results and findings

Reviewed on **3 November 2026** for **v77**. This summary reconciles completed
audits and saved research results; it is not a new simulation run. The latest
release checks and full practice audit were completed on 1 November.

## Official-priority default

| Check | Recorded result | Scope |
|---|---:|---|
| Rust unit tests | 167/167 pass | Fresh v77 default release checks |
| Python validation tests | 39/39 pass | Includes 12 promotion-gate tests |
| Official regression fixtures | 84/84 pass | Strict recorded observations, including post-finish checks |
| Active official practice captures | 696,777/696,778 match | Full audit of the frozen 28 October inventory |
| Quarantined practice capture | 1/1 match | Remains quarantined; excluded from the active score |

The active practice result is **99.99986% observed agreement**, with one remaining
case. Every race in that audit was simulated with full frames. Earlier strict
passes were reused only for identical captures, projected inputs and complete
outputs; changed or previously failing results received fresh comparisons.
This does not mean every old comparison was recomputed or every official tick
was available to observe. The v77 release retained the audited simulation code.

The 84-fixture suite contains 79 practice and five Career cases across 47 course
IDs. It contains 538 equipped skill IDs, of which 464 are observed activating;
320 of 573 equipped branch versions are observed, with branch observations in
11 races. These are coverage counts, not an accuracy percentage for all skills.

Earlier worker/prepared-library checks passed 42 equivalent runs in each of two
frame modes, plus 24 concurrent runs per profile, for both the default and full
experimental profiles. Handle isolation and recovery also passed. These are
implementation-equivalence checks, separate from official accuracy.

Sources: [v77 checks](V77-VALIDATION.md), [fixture coverage](../evidence/v77-coverage.json),
[full official audit](../evidence/official-practice-validation-20261101.json),
[worker/library checks](../evidence/experimental-pacing-log-20261101.json).

## Third-party comparisons

The most recent qualified secondary audit excludes two cases with unresolved
checker errors. Follow-up work completed, but those cases remain inconclusive
in the qualified count; this was not an uninterrupted clean audit.

| Profile | Qualified sampled exports | Targeted probes |
|---|---:|---:|
| Official-priority default | 354,484/399,998 | 3/40 |
| Whole-metre cutoff plus pacing logarithm | 354,487/399,998 | 13/40 |

The two-feature candidate failed the official promotion gate: it matched
696,775/696,778 active practice captures, with two new failures and no fixes.
One new failure is a reproducible cutoff regression; the other is a retained
event-clock anomaly that did not repeat. It remains disabled by default.

The separate full experimental profile matched **399,947/400,000 sampled exports
(99.98675%)**, with **53 discrepancies**, and 40/40 targeted probes. It passed
only **72/84 official regression fixtures**, so it is not the default engine.
Those 400,000 sampled results come from its own audit; they must not be combined
with the qualified secondary panel into a single score.

Its detailed 85-capture panel recorded:

| Observation | Matches |
|---|---:|
| Ordinary exported reports | 73/85 |
| Every recorded binary replay frame | 73/85 |
| Pacing reference and mode through 15 seconds | 68/85 |
| Full-race recorded pacing decisions | 73/85 |

The pacing-log option corrected 551 threshold values in eleven captures and
raised decision agreement from 62/85 to 73/85 without changing any of those
85 complete outputs. Seventeen controlled wisdom captures, including ten
predictions made before collection, support the numerical observation.
Recorded remote frames still leave unobserved ticks between samples.

Sources: [qualified secondary audit](OFFICIAL-PRIORITY-20261101.md),
[experimental profiles](EXPERIMENTAL.md),
[numerical evidence](../evidence/experimental-pacing-log-20261101.json).

## Latest restoration contrasts

The latest study tested three proposed ways to select the pacing reference when
the forward group retakes the lead. Its focused panel deliberately includes
known failures and sensitive matching controls.

| Selection experiment | Before | After | Fixed | Regressed |
|---|---:|---:|---:|---:|
| Incumbent loses rank; choose previous-order leader | 1,327/1,402 | 1,315/1,402 | 18 | 30 |
| New forward runner passes incumbent; choose previous-order leader | 1,327/1,402 | 1,315/1,402 | 18 | 30 |
| Same passing trigger; choose current-order leader | 1,327/1,402 | 1,314/1,402 | 17 | 30 |

All three were rejected. Seven further report/replay pairs include five repeated
cases and two new one-point power variants. All five repeated observations agree
with their earlier captures. The retained experimental engine matches 3/7
reports; each proposal matches 2/7 and breaks all three matching controls.
The +1-power variant still fails, ruling out rank loss or a newly passing runner
as a sufficient explanation.

The four retained-profile failures first disagree on the pacing reference at
7.792 or 8.125 seconds, before their first recorded position difference at
8.525 seconds. This localizes the observed divergence; it does not establish
the remote simulator's hidden implementation. No engine change was retained.
These selected captures do not increase the 400,000-sample denominator.

The [restoration evidence](../evidence/experimental-restoration-20261101.json)
records case-level differences, the frozen predictions and artifact receipts.

## Remaining findings and limits

| Finding | Current status |
|---|---|
| Official Tokyo dirt 1,600 m, seed 580983021, 16 runners | Active unresolved case: 85 position differences, maximum 1.221 mm; two HP and two spurt-distance differences. Raw finishes and placements match. |
| Official duel-stamina boundary | Default matches; float32 percentage arithmetic keeps a duel active one tick too long. |
| Official Fukushima 1,150 m entry boundary | Default fractional cutoff matches; whole-metre rounding removes valid competition entry. |
| Career initialization | 410 historical active discrepancies; no fresh full v77 Career audit. Nonzero team-rank bonuses still need official capture validation. |
| Experimental complete-output repeatability | One retained variation among 400,040 outputs did not repeat; whole-audit byte equivalence is not claimed. |
| Validation/process incidents | Checker disagreement and Python process failures remain unresolved. Successful reruns do not erase the original evidence. |

The active official inventory contains 696,778 practice and 12,289 Career
captures. This is the 28 October inventory, not a live collection count.
Startup mismatch hypotheses do not justify changing an official seed.

The isolated pacing-log and alternative duel-arithmetic corpus experiments also
remain incomplete; passing their partial checks does not qualify either for
promotion. Earlier rejected counter-reset, restoration-order and eligibility
experiments retain their original results in the [experimental history](EXPERIMENTAL.md).

Official captures, third-party reports, internal decision traces, targeted
controls, unit tests and output-equivalence checks have different and often
overlapping scopes. **There is no defensible combined accuracy percentage for
all of them.** See [accuracy and coverage](ACCURACY.md) for the comparison rules
and [Career validation](CAREER-VALIDATION.md) for collection priorities.

The [consolidated snapshot](../evidence/accuracy-snapshot-20261103.json) binds this
summary to the recorded source reports. Historical reports and performance
measurements retain their original dates, engine profiles and denominators.
