# Third-party simulator comparison — 31 October 2026

This report compares exported uma.moe observations with GlobalSim. The reference
is another simulator, so these percentages are not official-game accuracy.

## Latest collection

The collection contains **240,000 distinct input/seed pairs** from 240 lobbies,
covering 22 courses and 9–18 runners with no equipped skills. It adds no duplicate
input/seed pairs to the preceding 197,000-race third-party cohort.

| Engine configuration | Matching all checked exported observations | Agreement |
|---|---:|---:|
| Committed production engine | 213,745/240,000 | 89.0604% |
| Separate, unapplied research configuration | 239,967/240,000 | 99.98625% |

These checks compare the available outcomes, diagnostics and exported trajectory
samples. Ordinary downloads do not expose all internal decisions or every tick.
Missing required observations are not counted as matches. This skill-free cohort
does not measure skill activation accuracy.

## Accepted passing corrections

Equal-cost passing routes consider both target reach and body clearance. The
preference applies only when both routes move toward their obstacle-passing side;
two outward edges on opposite sides of the runner must retain the original tie.
The corrections resolve an 18-runner Kokura 1200 m case and a 15-runner Sapporo
1500 m case. [Field mechanics](FIELD-MECHANICS.md) describes the general rule.

The final refinement preserved all 52,096 checked official complete outputs.
Across 437,000 third-party inputs it left 436,999 complete outputs unchanged and
corrected the Sapporo discrepancy, with no regressions. All 163 Rust tests and
82 official fixtures pass. These regression counts include pre-existing reference
differences; they are not 437,000 matching third-party races.

## Remaining differences and reference limits

All 12 requested detailed captures reproduced their original exported observations.
The 33 remaining research differences are concentrated on 1,000–1,300 m courses,
with the first exported difference appearing 7–18 seconds into the race. Observed
families include pacemaker selection and lead-competition entry/participants.

A broader lead-competition change matches two more uma.moe cases but fails the
official `lead-competition-partner-past-entry-cutoff` regression. A resource-checked
uma.moe reproduction follows that conflicting behavior. The production engine
therefore retains the official-capture behavior; the higher-agreement research
configuration remains unapplied.

Further comparisons should pair each suspected boundary with matching controls
and an official capture where possible. Keep runtime geometry, inputs and seeds
consistent, and use detailed replays when ordinary downloads cannot distinguish
the competing explanations.

## Evidence

- [Current comparison summary](../evidence/documentation-snapshot-20261031.json)
- [Passing-clearance validation](../evidence/passing-clearance-20261031.json)
- [Passing-direction validation](../evidence/passing-route-direction-20261031.json)
- [Earlier 45,000-race diagnostic snapshot](../evidence/umamoe-movement-20261031.json)

The earlier snapshot predates the accepted passing corrections and expanded
collection. Its unchanged-source statement and diagnostic counts apply only to
that earlier experiment. Official dataset membership is unchanged by this work.
