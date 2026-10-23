# Practice intake validation — 23 October 2026

> **[23 October corridor follow-up](CORRIDOR-FIX-20261023.md):** The corridor correction now resolves this intake's remaining case and both older practice rounding cases. All 35,134 races in this intake pass. The results below describe the original historical audit.

**35,133 of 35,134 new practice races match the checked official observations.**
One reproducible numerical discrepancy remains active. There were no capture
integrity errors, input preflight failures, engine crashes or output-contract
failures. Career races were excluded from this intake.

The selection uses the manifest prefixes from the 08:54 Brisbane count on
23 October, deduplicated against the previous practice inventory and archived
captures. Later collections are outside this batch. All new references,
provenance receipts, original seeds, replay payloads and master identities
passed verification before simulation.

| Runners | New races | Strict matches | Unresolved |
| --- | ---: | ---: | ---: |
| 12 | 250 | 250 | 0 |
| 14 | 5,090 | 5,090 | 0 |
| 15 | 914 | 914 | 0 |
| 16 | 15,782 | 15,781 | 1 |
| 18 | 13,098 | 13,098 | 0 |
| **Total** | **35,134** | **35,133** | **1** |

Validation retains every engine tick and checks dense output integrity,
recorded positions and encoded lane/speed/stamina values, post-finish samples,
skills, durations, target associations, exposed mechanic events and supplementary
results. The engine and validator match the [previous full-corpus audit](FULL-VALIDATION-20261022.md).
No engine correction or validation tolerance change was made. Agreement on
these recorded observations does not establish universal game parity; the
[remaining validation limits](VALIDATION-HARDENING-20261022.md#remaining-limits)
still apply.

## Remaining numerical case

Capture: `4c47e5cffaa028662c05099b744a7063334701cd18c9aa3ac3164b0868dbf4b2`.

- Course **11102**, **1,800 m dirt**, **heavy** ground, **16 runners**;
  original seed **1344019483**.
- Mixed roster: three Front Runners, six Pace Chasers, four Late Surgers and
  three End Closers.
- Gate **11** (zero-based runner 10, Late Surger) first differs at
  **56.4772605896 seconds**. Expected position is **1105.271484375 m**;
  simulated position is **1105.2716064453125 m**, a **0.122 mm** difference.
- Its preceding recorded sample, at **55.4116477966 seconds** and
  **1083.799072265625 m**, agrees. This interval crosses the third-corner entry
  near 1100 m. It bounds the first observed divergence, not the exact hidden
  simulation tick responsible.
- Gate **3** (runner 2) first develops a position difference at
  **84.1820373535 seconds**. Only these two runners have recorded position or
  lane differences.
- There are **110 position mismatches**: 54 before the affected runner's
  finish and 56 afterward. Maximum sampled position error is **0.854 mm**.
  Eight encoded lane values differ by one wire unit.
- Gate 11 finishes **22.888 microseconds later** in the simulation:
  **86.139404296875 s**, versus **86.1393814086914 s** recorded.
  All other raw finish times and every finish placement agree.

Recorded speed, stamina, rushing state, blocker identity, checked skills and
mechanic events agree. A movement/geometry rounding difference is a plausible
lead, but the root cause is unconfirmed. Quantized observations cannot rule
out smaller hidden state differences.

Independent reconstruction from the original reference produced the exact
same input and complete binary output as the bulk audit. The canonical checker
reproduced the same discrepancy counts. Fresh-process, reordered-batch,
duplicate-input and JSON/binary probes all matched the baseline output:
**70 comparisons passed**, covering this failure and ten passing controls
across all five runner counts. These probes test reproducibility, not official
accuracy.

Nine existing diagnostic geometry arithmetic alternatives were also tested.
The diagnostic build's default output was first verified byte-for-byte against
the accepted engine. Every alternative retained or increased the discrepancies;
none restored strict agreement. The probe outputs are retained separately and
were not adopted as engine changes.

## Collection target

Repeat this full 16-runner lobby on course 11102, preserving the gate-11 build
and heavy-ground conditions. Focus raw observations on **54–65 seconds** and
**1080–1260 m**, spanning the corner entry and the first lane discrepancy.
The affected runner has speed 1334, stamina 552, power 938, guts 794 and wisdom
1100, with A distance, surface and strategy aptitudes. Exact skills, the full
roster and environment are retained with the original input; these summary
stats alone are not a reproduction recipe.

Capture repeated seeds where possible, plus new seeds with the same roster,
and retain raw replay precision and post-finish observations. This should help
separate corner-entry arithmetic from interactions between runners. Matching
the finish alone is insufficient: the position and lane samples must agree too.

## Corpus registration and evidence

The registered corpus now contains **470,350 unique races**, including
**457,989 practice races** and the unchanged **12,361 Career races**. The existing
799 quarantined races remain unchanged. The new discrepancy stays active and
is retained for investigation; it has not been classified as a bad capture.
The older active practice rounding case and 410 Career startup cases remain
separate from this intake. Earlier races were not rerun in this batch.

The parallel simulation-and-validation pass took **194.6 seconds** with 24
workers. This includes capture reads and Python validation; it is not a
single-race engine latency measurement and excludes intake preparation and
follow-up investigations.

The [machine-readable report](../evidence/new-practice-20261023.json) includes
the case breakdown, coverage limits, frozen artifact receipts and collection
target. Original captures, full failed input/output, per-sample differences,
repeatability records and rejected probe outputs are preserved at:

```text
D:/UmamusumeData/race-datasets/new-practice-20261023
```

The intake registry points to the expanded practice inventory and its active
view. The Career registry, quarantine registry and historical full-audit report
were preserved unchanged.
