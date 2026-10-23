# Practice competition recording and Career quarantine

> **[23 October corridor follow-up](CORRIDOR-FIX-20261023.md):** The isolated practice rounding case (`f400e64edf548950`) now passes after the corridor correction; its quarantine membership is unchanged. The results below describe the original historical audit.

The competition recording correction resolves **all 172 event-ordering
discrepancies** from the 22 October intake. It changes observation ordering
only: every dense frame byte, every other output field and the competition
event multiset remain identical for all 172 corrected races.

## Recording order

The captures distinguish three stages:

1. Form the initial Front Runner base-style roster, including Runaways and
   excluding other strategies. Preserve the framework comparison sort's tie
   permutation for this roster; groups above sixteen entries do not retain
   ascending gate order.
2. Record the Runaway competition before the ordinary Front Runner competition
   when both start in the same tick.
3. Within each competition, record its cached leader first, then invited
   partners in the initial base-style roster order.

This is a capture-supported reconstruction of the observed ordering. Movement,
skill processing, random draws and competition state transitions retain their
execution order. Duel event positions and their activation sequence are
preserved separately.

A whole-field sorting candidate fixed the original failures but introduced
1,356 practice and 44 Career regressions. It was rejected. The accepted
base-style roster rule fixes all 172 failures with **zero regressions** across
46,880 practice and 1,441 Career races. Four captured fixtures cover a large
tied roster, two simultaneous strategy groups, Runaways sharing the base-style
roster and exclusion of other strategies. Three unit tests cover the same
boundaries, preserved duel slots and recording while a solver is borrowed.

## Active validation status

| Cohort | Active races | Strict matches | Unresolved |
| --- | ---: | ---: | ---: |
| Practice | 46,879 | 46,879 | 0 |
| Career | 1,302 | 1,302 | 0 |
| New active batch | **48,181** | **48,181** | **0** |

After the correction audit, the remaining numerical practice case was isolated
at the user's request. These active counts exclude that unfixed race and the
139 Career inconsistencies. The original audit results remain unchanged;
exclusion is not a simulator correction.

All **154 Rust library tests** and **78 strict regression fixtures** pass.
This batch validates observed outputs and the output contract; it does not
establish universal game parity or represent a rerun of the entire corpus.

## Career quarantine

The 139 Career seed/replay inconsistencies are now excluded from normal corpus
selection, as requested. Original reference data, seeds, submitted inputs,
master data and diagnostic evidence are preserved in a reversible quarantine.
The captured seeds were not replaced with inferred seeds.

The cumulative quarantine contains **799 races**: 798 Career captures and the
one isolated practice case below. The frozen registered corpus contains
435,216 unique races, of which **434,417 are active**. The standard
Career selector returns 11,563 active races from the 12,361-race combined
Career manifest; explicitly including quarantine restores all 12,361.

Release requires evidence resolving the captured seed/replay mapping and a
review before restoring the races. Quarantine records an input inconsistency,
not a proven cause such as capture corruption or server reseeding.

## Isolated numerical case

Capture `f400e64edf548950c2acb43eb21886a6d0855ff3656cd2ce27a5c9568636e7e9`
remains unresolved and is excluded from active validation: course 11002,
1800 m, 16 runners, seed 554335710,
runner index 11. The first sampled position difference appears at
58.6084861755 seconds, and finish time differs by approximately 30.5
microseconds. Validation tolerances and the original input are unchanged.
Its reason is `unresolved-numerical-drift`, distinct from the Career seed/replay
inconsistencies. Reference data, input, expected and actual outputs, and the
diagnostics are preserved under
`D:/UmamusumeData/race-datasets/quarantine/practice-numerical-drift-20261022`.
Restoration requires a general fix passing the capture's full strict checks
and regression controls, followed by review. Explicit inclusion remains
available for research. The [isolation receipt](../evidence/practice-isolation-20261022.json)
records the partition and active counts.

Small target-speed changes did not restore the replay. Diagnostic one-store
rounding perturbations can recover the finish time while leaving position
samples wrong, so a matching finish alone is insufficient evidence for a fix.
These experiments are confined to a private probe build; none is part of the
production engine. Seven target-speed probes, 152 single-store perturbations
and nine geometry arithmetic alternatives did not restore full agreement.
The geometry alternatives either retained the original error or introduced
additional mismatches. Further investigation should retain dense movement,
lane and world-coordinate evidence around the first divergence.

The original [validation report](../evidence/practice-fixes-20261022.json) records the
accepted engine, checks, quarantine receipt and unresolved case. Private
reproduction artifacts are under
`D:/UmamusumeData/race-datasets/practice-fixes-20261022`; original intake
results remain under `new-races-20261022`.
