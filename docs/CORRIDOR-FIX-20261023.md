# Passing-corridor boundary correction

The corridor rounding correction resolves the latest practice discrepancy and
both older practice rounding cases. **All 457,989 practice captures in the frozen
corpus now match the checked official observations**, including the one practice
capture still held in quarantine. Re-simulation of all **470,350 registered
captures** found **no regressions or unresolved execution errors**. The 410 active
Career discrepancies and 798 quarantined Career discrepancies remain separate.

## Cause and correction

The passing search checks whether another runner occupies an inward corridor.
It previously rounded the unshifted corridor edge to float32, then subtracted
the inward offset in wider arithmetic. At a boundary this could exclude a
runner that should block the path.

The correction evaluates the complete shifted bound in float64, then rounds it
once to float32 before the existing inclusive comparison. It applies to both
corridor bounds and all runners. No seed, runner, skill, course or replay-specific
branch was added, and neither the inputs nor validation tolerances changed.

In the latest 16-runner capture on course 11102, the decisive simulated tick is
**50.48318862915039 seconds**, when gate 11 is at **983.1047973632812 m**.
The first recorded position discrepancy appeared later, at 56.4772605896 seconds.

| Value | Lane coordinate |
| --- | ---: |
| Gate 11 lane | 0.1712033450603485 |
| Gate 15 lane, blocking the inward path | 0.0712033361196518 |
| Previous lower corridor bound | 0.07120334729552269 |
| Corrected lower corridor bound | 0.0712033361196518 |

The old bound excluded gate 15 by approximately 0.0000000111758709 lane units.
The corrected bound includes it exactly. Gate 11 therefore selects the outward
move for that tick. This removes the later position and lane drift, including
the 22.888-microsecond finish-time difference. Every checked observation in the
capture then agrees, including post-finish samples and the affected second
runner. This is a reconstruction supported by the captures; the hidden official
per-tick arithmetic is not directly observed.

## Captured regression cases

| Capture prefix | Course | Runners | Original seed | Status |
| --- | ---: | ---: | ---: | --- |
| `4c47e5cffaa02866` | 11102 | 16 | 1344019483 | Latest active case corrected |
| `1a711441c217d665` | 10404 | 12 | 605833787 | Older active case corrected |
| `f400e64edf548950` | 11002 | 16 | 554335710 | Corrected; quarantine retained |

All three are now permanent strict fixtures with their original inputs and
official expected observations. The old engine passes 78 of the expanded 81
fixtures and fails exactly these three; the corrected engine passes all 81.
The focused boundary unit test also checks the adjacent float: a blocker on
the lower edge blocks the inward move, while moving it one float below the
edge allows that move. All **155 Rust unit tests pass**.

Fresh-process, reordered-batch, duplicate-input, binary and two JSON output
probes pass **70 comparisons across 13 inputs**: the three corrected races and
ten passing controls. These are repeatability checks, separate from comparison
with official observations.

## Full frozen-corpus regression check

The corpus is frozen at the 23 October 08:54 Brisbane intake. Later collections
are outside this report. The new 35,134-race batch and all 435,216 older captures
were re-simulated using the clean release build and original seeds.

| Partition and cohort | Races | Strict matches | Discrepancies |
| --- | ---: | ---: | ---: |
| Active career | 11,563 | 11,153 | 410 |
| Active collected-practice | 444,801 | 444,801 | 0 |
| Active official-archive | 13,187 | 13,187 | 0 |
| Quarantined career | 798 | 0 | 798 |
| Quarantined collected-practice | 1 | 1 | 0 |
| **Entire frozen corpus** | **470,350** | **469,142** | **1,208** |

Exactly **470,347 complete binary outputs are identical** to their
previously audited outputs. Their original strict validation results are reused
only after both input and full-output SHA256 equality. All **3
changed outputs** were checked against the original official observations with
the unchanged expanded validator. **3 previous failures are fixed**;
no previously passing race fails. Every dense engine frame is retained in the
output being compared, including unrecorded ticks.

Original reference content hashes, pinned projections, input hashes, validator
source hashes and candidate source receipts were verified. The earlier full
audit's one resolved preflight retry remains part of that baseline. Its original
failed attempt and successful retry are retained. The comparison resumed from
36,864 saved results with remaining files read in natural path order to reduce
disk seeking; this changed scheduling only.

Six attempts during this run encountered resource errors: five Python
`MemoryError` exceptions and one truncated engine reply. The worker log records
Windows error 1450, insufficient system resources, for the truncated reply.
All six unchanged inputs passed independent full canonical rechecks with
complete output hashes identical to the original audited engine. A further
**35 fresh-process, order and binary/JSON comparisons** passed. Original failed
attempts and worker logs are retained; the effective totals apply the separate
successful retries. No unresolved execution error remains, and these resource
errors were not reclassified as capture defects.

The active corpus has **469,141 strict matches in
469,551 races**. All 410 active failures are the
previously documented Career startup cases. Quarantine membership is unchanged:
798 Career captures and one now-passing practice capture remain quarantined.
The newly corrected practice capture is not evidence of corrupted capture data.

## Evidence and limits

The [machine-readable report](../evidence/corridor-fix-20261023.json) records the
build and source receipts, complete comparison totals, changed cases, remaining
Career failures, fixtures and repeatability results. The current intake and
full-validation registries point to it. Historical audit reports remain intact.
Full diagnostic and comparison artifacts are retained at:

```text
D:/UmamusumeData/race-datasets/practice-fix-20261023
```

Agreement covers the recorded observations checked by the validator. It does
not establish universal game parity or equality of every hidden state; the
[documented validation limits](VALIDATION-HARDENING-20261022.md#remaining-limits)
still apply. Captured provenance already verified by the original audits is
reused after reference content equality. Archived skill definitions remain
pinned to their original audit projection. Comparison wall times include disk
reads and validation and are not single-race engine latency measurements.

Further captures targeting this mechanism should preserve close overtaking
traffic near a lateral corridor edge. For the latest lobby, retain the full
roster and original conditions and focus on the interval around 50.48 seconds,
before the later visible corner-entry divergence.
