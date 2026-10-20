# Dataset quarantine

492 Career captures are quarantined for a **captured-seed/replay mismatch**.
Their recorded start delays imply a different seed from the original input.
The cause remains unresolved; this does not establish that their replays are
invalid or that the simulator reproduces the server's seed-selection behaviour.

The [portable quarantine index](../evidence/quarantine-20261020.json) identifies
the exact replay hashes. The original captures, seeds and historical audit
results remain preserved in the local dataset. No captured seed was replaced.
The local Career recheck tool excludes these hashes by default and supports
explicit inclusion for research. Archived audit scripts retain their original
selection so historical runs remain reproducible.

| Latest audited batch | Races |
| --- | ---: |
| Original batch | 95,062 |
| Quarantined | 492 |
| Active batch | 94,570 |
| Strict matches | 94,566 |
| Active unresolved discrepancies | 4 |

This is a partition of existing results, not another simulation run. The
original result remains 94,566 matches and 496 unresolved cases. The older
247,851 races were not rerun in this batch.

491 quarantined races match all strict observations when given a seed inferred
from their replay. These are diagnostic matches, not validation passes. The
remaining quarantined race also exposes a later pacing discrepancy.

Quarantine applies only to the listed identities. New failures are investigated
individually. Restoration requires evidence resolving the mismatch or a verified
seed mapping available before simulation, followed by explicit review of the
manifest and active dataset views.
