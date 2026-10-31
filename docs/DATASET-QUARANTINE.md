# Dataset quarantine

Quarantine separates explicitly identified captures from active validation while
preserving their original inputs, seeds, replays and historical results. It does
not turn a discrepancy into a simulator pass or establish that a capture is corrupt.

## Recorded scope

The latest documented inventory, frozen on **28 October 2026**, contains:

| Capture type | Registered | Quarantined | Active |
|---|---:|---:|---:|
| Practice | 696,779 | 1 | 696,778 |
| Career | 13,174 | 885 | 12,289 |
| Total | 709,953 | 886 | 709,067 |

The most recent quarantine action added 87 Career captures on 25 October.
One quarantined practice capture now has a passing retained audit; its manifest
membership remains unchanged. Historical active findings still include one
practice discrepancy and 410 Career discrepancies. Inventory membership and
validation status are separate records.

See the [latest intake](NEW-RACES-20261028.md),
[87-case quarantine report](CAREER-QUARANTINE-20261025.md) and
[quarantine evidence](../evidence/career-quarantine-20261025.json).

## Interpretation and restoration

Career startup discrepancies can match when rerun with a seed inferred from the
replay. These are diagnostic matches, not canonical passes: inference uses the
answer being tested. Conditional Career goal-failure handling remains a possible
explanation, and the general rule is not established.

New discrepancies are investigated individually. Keep exact capture identities,
original and diagnostic inputs, the reason for exclusion and the audit scope.
Explicitly include quarantined records when investigating them; preserve the
selection used by historical reports.

Restore a record after a verified input mapping available before simulation or
a general engine correction reproduces its original observations. Review the
manifest change explicitly and rerun the relevant checks. Changing selection,
substituting a replay-derived seed or loosening a tolerance is not an accuracy fix.
