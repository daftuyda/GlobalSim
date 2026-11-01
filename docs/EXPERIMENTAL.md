# Experimental comparison profiles

GlobalSim provides opt-in rules for investigating differences with uma.moe's
exported races. These profiles retain the same seeded worker and prepared-library
interfaces. They are research configurations of v76, not a new stable release.

## Build

The combined profile enables both experiments:

```sh
cargo build --release --locked --features experimental-umamoe --target-dir target/experimental-umamoe
cargo test --release --locked --features experimental-umamoe --target-dir target/experimental-umamoe
```

To evaluate only the duel-stamina arithmetic:

```sh
cargo build --release --locked --features experimental-duel-hp --target-dir target/experimental-duel-hp
cargo test --release --locked --features experimental-duel-hp --target-dir target/experimental-duel-hp
```

Use the worker or library from that profile's release directory. The feature
applies to both interfaces and every seed. `experimental-umamoe` enables
`experimental-duel-hp` and `experimental-gap-refresh`; either component can also
be selected independently. A build without these features uses the default rules.

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

## Recheck official fixtures

```sh
python tools/check_accuracy_regressions.py --engine target/experimental-umamoe/release/race_engine_rs_v2.exe --report target/experimental-umamoe/official.json
```

Use the duel-only directory to check that profile; omit `.exe` on Linux or macOS.
The combined profile is expected to fail some official fixtures. The validator
keeps its normal strict comparison and returns failure for those disagreements.

## Results

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
across 400,040 races. It is excluded from these builds. Remaining discrepancies
include early pacemaker transitions and lead-competition entry and membership.

See the [validation record](../evidence/experimental-profiles-20261101.json) for
the checked profiles, fixture failures and audit scope.

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
Known lead-competition differences remain unresolved; the experimental profiles
preserve the official partner-cutoff rule.
