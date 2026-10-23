# Full corpus validation

> **[23 October corridor follow-up](CORRIDOR-FIX-20261023.md):** The corridor correction resolves both practice discrepancies in this historical corpus. The newer full regression check covers 470,350 captures with no regressions. The results below describe the original historical audit.

The expanded validator checked all **435,216 races in the frozen registered
corpus**, using the original seeds and inputs. **434,006 of 434,417 active races
match the checked official observations.** All 411 active discrepancies were
already present in historical audits. No new simulator discrepancy was found.

## Results

| Partition | Races | Strict matches | Discrepancies |
| --- | ---: | ---: | ---: |
| Active collected practice | 409,667 | 409,666 | 1 |
| Active archived practice | 13,187 | 13,187 | 0 |
| Active Career | 11,563 | 11,153 | 410 |
| **All active races** | **434,417** | **434,006** | **411** |
| Quarantined Career | 798 | 0 | 798 |
| Quarantined practice | 1 | 0 | 1 |
| **Entire frozen corpus** | **435,216** | **434,006** | **1,210** |

Quarantine classifications were retained. A quarantined failure is not counted
as an active-corpus failure, and exclusion is not a simulator correction.
Captures collected after the frozen intake are outside this audit.

The active Career failures are the 410 previously documented
[startup cases](CAREER-VALIDATION.md). The practice discrepancy is the existing
[20 October rounding case](DISCREPANCIES-20261020.md): course 10404, 2,600 m,
12 runners, seed 605833787. Its input and complete binary output hashes match
the historical failed audit exactly. It first differs at 87.378693 seconds,
has 126 differing recorded positions and one differing speed value, and one
runner finishes 30.517578125 microseconds later. Winner and placements match.
It remains active and unresolved; no seed or arithmetic adjustment was applied.

## What was checked

Every race went through reference/replay integrity checks, available provenance
checks, capture preflight, simulation, dense output integrity and strict
recorded-observation comparisons. These include post-finish samples, skill
identities, durations, ordering, branch/target associations, exposed competition
events and supplementary result fields. All completed outputs pass the internal
output contract, including finish-rank and cast-accounting consistency.

| Checked surface | Count |
| --- | ---: |
| Dense simulation frames | 592,146,924 |
| Dense runner samples checked for integrity | 7,059,322,509 |
| Official runner samples in the comparison | 753,344,679 |
| Post-finish official runner samples | 119,765,325 |
| Official skill branch records | 55,864,187 |
| Supplementary result values | 36,271,634 |

There were no unknown replay event layouts, nonzero activation flags or
populated extension sections. The retained activation flags are still not
independently compared with an engine equivalent. The
[remaining validation limits](VALIDATION-HARDENING-20261022.md#remaining-limits)
still apply: matching recorded observations is not proof of universal game
parity or of all hidden mechanics.

The bulk checker uses NumPy to check every dense scalar and timestamp without
constructing a Python dictionary for every unrecorded frame. Recorded frames
still use the canonical comparison logic, and any failure falls back to the
full checker for diagnostics. Both paths produced identical complete reports
on all 78 captured regression fixtures. Three mutation-test groups cover
invalid unsampled states, timestamps, missing observations, result differences
and legacy timestamp collisions.

## Repeatability and recovered validation errors

Repeatability probes covered all 1,210 discrepant inputs, 78 regression fixtures
and one additional control. They included fresh processes, changed batch order,
duplicate requests, binary output, ordinary JSON output and JSON replay-worker
output. No engine output difference was found in the completed comparisons.

There were 6,888 recorded process/output comparison attempts: 6,887 matched and
one raised a Python comparison exception. That exception's saved output matches
the baseline exactly across all 1,015 frames; ten direct rechecks and all 325
checks in the repeated 64-input batch passed. An interrupted parallel probe run
also retained its partial artifacts and completed batches before resumption.

One other Python exception occurred during input preflight, before simulation.
Its unchanged saved input passed 20 repeated preflights, a full canonical audit
and 14 process/output probes. Independent reconstruction from the original
capture produced the same input and passing output. The original failed attempt
remains in `results.jsonl`; the final report applies the separately retained
successful retry. No unresolved input or execution error remains. These errors
are documented as validation-run issues, not evidence of tainted captures or
simulator nondeterminism.

## Evidence and baseline

The [machine-readable report](../evidence/full-validation-20261022.json) contains
partition totals, individual discrepancies, coverage gaps, runner-count groups
and artifact receipts. Full inputs, expected observations, failure outputs,
worker logs, retry evidence and the frozen audit configuration are retained at:

```text
D:/UmamusumeData/race-datasets/full-validation-20261022
```

The capture repository's `.local/race-datasets/latest-full-validation.json`
registers this baseline separately from the older intake reports. Earlier results
and quarantine records were preserved. The engine binary and validator source
receipts were verified at completion. Archived skill definitions use the pinned
archive template; this audit does not revalidate the original archive master
databases. Source definitions, known gaps and original inputs were not inferred
from outcomes or silently replaced.

The complete audit, preparation, retries and final verification took about
68 minutes. This is validation workflow time, including disk reads and Python
checks; it is not a benchmark of individual race simulation latency.
