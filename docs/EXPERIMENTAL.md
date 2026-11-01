# Experimental comparison profiles

GlobalSim provides opt-in rules for investigating differences with uma.moe's
exported races. These profiles retain the same seeded worker and prepared-library
interfaces. They are research configurations of v76, not a new stable release.

## Build

The original aggregate enables the gap and duel experiments:

```sh
cargo test --release --locked --features experimental-umamoe --target-dir target/experimental-umamoe
cargo build --release --locked --features experimental-umamoe --target-dir target/experimental-umamoe
```

To enable the four additional follow-up experiments as well:

```sh
cargo test --release --locked --features "experimental-umamoe,experimental-pacemaker-staging,experimental-spot-cutoff,experimental-spot-entry-floor,experimental-pacing-log" --target-dir target/experimental-followup
cargo build --release --locked --features "experimental-umamoe,experimental-pacemaker-staging,experimental-spot-cutoff,experimental-spot-entry-floor,experimental-pacing-log" --target-dir target/experimental-followup
```

Use the worker or library from that profile's release directory. The feature
applies to both interfaces and every seed. `experimental-umamoe` enables
`experimental-duel-hp` and `experimental-gap-refresh`; either component can also
be selected independently. The four follow-up features are separate opt-ins;
they are not added to the existing aggregate. Each feature can also be selected
independently. A build without these features uses the default rules.
Build the release artifacts after running tests; test builds can replace the
library with a different panic configuration.

## Mechanics

**Inward-gap refresh.** During the opening phase, an inward movement permission
changing from blocked to open can request one ordinary lane search. This applies
while the runner has neutral pacing, no passing targets, has reached its current
lane target, and cannot move outward. The search bypasses passing selection for
that update and preserves the passing cooldown for subsequent updates. This rule
has substantial known differences from official captures.

**Duel-stamina percentage.** For the retail HP model, the five-percent duel-exit
test calculates `(remaining_hp * 100) / maximum_hp` in single precision before
comparing with `5`. Multiplying before division can round a boundary value to
exactly five percent. That can preserve a duel which the default ratio comparison
would end. Other HP policies and non-retail duel behavior keep their existing test.

**Staged pacemaker restoration** (`experimental-pacemaker-staging`). At a section
boundary after a handover, the returning forward-style group can use its new
reference one tick before other styles. That selection is retained for the next
restoration when the runner remains eligible. The reference lookup uses the
runner's temporary running style while rushing, consistent with its pacing
controller. Ordinary restoration keeps its existing selection rule.

**Participant cutoff timing** (`experimental-spot-cutoff`). Invited
lead-competition partners use their previous-tick positions for the entry cutoff.
Other eligibility checks retain their live timing. This rule conflicts with an
official capture: the official race admits two participants, while the projected
uma.moe race and this experiment admit three. It is not an official-game fix.

**Whole-metre entry cutoff** (`experimental-spot-entry-floor`). The retail
lead-competition entry distance is rounded down to a whole metre. On the 1,150 m
course, controlled uma.moe captures admit a runner at 286.9945 m but reject one at
287.0020 m, supporting 287 m rather than 287.5 m. Those observations do not establish
the official game's rule. Both cutoff experiments preserve non-retail behavior.

