# Practice movement investigation — 25 October 2026

> Historical report: counts, open issues and test results below describe
> the stated audit and its follow-ups. See [current accuracy](ACCURACY.md)
> and [testing](TESTING.md) for the committed engine.

The earlier 16-runner practice capture on course 10611, seed 580983021,
remains an active strict failure. This follow-up found no supported engine
correction. Original inputs, validation tolerances and production behavior
are unchanged.

The capture has 85 sampled position differences, at most 1.221 mm, two encoded
stamina differences and two final-spurt distance differences. Raw finish times
and placements match. The first recorded drift appears at 35.165 seconds,
initially affecting gates 8 and 10.

## Additional checks

- **Movement precision:** Twelve alternatives retained extra precision in
  speed-times-timestep, allowed world movement, square root or the final position
  sum. Every alternative increased position discrepancies; none passed strict
  validation. Four fresh default processes reproduced the baseline byte for byte.
- **Lane target selection:** Tracing covered 2,061 passing-search calls over the
  first 730 metres, including 973 comparisons against a prior candidate and 915
  comparisons between distinct targets. No equal-cost or nearly identical
  competing targets occurred. Five positive/negative trace controls passed,
  and the instrumented race remained byte-identical to baseline.
- **Movement correction threshold:** Independent arithmetic reconstructed all
  1,248 baseline positions between 30 and 35.2 seconds. Gate 8's two previously
  identified intervention sites are far from threshold equality. The closest
  threshold elsewhere produces the same stored position with or without the
  correction.

These checks narrow particular explanations. They do not validate hidden official
state: the recording contains sparse observations, and candidate tracing covers
only targets accepted by the existing corridor checks. Artificial perturbations
that improve this one race still do not establish a general mechanic.

The most useful additional evidence is a trusted full-tick trace for the saved
lobby and seed, especially gates 7, 8, 10 and 11 between 30 and 37 seconds.
Positions, lanes, world coordinates and movement state would distinguish the
remaining upstream explanations. The original input is preserved in the existing
reference-request package.

See [diagnostic evidence](../evidence/movement-causal-followup-20261025.json)
and the [preceding investigation](PACING-FIX-20261024.md).

## Extended investigation

A second follow-up checked every float32 value within the recorded final lane
range for gates 8 and 10, changing one runner at a time at its last baseline lane
movement. All **26,844 candidate checks** completed through full race simulation;
none matched all recorded observations. The best artificial trial still leaves
17 position mismatches and introduces an earlier error at 34.099 seconds.
It was not adopted. The original case still has its original discrepancies.

An initial inverse calculation held the baseline speed schedule fixed to narrow
possible lanes. The subsequent complete range check reran the full engine,
including the blocking response to changed positions. It remains a test of
single-runner lane interventions; combinations of changed lanes and different
earlier position/world histories are outside its scope.

Tracing rejected lane candidates reproduced 9,123 corridor decisions, 8,901
target lists and 7,062 generated candidates. A focused review of 98,184 boundary
comparison records found no sensitive decision near the first drift. Earlier
close comparisons involved ineligible targets, another independent blocker or
a runner holding its lane. These are checks of simulator state, not independent
official observations.

Independent geometry calculations matched all 8,440 traced world positions
against both the older source and the optimized implementation. Interpolation
clamping and endpoint branches were inactive near the drift, and exact rounding
checks found no quaternion-normalization discrepancy. Direct decoding of all
2,176 original recorded positions also matched the validation input.

Skill 202272's target-speed bonus expires at the expected tick, 34.099391937
seconds. Gate 8 remains below its target at the two sensitive intervention
points, so the optimization that skips acceleration work at equal speed and
target does not execute there. Source review found no stale modifier total or
missing acceleration-cache dependency explaining the drift.

The long diagnostic runner initially terminated unexpectedly after preserving
8,842 completed results. Smaller resumed batches completed the remaining range;
verification confirmed 26,844 unique results and preserved the earlier records.
No individual engine failure was recorded; the initial runner termination has
no established cause. Five fresh default processes subsequently reproduced the
baseline byte for byte. Production source and strict validation are unchanged.

An indexed search found no independent capture with the same seed. A bounded
100-reference comparison found no other exact input after excluding the seed;
the other 99 captures all pass their retained strict audits. Nine same-course
controls share the same 16 runner builds and race context, but use different
gate orders; seven also differ in tied popularity assignments. Their final-engine
outputs are byte-identical to the strictly audited outputs. These controls help
define the capture conditions but do not resolve the failing race. The bounded
sample spans six sub-batches and is not a continuous chronological window.

[Extended diagnostic evidence](../evidence/movement-causal-followup-2-20261025.json).
