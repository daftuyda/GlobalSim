# Career race validation

GlobalSim supports Career stat adjustments, team-rank bonuses, NPC fields and
captured race inputs. Career validation keeps canonical captured seeds and
reports startup discrepancies separately from later simulation differences.

## Recorded coverage

The latest documented inventory, frozen on **28 October 2026**, contains 13,174
Career captures: 12,289 active and 885 quarantined. Historical audits retain
410 active Career discrepancies. No additional Career intake was found in the
26 or 28 October snapshots. These are accumulated records, not a fresh full
Career audit of the 31 October source revision.

The most recent Career intake on 25 October tested 813 races: 726 matched every
checked observation and 87 disagreed at startup. Those 87 were subsequently
quarantined with their original evidence retained. The intake uses one character
card and scenario 3, limiting conclusions about other Career configurations.
See the [intake findings](NEW-RACES-20261025.md) and
[quarantine policy](DATASET-QUARANTINE.md).

## Supported mechanics

- **Adjusted stats:** `singleMode` adds the Career bonus at the adjusted-stat
  stage. Speed and wisdom retain wider intermediate arithmetic until the final
  float32 store.
- **Team rank:** ranks 1–5 add 50 to base stats, 6–10 add 30, 11–15 add 20,
  and 16–20 add 10. Zero or ranks from 21 onward add no bonus. Apply this
  through `teamRank`, after overcap halving and before mood; do not pre-add it.
- **Stat ranking:** approximately equal adjusted guts or wisdom values use the
  runner-identity tie rule.
- **Replay recording:** skill and competition observations retain their defined
  sequence, including simultaneous starts and finalized durations.

Nonzero team ranks have synthetic equivalence, rejection/recovery and API
coverage. Official capture validation of nonzero ranks and exact tier boundaries
remains outstanding. The [mathematics](MATHEMATICS.md) and
[interface](INTERFACE.md) guides define the input and calculation order.

## Unresolved startup behavior

The retained startup family disagrees in every runner's start delay and diverges
at the first recorded movement sample. Original seeds remain consistent with
their captured packets. Reconstructing a different seed from the replay can
recover matching observations, but does not explain how the game selected it.

In the 87-case intake, each original-seed result fails a mandatory Career goal.
Some other goal failures retain the original seed, and some official replays
still fail the goal. This supports further investigation of conditional retries
or result selection; it does not establish a fixed reroll probability, a seed
transformation or capture corruption. No outcome-dependent seed substitution
is part of the production engine.

Earlier late-race pacing cases were resolved by the
[pacemaker restoration correction](PACING-FIX-20261021.md). Historical reports
retain the failures observed before that correction.

## Capture priorities

1. Preserve full pre-race state, mandatory goals, start/retry responses and linked
   completion data for both wins and losses. Include multiple character cards
   and scenarios.
2. Keep player identity, NPC type, continuation and forced-style fields, items,
   raw stats, skills and any exposed effective seed.
3. Collect controls where a failed mandatory goal retains its original seed.
4. Capture nonzero team ranks on both sides of each stat-bonus boundary.

Strict validation compares recorded movement/state samples, raw results, skill
branches/order/durations/recipient masks and competition sequences. Hidden
initialization state and unsampled official ticks remain outside the observed
surface. See [accuracy](ACCURACY.md) and [testing](TESTING.md).
