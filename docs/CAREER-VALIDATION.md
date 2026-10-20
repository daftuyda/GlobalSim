# Career race validation

The 4,442-race official Career cohort now has **4,031 strict observation
matches (90.75%)**. This measures the supplied captures, not population-wide
accuracy or hidden engine state. All races retain their captured seeds and
runner inputs, including cases that still disagree.

The original 4,437-race cohort improved as follows before the latest intake:

| Check | Before | After corrections |
| --- | ---: | ---: |
| Strict exposed observations, including event order | 659 | 4,026 |
| Observations excluding event order | 3,988 | 4,026 |
| Complete raw finish-time vector | 4,016 | 4,026 |
| Complete placement order | 4,026 | 4,026 |
| Winner | 4,150 | 4,150 |

The cohort covers 69,045 runners, 35 courses and fields of 9–18 runners.
All are turf races with team rank zero; nonzero Career rank bonuses still
require official capture validation. The earlier 203,572-race practice
benchmark is separate and uses its documented comparison definitions.

## Corrections

- **Adjusted stat precision:** Career speed and wisdom include the mode bonus
  before the final float32 rounding. Storing the intermediate product first
  produced small movement and finish differences in 34 races.
- **Stat ranks:** initial guts and wisdom values that are approximately equal
  share the tie rule, resolved by runner identity. Exact float comparisons
  misordered four result pairs after mood adjustment.
- **Replay event order:** timed skills retain their activation order when
  their durations are finalized. Spot struggles record the cached leader
  first, followed by the other gates; duels retain activation order. These
  recording changes do not alter movement or random draws.

There were no regressions among the 4,437 Career inputs. Three official
captures now protect the precision and ranking corrections in the repository's
fixture format; they fail the earlier engine and pass the candidate.

## Remaining discrepancies

**410 races diverge at startup.** Every runner's start delay differs in each
race, with movement already different at the first recorded simulation tick.
The original seeds agree with the captured packets and frozen source index.
All 410 have `continue_num=0` and `is_force_running_style=0`.

All 410 simulated player runners lose; the official player wins 163 of those
races. This is a useful correlation for investigating a conditional Career
initialization path, but does not establish a rerun, reseed or assistance rule.
A simple shifted RNG stream did not explain the representative case tested.
No replacement seeds or outcome-dependent corrections were introduced.

**One race first diverges later**, at 6.393596649 seconds, gate 18:
official position 113.639030457 m versus simulated 113.660209656 m.
The runner is a skill-free Pace Chaser NPC recovering from an early rushing
episode. Both recordings show rushing ended at that sample, but the simulated
speed and HP already differ. No nonzero-time skill or competition event has
occurred yet. Rushing exit and position-keep transitions are the next useful
inspection points; the exact cause remains unresolved. The maximum final
time error in this race is 0.201660156 seconds.

The local [Kachi simulator](https://github.com/kachi-dev/uma-race-simulator)
comparison also diverges on all 410 startup cases and at the same first
position difference in the later case. Ten positive controls have exact
finish times and orders, though their recording cadence differs. External
agreement helps localize shared gaps; it is not an official accuracy oracle.

## Capture priorities

1. Keep complete start/end responses and preceding Career state for competitive
   player builds, especially 16- and 18-runner fields. Pair wins and losses
   under similar configurations to test the initialization correlation.
2. Retain player identity, NPC types, original seed, continuation and forced
   style fields, program ID, item effects and complete runner stats/skills.
   Do not discard losses or correct seeds from outcomes.
3. Collect early rushing episodes in outside-gate, low-wisdom Pace Chaser NPCs,
   especially near their transition back to normal pacing. Preserve the full
   replay so the last matching sample can be inspected.
4. Add nonzero team-rank captures around each bonus threshold; this cohort
   cannot establish those boundaries.

## Validation limits and results

The strict checker compares the six exposed movement/state channels at
official samples, including post-finish samples, raw finish times, seven
result fields, skill identities/order/durations/recipient masks and exposed
competition event sequences. Unsampled ticks, recording cadence, global
ordering across separate event streams and unexposed result fields remain
outside its scope.

All **67 fixtures** pass the earlier pre-finish checks; **63/67** pass the
strict checks. Four older fixtures retain only time-zero skill-order
differences involving the `activate_count_start>=3` condition. The 48 rank
equivalence checks, 12 invalid-rank rejection/recovery checks, 144 prepared
library comparisons, 18 additional captured-input comparisons and 24 concurrent
comparisons pass. The Rust suite has
136 passes and the same 11 previously recorded failures; it is not fully green.

[Machine-readable results](../evidence/career-discrepancy-investigation-20261018.json)
include artifact hashes, individual fixed cases, remaining discrepancy counts,
external comparator revision and detailed test results. The external dataset
retains an itemized unresolved-case index and the immutable baseline audit.

## Latest intake

Five additional captures (two normal Career and three Grand Live) all pass the
strict checks. The importer validates each scenario against its matching
endpoint family. All 4,442 combined Career inputs were rerun with the corner
correction: 4,031 strict matches, the same 411 discrepancies, and byte-identical
outputs for every race compared with the preceding Career candidate.
[New findings](NEW-RACE-FINDINGS-20261018.md) describe the follow-up pacing
experiment and focused collection targets. The original cohort table and
runner count above retain their original scope.