**Pacing logarithm** (`experimental-pacing-log`). The wisdom lottery uses a split
single-precision logarithm calculation, reproducing the thresholds in seventeen
controlled third-party captures. The reduction follows the structure of the
[older SunPro/glibc calculation](https://github.com/bminor/glibc/blob/glibc-2.39/sysdeps/ieee754/flt-32/e_log10f.c),
with double-to-single rounding for the inner natural logarithm. This identifies a
compatible calculation, not the remote engine's platform or library. The default
retains its double-precision base-ten logarithm rounded to single precision.

## Recheck official fixtures

```sh
python tools/check_accuracy_regressions.py --engine target/experimental-umamoe/release/race_engine_rs_v2.exe --report target/experimental-umamoe/official.json
```

Use the appropriate release directory for the profile being checked; omit `.exe`
on Linux or macOS. Experimental combinations fail some official fixtures. The validator
keeps its normal strict comparison and returns failure for those disagreements.

## Original profile results

| Check | Default | Duel arithmetic only | Combined experimental |
|---|---:|---:|---:|
| Rust unit tests | 165/165 | 165/165 | 165/165 |
| Official captured-race fixtures | 82/82 | 82/82 | 73/82 |
| Official practice regression cohort | 52,096/52,096 | 52,096/52,096 | 45,026/52,096 |
| New sampled uma.moe collection | 140,741/160,000 | 140,741/160,000 | 159,938/160,000 |
| Targeted uma.moe boundary probes | 3/40 | 3/40 | 3/40 |

The combined profile matches **99.96125%** of the newer sampled collection. Across
the two latest sampled collections it matches **399,905/400,000 (99.97625%)**.
Those percentages describe third-party observations. The combined profile has
**7,070 official practice discrepancies** in the cohort above and is unsuitable
as a replacement for the default official-tested rules.

The duel-only profile preserves every complete output in the 52,096-race official
cohort and the 160,040 new third-party checks. Its boundary unit test establishes
the numerical difference between the formulas; unchanged captured outputs do
not establish which formula the official game uses at that boundary. It remains
experimental, with no new corpus matches demonstrated in these checks.

The combined build reproduces the prior research configuration's complete outputs
on all 400,040 third-party inputs checked. This guards the implementation of the
packaged profile; previously mismatching reference races remain mismatches.
Prepared-library comparisons, independent handles, concurrent runs and recovery
checks pass for both packaged profiles. The gap-only component also passes all
165 unit tests; its standalone corpus accuracy has not been measured here.

A separate immediate pacemaker-handover trial fixed 19 cases but regressed 51
across 400,040 races. It is excluded. The staged-restoration feature above uses
a narrower rule and preserves the older matching controls.

See the [validation record](../evidence/experimental-profiles-20261101.json) for
the checked profiles, fixture failures and audit scope.

## Follow-up results

The packaged build with all five features was checked against the same frozen
400,040 inputs. It fixes **79 previously failing checks with zero regressions**
relative to the original gap-and-duel profile.

| Check | Gap + duel | All follow-up features |
|---|---:|---:|
| Earlier sampled collection | 239,967/240,000 | 239,981/240,000 |
| Newer sampled collection | 159,938/160,000 | 159,966/160,000 |
| Existing targeted boundary probes | 3/40 | 40/40 |
| New Wave 4 exact-seed checks | 19/33 | 33/33 |
| Older adversarial controls | 34/47 | 36/47 |
| Official captured-race fixtures | 73/82 | 72/82 |

The sampled total is **399,947/400,000 (99.98675%)**, leaving 53 sampled
discrepancies. Targeted checks and older controls are separate panels and are not
added to that percentage. Eleven older adversarial checks also remain mismatches.
The 33 new checks have both reports and detailed replay captures; the 33/33 result
above describes the ordinary report comparisons, not every internal state.

The default build still passes all 82 official fixtures and preserves every
complete output in the 52,096-race official practice cohort. The full follow-up
combination has not been audited on that larger official cohort; its 72/82 fixture
result already rules out replacing the default. The original gap-and-duel
cohort result above must not be attributed to the new combination.

Both final default and follow-up builds pass 165 Rust tests, with 27 Python tests
passing. Prepared-library, worker, concurrent-handle and recovery checks pass.
The packaged follow-up build matches the prototype's complete outputs on all
1,066 focused checks, including the new captures and older adversarial controls.

Detailed staging checks improve reference-and-mode histories through 15 seconds
from 16/32 to 26/32 and full-race pacing-decision comparisons from 20/32 to 27/32.
Two observed one-tick reference differences for rushing runners are corrected
by the Wave 5 change below.
Those checks cover the staging experiment and must not be read as complete
state validation of all five features.

One earlier participant-timing prototype audit recorded an unexpected replay
digest for a report-matching race. Its original full output was not retained.
The difference did not recur across 32 processes and 6,766 runs, including two
replays of the preceding input sequence. It remains unexplained. The final full
audit validates changed outputs directly and retains them; it does not claim
complete byte equivalence to that earlier prototype audit.

See the [follow-up validation record](../evidence/experimental-followup-20261101.json)
for measured scopes, remaining official failures and the retained anomaly.

## Wave 5 validation

Eight new report/replay pairs reproduce eight previously failing sampled races.
All eight repeat their earlier exported observations on the newer upstream
engine. They are repeat checks, not additional independent sampled races.

The retained correction makes staged reference selection use the pacing
controller's temporary running style while rushing. It fixes the observed
one-tick reference differences in two Wave 4 controls. Across 46 detailed checks,
reference-and-mode agreement through 15 seconds improves from **35/46 to 37/46**,
with complete trace coverage in that window. Full-race pacing-decision agreement
remains **37/46**. The research observer includes attempts rejected before a
lottery and verifies that logging leaves the complete race output unchanged.

The corrected implementation preserves **all 400,040 complete race outputs** in
the frozen third-party audit. Its packaged build also matches the checked outputs
on 1,074 focused inputs. Ordinary report accuracy therefore remains
**399,947/400,000 sampled races**, with **53 sampled discrepancies**, and the eight
Wave 5 repeats remain unresolved. The 40 targeted boundary probes and 33 Wave 4
checks still match; the older adversarial panel remains 36/47.

A separate section-counter reset trial initially fixed 11 checks in the focused
screen, including three repeated Wave 5 cases, without a regression there. The
full audit instead found **8 fixes and 51 regressions** among 400,040 inputs. It
also broke the official `pacemaker-restoration-after-selection-window` fixture.
The reset was rejected and is absent from this build. Its apparent fixes do not
count toward the retained profile's accuracy. An always-leading-returner trial
also broke matching controls and was rejected.

The retained experimental profile remains **72/82** on official fixtures; the
default remains **82/82**. Both profiles pass 165 Rust tests, and 27 Python checks
pass. Prepared-library/worker equivalence, concurrent handles and recovery checks
also pass. These results do not promote the experimental rules to the default.

Longer trace windows still contain missing coverage after idle pacing controllers
stop updating. One counter-reset trial also matched a report while retaining a
different internal reference history. Report agreement and complete internal
state agreement remain separate checks.

See the [Wave 5 validation record](../evidence/experimental-wave5-20261101.json)
for the retained correction, rejected trials and remaining capture questions.

### Return controls and recorded-frame checks

A further 21 report/replay pairs cover eight repeats, twelve synthetic power
variants and one projection of an official fixture. The thirteen return and
counter controls match **11/13** ordinary reports; the eight boundary-sweep
variants match **7/8**. These targeted checks do not increase the independent
sample count. Repeated inputs preserve the earlier exported observations.

The detailed audit now also compares position, speed, HP, lane, rushing state
and blocker identity at the binary replay's own recorded timestamps. It uses
the encoded channels' wire precision, maps runner identities and refuses to
interpolate missing ticks or omit post-finish samples. Across 67 targeted
captures, **55/67** match both the ordinary reports and every recorded binary
frame. There are 4,629 recorded frames; four clocks absent from local output
belong to already failing cases. This adds coverage without exposing the
remote engine's intervening ticks.

Reference-and-mode histories through 15 seconds match **50/67**, with complete
coverage in that window. Seven report-matching captures, representing five
distinct inputs, still have reference-annotation differences. They do not
demonstrate a movement error by themselves. Full-race pacing decisions match
**54/67**. One report-matching official projection differs only in ten reported
lottery-threshold values; its rolls and decisions agree. The subsequent wisdom
checks below reproduce those values with the optional split-logarithm calculation.

The official projection agrees with all nine captured elapsed finish times and
80 skill-activation identities/times. Other official result fields are absent
from that comparison, and full official frame/state parity is not established.

Further counter experiments were rejected. Using the fresh rank for the section
counter fixes **1** input but breaks **80** in the 400,040-input audit. Three
stricter guards each fix **2** checks but break **74** in an expanded 1,220-input
screen. None is included in the engine. The retained sampled result remains
**399,947/400,000**, with **53** discrepancies; no simulation rule changes follow
from these checks.

An alternative restoration-order trial initially fixes 46 checks and regresses
11 in that focused screen. The full 400,040-input audit instead finds **33 fixes
and 184 regressions**. It is also rejected. Its apparent improvements in the
captured problem cases do not count toward the retained engine's accuracy.
Requiring a previously active pacing effect does not resolve this: two variants
each fix 45 checks but regress 153 and 154 respectively in the expanded
1,402-input screen. Both remain excluded.

See the [control and frame audit](../evidence/experimental-wave5-controls-20261101.json)
for capture identities, validation scopes and rejected-trial results.

### Pacing threshold arithmetic

Seven skill-free, nine-runner captures isolate wisdom from equipped-skill effects.
The thresholds at effective wisdom 1,149 and 1,270.5 differ by a few millionths;
their neighboring values agree. The split-logarithm calculation reproduces both
rounding directions. Ten further captures test predictions recorded before
collection, covering eight previously untested differences and two controls.
All seventeen now match the complete recorded pacing decisions. No stat-specific
exceptions or comparison tolerances were added.

| Detailed comparison | Previous follow-up profile | With pacing-log option |
|---|---:|---:|
| Ordinary reports | 73/85 | 73/85 |
| Every recorded binary frame | 73/85 | 73/85 |
| Reference and mode histories through 15 seconds | 68/85 | 68/85 |
| Complete pacing decisions | 62/85 | 73/85 |

The change corrects **551 threshold values in eleven captures**, including the
earlier official-fixture projection. All 85 fresh complete outputs remain
identical. These are targeted observations, including repeated inputs, and do not
increase the independent sampled-race denominator.

The full 400,040-input regression retains all report pass/fail results: sampled
agreement remains **399,947/400,000**, with **53 unresolved discrepancies**, plus
40/40 targeted boundary probes. It preserves 400,039 complete outputs. One saved
variation changes a single lane observation and one runner's spurt-start time
annotation, while still matching the report. Neither build reproduces it during
fresh runs, a repeat of the full 1,000-race batch, or a replay of the preceding
34,003-race sequence. The output remains recorded as an unresolved repeatability
anomaly; it is not attributed to the numerical change.

A fresh remote control agrees with the normally reproduced output on all sixteen
immediate spurt-decision times. The isolated variant's annotation is one tick late
for one runner. Its differing lane timestamp falls between recorded remote frames
and cannot be checked from that replay. This illustrates why matching an ordinary
report does not validate every output field.

The default and complete follow-up profiles each pass **167 Rust tests** and
retain their respective **82/82** and **72/82** official-fixture results. Native
library and worker checks pass in both frame modes, including concurrent handles
and recovery after rejected inputs. The numerical option remains separate from
the original aggregate, and no stable version is promoted.

See the [numerical validation record](../evidence/experimental-pacing-log-20261101.json)
for the controlled thresholds, frozen outputs, repeatability checks and scopes.

## Validation scope

Third-party comparisons cover exported whole-second position, lane, speed and HP,
elapsed finish times, placements, start delays, planned spurt distances and
exhaustion flags. Equipped-skill panels also check activation identities, timing,
detail branches and multiplicity. Ordinary exports do not expose every frame or
all internal decisions. Matching these observations does not establish universal
1:1 equivalence or an official-game accuracy percentage.

Targeted boundary probes deliberately reuse difficult cases and are counted
separately from sampled panels. Official captures retain their own strict
validation, including recorded post-finish observations and event ordering.
The default and original aggregate preserve the official partner-cutoff rule.
The separate participant-timing experiment deliberately evaluates the conflicting
third-party behavior. Input projections do not establish that every unknown
dependency of the remote simulator is identical to the official race.
