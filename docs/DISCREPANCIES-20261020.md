# Discrepancy investigation — 20 October 2026

The startup event-order correction resolves **4,681 races**. A fresh run of all
**95,062 races in the new batch** has **94,566 strict matches**, **496 remaining
discrepancies**, and no audit errors or newly failing races.

| Cohort | Races | Strict matches before | Strict matches after | Remaining |
| --- | ---: | ---: | ---: | ---: |
| Practice | 90,283 | 85,600 | 90,281 | 2 |
| Career | 4,779 | 4,285 | 4,285 | 494 |

These are matches to exposed official observations, not a claim of complete
game accuracy. The older 247,851 races were not rerun in this investigation.
The [original audit](NEW-RACES-20261020.md) remains a historical baseline.

## Corrected startup event order

The constructor evaluates each runner's startup skills twice. A skill such as
Groundwork can become eligible on the second pass after other startup skills
activate. The replay records the first pass across the field before the
second-pass activations.

The engine now retains which activations came from that repeat pass and uses
it when ordering simultaneous replay events. This applies to the activation
pass rather than a particular skill ID. It does not change skill application,
movement, seeds, or random draws.

All 4,681 affected races now pass strict checks. Exactly those 4,681 practice
outputs changed; all other practice outputs are byte-identical to the archived
baseline. All **69 strict regression fixtures pass**, including the four that
previously failed startup event ordering. A focused check also passed all 256
previously matching control races.

## Remaining cases

| Family | Cases | Evidence and next investigation |
| --- | ---: | --- |
| Career initialization | 492 | Every runner's start delay differs; movement differs at the first recorded tick. Investigate the conditional Career initialization path. |
| Career pacing | 2 | Both first differ at 8.524792 s after a simulated PaceDown exit. Investigate exit-gap inputs and earlier shared random draws. |
| Practice pacing | 1 | First differs at 10.655987 s around a PaceUp exit and phase transition. Investigate effect termination and earlier shared state. |
| Practice rounding | 1 | First recorded position difference is about 0.000244 m; maximum finish error is about 0.0000305 s. Investigate movement correction and intermediate rounding. |

### Career initialization

All 492 simulated player runners lose; the official player wins **229** of
these races. The group spans ten courses, including 271 races on course 10501.
There are 297 fields of 16 runners and 147 fields of 18.

This repeats the earlier loss correlation and suggests investigating a
conditional initialization path. It does not prove that the server retries,
changes seeds, or applies assistance. The original packet seeds remain intact.

On one representative case, 345 diagnostic seed candidates covering nearby
seeds, original-stream derived seeds, repeated reseeding, and single-bit changes
matched none of its 16 start-delay values. This rejects those tested candidates
for that case; it does not rule out every possible initialization rule.

### Pacing

The two Career races have course 10501, distance 1,200 m, 16 runners, program 75,
and turn 42:

| Seed | First differing gate | Simulated PaceDown exit | First recorded mismatch | Maximum finish error |
| ---: | ---: | ---: | ---: | ---: |
| 957197632 | 5 | 8.458192 s | 8.524792 s | 0.290115356 s |
| 488112673 | 13 | 8.458192 s | 8.524792 s | 0.204067230 s |

Neither first-differing runner is rushing at that point. Both leave PaceDown
when the simulated pacemaker gap crosses the stored exit threshold. This
localizes the symptom; it does not establish whether the incorrect input is
the threshold, pacemaker state, or an earlier random-draw decision.

The practice race is course 10901, 1,200 m, 12 runners, seed 887706323, gate 8.
PaceUp begins at 8.258392 s and ends at 10.655987 s, near the transition into
the middle phase. Its maximum finish error is 0.577621460 s and the winner differs.

The retained local Kachi simulator also first diverges at the same positions
in both Career pacing races. It also misses the representative Career startup
case. These comparisons used the captured seeds and full raw runner records,
with explicit loader compatibility adjustments. An external simulator is
another comparison, not an official oracle.

### Rounding

Course 10404, 2,600 m, 12 runners, seed 605833787 first differs at 87.378693 s,
gate 4: official position 1651.5216064453125 m, simulated position
1651.5213623046875 m. Winner and all placements match.

The traced runner is blocked while climbing; world-distance movement
correction is active. Six isolated arithmetic variants tested float versus
wider world-distance norms and ratios, float interpolation, quaternion
reciprocal normalization, and blocking-cap ratio/rate calculations. None
cleared the case. Several introduced substantially earlier or larger errors.
No arithmetic variant was adopted.

## Targeted capture priorities

1. **Career startup:** retain complete start/end responses and preceding Career
   state for competitive builds on the affected courses. Preserve wins and
   losses, program ID, player identity, continuation and forced-style fields,
   NPC types, item effects, and original seeds. Compare matched configurations
   across outcomes without replacing inputs from replay results.
2. **Career pacing:** prioritize 16-runner, 1,200 m course 10501 fields with
   Pace Chasers and multiple Front Runners. Retain the entire replay, particularly
   the first 12 seconds and pacemaker handovers around 150–200 m.
3. **Practice pacing:** prioritize 12-runner course 10901 fields with Pace Chasers
   crossing into the middle phase while catching the pacemaker. Preserve the
   first 15 seconds, including other runners' rushing and skills.
4. **Rounding:** prioritize course 10404 with runners blocked on the uphill
   around 1,650 m. Dense observations around 85–90 seconds would help separate
   speed-cap arithmetic from world-distance correction.

These are capture filters suggested by the symptoms, not proven causal
conditions. Keep failing cases in the corpus.

## Validation and reproduction

- Full practice audit: **90,283 freshly simulated and scored races**, 567.4 s.
- Full Career recheck: **4,779 freshly simulated and scored races**, 80.7 s.
- Strict regression fixtures: **69/69 pass**.
- Rust library unit suite: **138 pass, 11 fail**. The archived pre-fix source,
  using the same test fixtures, reproduces the same 11 failures. These cover
  existing Python-vector comparisons, geometry, HP/spurt, and duel tests;
  the unit suite is not fully green.
- Diagnostic instrumentation reproduces the baseline frame bytes and results
  in all five focused cases.
- Audit timing includes parallel process communication, decoding, scoring,
  and saved evidence. It is not isolated simulation latency.

The [investigation evidence](../evidence/discrepancies-20261020.json) records
the engine, source snapshot, complete result files, focused fixtures, traces,
rejected arithmetic probes, startup analysis, and independent comparison.
The evidence is under
`D:/UmamusumeData/race-datasets/discrepancies-20261020`.

No remaining behavioural discrepancy has a sufficiently supported correction
from this investigation. No seed replacement, case-specific timing adjustment,
or outcome-dependent patch was added.

The [subsequent follow-up](UNRESOLVED-FOLLOWUP-20261020.md) identifies different
diagnostic seeds for the Career startup group and tests further pacing and
rounding hypotheses. Those diagnostic results do not replace the canonical
validation counts above.
