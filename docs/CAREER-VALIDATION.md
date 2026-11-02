# Career race validation

GlobalSim supports Career stat adjustments, team-rank bonuses, NPC fields and
captured race inputs. Career validation keeps canonical captured seeds and
reports startup discrepancies separately from later simulation differences.

## Recorded coverage

The latest documented active inventory, frozen on **28 October 2026**, contains
12,289 Career captures. Historical audits retain 410 active discrepancies.
No additional active Career intake was found in the 26 or 28 October snapshots.
These are accumulated records, not a fresh full v77 Career audit.

The 25 October intake includes 726 races matching every checked observation.
Its narrow character-card and scenario coverage limits broader conclusions.
The [current validation summary](VALIDATION-SUMMARY-20261103.md) keeps Career
coverage separate from the completed official practice audit.

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

Startup mismatches require additional initialization evidence before changing
the seed-selection model. No outcome-dependent seed substitution is part of
the production engine.

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
