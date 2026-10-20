# Practice edge-case follow-up

Two of the three practice discrepancies from the latest intake are corrected.
Both were caused by the addition order of overlapping duel and Conserve Power
acceleration bonuses. The pacing discrepancy remains unresolved.

## Acceleration overlap

Ordinary skill acceleration is accumulated first. The duel bonus is added next,
followed by Conserve Power, with float32 rounding after each addition. The old
implementation added these last two bonuses in the opposite order.

For the affected runner, the values at the overlap are approximately:

| Component | Acceleration, m/s² |
| --- | ---: |
| Base plus ordinary skills | 0.4730109 |
| Duel | 0.08700413 |
| Conserve Power | 0.093910605 |
| Old combined value | 0.6539256 |
| Corrected combined value | 0.65392566 |

The small acceleration difference changes an underlying speed value before
it becomes visible in a recorded position. Recorded speed is quantized more
coarsely, so it can still agree while the position differs by one float step.

Both official captures now pass all strict exposed-observation checks:

| Course | Original seed | Affected gate | Previously observed difference |
| --- | ---: | ---: | --- |
| 10305 | 382718690 | 3 | 14 pre-finish positions; finish times already matched |
| 10305 | 1256702895 | 12 | 22 pre-finish positions and a 7.629394531 µs finish-time difference |

The fixtures `duel-conserve-overlap-position` and `duel-conserve-overlap-finish`
retain the original inputs and official observations. Both fail the preceding
production build and pass the corrected build.

Earlier research established that these two bonuses follow ordinary skills,
but did not identify their order when both are active. This correction uses
the smallest change to the existing arithmetic. Grouping or widening the two
late additions also matches the two captures; the available observations do
not uniquely establish every hidden intermediate. Widening all acceleration
additions, or all speed updates, introduces unrelated discrepancies and was
rejected.

## Remaining pacing case

Course **10107**, original seed **2128097749**, gate **6** first differs at
**11.721584320 s**. Official position is 209.710037231 m; simulated position is
209.715911865 m. The official speed wire value is 1905, versus simulated 1910.
The winner differs and the maximum finish-time error is 0.744079590 s.

The trace narrows the lead to this sequence:

1. Pace Down ends at 8.591391563 s after the pacemaker gap crosses its exit
   threshold. The preceding recorded movement still matches.
2. At 11.654984474 s, the next pacing check starts Pace Up. Its wisdom roll is
   6.0790277 against a threshold of about 30.26, so a near-equality comparison
   at this lottery does not explain the mismatch.
3. The first recorded mismatch follows on the next tick. There is no rushing
   episode or self-cast timed skill before it; the runner does receive another
   runner's current-speed debuff.

Diagnostic suppression of that one Pace Up action postpones the first visible
error to 12.787181854 s, where another runner diverges. It does not fix the race.
Nearby random-stream shifts and one- or two-tick changes to earlier pacing
exit clocks also fail. Shifting the post-initialization stream by up to 256
draws in either direction makes the first position difference earlier.
These are rejected diagnostic interventions, not production rules or altered
corpus seeds.

The next useful official captures preserve this exact lobby and vary the seed,
with particular attention to Pace Down exits, the following pacing lottery,
pacemaker changes and current-speed debuffs around 7–13 seconds. Keep complete
runner inputs, skill definitions and the original replay. The sampled replay
does not expose pacing mode, cooldowns or the RNG cursor, so the exact cause
remains unproven.

## Validation

All 69 captured fixtures pass the existing pre-finish checks; 65 pass strict
validation. The four older time-zero skill-order discrepancies are unchanged.
The Rust suite remains at 136 passes and the same 11 known failures. The
prepared library agrees with the worker on 102 sequential checks and 24
concurrent runs, including both new fixtures.

The full 4,442-race Career rerun produces identical binary outputs for every
race: 4,031 strict matches and the same 411 unresolved cases.

The completed diagnostic screen of this modifier order matched observed results
for 51,811 of 51,812 races: all but the pacing case. This covers all 39,837 new
practice races (39,836 matches) and 11,975 older controls, using the existing
practice observation checks and inherited scores for identical outputs.

The production before/after binary comparison was stopped at the user's request
after 99,864 of 243,409 practice races. All 99,864 completed outputs were
identical, with no errors. The full comparison is incomplete; this is not a
fresh full-corpus official-observation audit. Investigation is paused with no
proven correction for the pacing case.

[Validation evidence](../evidence/practice-three-followup-20261018.json) records
the practice corpus comparison and official-observation screening results.
See the [original intake report](NEW-RACE-FINDINGS-20261018.md) for the frozen
cohort definitions. Neither report claims universal 1:1 game accuracy.
