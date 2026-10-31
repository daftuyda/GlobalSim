# New race findings — 18 October 2026

> Later result: the [21 October pacemaker correction](PACING-FIX-20261021.md)
> resolved the pacing case described here.

> Historical report: counts, open issues and test results below describe
> the stated audit and its follow-ups. See [current accuracy](ACCURACY.md)
> and [testing](TESTING.md) for the committed engine.

Follow-up: both position-rounding cases below are now corrected by the
acceleration-bonus ordering fix. The pacing case remains open. See the
[edge-case follow-up](PRACTICE-EDGE-CASES-20261018.md); the original intake
measurements below retain their historical scope.

The frozen intake adds **39,842 official races**: 39,837 practice races and
five Career races. The Career additions include two normal scenarios and three
Grand Live scenarios. All five pass the strict exposed-observation checks.
The importer now accepts both supported scenarios and requires their matching
race-start and race-end endpoint families. Original seeds and runner inputs
are retained; discrepant captures remain in the corpus.

## Practice discrepancies

The initial audit found four discrepancies. Correcting natural corner expiry
fixes one, bringing the new batch to **39,834/39,837 observed matches**.

| Case | Course / seed | First difference | Result |
| --- | --- | --- | --- |
| Natural corner expiry | 11102 / 1539255985 | Gate 8 casts skill 200382 one tick early at 24.175872803 s | Fixed; official cast is 24.242473602 s |
| Pacing transition | 10107 / 2128097749 | Gate 6 position and speed at 11.721584320 s | Unresolved; winner differs, maximum finish error 0.744079590 s |
| Position rounding A | 10305 / 382718690 | Gate 3 position at 61.805324554 s | Unresolved; 14 pre-finish position samples differ, all finish times and placements match |
| Position rounding B | 10305 / 1256702895 | Gate 12 position at 65.002105712 s | Unresolved; 22 pre-finish position samples differ, one finish time differs by 7.629394531 microseconds; placements match |

All four also reproduce with the previously validated practice engine. The
new findings are not regressions introduced by the Career corrections.

### Corner correction

At exactly 450 m on course 11102, the runner remains in the corner until moving
beyond its natural end. An explicit straight-entry event instead clears the
corner immediately, including when it coincides with the natural end. Treating
both exits as exclusive caused the early `phase==1&corner==0` healing skill.

The correction preserves explicit straight-entry behavior on course 10611.
A captured regression and adjacent-float boundary unit test protect the rule.
Across the new practice batch only this race's output changes; all 4,442
Career outputs remain unchanged.

### Pacing lead

The affected practice runner starts a Pace Up action at 11.654984 s, just before
the first observed speed difference. There is no rushing episode or timed
skill affecting this runner before that point. The decision or an earlier
unobserved pacing state is a lead, not an established cause. No timing or seed
override has been introduced.

### Shared rounding lead

Both position cases use course 10305 and the same runner build: character 1028,
raw stats 1642/439/1306/600/901, Pace Chaser, normal mood, and the same skill
loadout. Each first differs by **0.0001220703125 m**, one float32 step at that
distance. Speed, HP, lane, skills and recorded state agree. Initial probes of
guts exponent precision and spurt arithmetic produced identical float bits;
they do not explain the discrepancy.

The local Kachi comparison reproduces the same first movement differences in
these three unresolved practice cases. This corroborates a shared gap but does
not establish the official implementation's behavior between samples.

## Career follow-up

The combined Career cohort has **4,031/4,442 strict matches**. The existing
411 discrepancies remain: 410 have different start delays for every runner
and diverge at the first movement tick; one diverges later after rushing ends.
See [Career validation](CAREER-VALIDATION.md) for the startup evidence.

For the later case (course 10801, seed 1241130000), gate 18 transitions back
to normal pacing when its pacemaker gap is just above 3 m; the previous tick
is just below 3 m. A diagnostic immediate pacing update at rushing exit
delayed the first mismatch but left later differences. That experiment was
rejected and is absent from the production engine.

## Targeted collection

1. Repeat course 10305 with the exact recorded 1028 build and varied seeds and
   gates. Keep the original skill loadout; then vary one stat or skill at a
   time to separate rounding from a state-dependent effect.
2. Repeat the course 10107 lobby, focusing on gate 6's Pace Down exit followed
   by Pace Up around 8–12 seconds. Preserve full inputs and sampled replay.
3. Seek skill-free, low-wisdom outside-gate Pace Chaser NPCs with early rushing
   in larger Career fields, especially near the 3 m pacing threshold.
4. For Career startup cases, retain preceding Career state, continuation and
   forced-style fields, original seed, player/NPC identities, program and item
   effects. Collect both wins and losses without selecting replacement seeds.

## Scope

Practice checks compare pre-finish sampled movement, finish results, skill
records, durations, recipient masks and exposed mechanics using the existing
multiset definitions. Career checks additionally include post-finish samples
and supported event-stream order. These are different denominators and must
not be combined into a strict accuracy percentage. Neither establishes hidden
state or all unsampled frames. The earlier 203,572 practice races retain their
previous audit evidence and were not rerun in this intake.

[Machine-readable intake and validation evidence](../evidence/new-races-20261018-combined.json)
records hashes, audit locations, test results and individual remaining cases.
