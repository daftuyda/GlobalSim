# Practice intake validation — 24 October 2026

> Historical report: counts, open issues and test results below describe
> the stated audit and its follow-ups. See [current accuracy](ACCURACY.md)
> and [testing](TESTING.md) for the committed engine.

> **[Pacing follow-up](PACING-FIX-20261024.md):** The pacemaker retention correction
> resolves the early pacing case. The full new intake now has 66,737 strict
> matches and one remaining numerical case. The original audit below is historical.

66,736 of 66,738 new practice captures match the checked official observations.
Two reproducible discrepancies remain active. All captures passed input and
provenance verification; there were no execution errors. Career intake was excluded.
The frozen count ended at 08:21:48 Brisbane time; later collections are outside this batch.

| Runners | New captures | Strict matches | Unresolved |
| --- | ---: | ---: | ---: |
| 12 | 660 | 660 | 0 |
| 14 | 9,700 | 9,700 | 0 |
| 15 | 1,866 | 1,866 | 0 |
| 16 | 31,128 | 31,126 | 2 |
| 18 | 23,384 | 23,384 | 0 |
| Total | 66,738 | 66,736 | 2 |

## Early pacing discrepancy

Capture `3f169903dc0ff973900793c6760b8cef055d7da9015fb4d89cf8412ca8c3c370`:
course 10501, 1,200 m turf, yielding ground, 16 runners, original seed 1717357352.
The roster contains three Front Runners, twelve Pace Chasers and one Late Surger.

Gate 4 first differs at 8.524791718 seconds: recorded position 159.223785400 m,
simulated position 159.229858398 m. Recorded speed is 18.85 m/s, versus 18.91 m/s
in the encoded simulation output. Gate 10 also has a one-unit stamina discrepancy
at this sample. Later differences affect all raw finish times and fourteen placements.

The diagnostic trace shows gate 4 leaving Pace Down on this tick. Its downhill
state remains inactive, and the shared RNG draw count stays at 1,156 across its
pace and downhill updates. The live pacemaker gap is 3.173172 m, exceeding its
3.1589794 m exit threshold; no positive speed skill cancels pacing. This narrows the immediate transition; it does not
prove that the reference uses a different exit rule or exclude an earlier hidden
state difference. Recorded skill activations before the first divergence agree.
No seed-specific correction or RNG adjustment has been adopted.

Collection target: repeat the full saved lobby and environment, preserving the
three Front Runners, gate-4 and gate-10 builds, early current-speed debuffs and
acceleration skills. Capture the original seed again if possible, plus new seeds.
Prioritize raw observations around 6–13 seconds and 110–250 m, including pacemaker
handover, Pace Down exits and the opening downhill section. The complete input
is retained with the failure; summary stats alone cannot reproduce this case.

## Small movement discrepancy

Capture `b0dd63b3423947afcbfcec686944f3810c5f8e4c45c6c80b3bf6d572b7fec337`:
course 10611, 1,600 m dirt, good ground, rainy weather, 16 runners, seed 580983021.
The roster contains three Front Runners, six Pace Chasers, four Late Surgers and
three End Closers.

Gates 8 and 10 first differ at 35.165004730 seconds, near 687–689 m. Their position
errors are respectively -0.305 mm and +0.244 mm. Gate 7 first differs at the next
recorded sample. There are 85 position mismatches, all before finish, with maximum
sampled error 1.221 mm; two encoded stamina values and two final-spurt start
distances also differ. Every raw finish time, placement, encoded speed and lane,
and checked skill/mechanic event agrees.

The onset is inside the first corner (650–875 m) and downhill section (575–775 m).
Movement arithmetic or runner interaction is a lead, not a proven cause.
Collection target: repeat the complete saved lobby on the same course and ground,
focusing on gates 7, 8 and 10 around 30–40 seconds and 600–800 m. Retain exact
positions and supplementary results: finish-only validation would miss this case.

## Verification and corpus registration

Both failures produce byte-identical complete outputs in v76 and the preceding
engine. They are not regressions introduced by the optimization. A separate
instrumented build also reproduced both outputs exactly before its traces were used.
Independent reconstruction and the canonical validator agreed for both failures
and ten passing controls. All 65 fresh/reused-process, reordered-input and output-mode
repeatability comparisons passed. Repeatability is separate from official accuracy.

The audit checked 10,310,269 official frame samples containing 169,458,885 runner
observations, with dense simulation-output integrity checks and strict event and
result comparisons. The 24-worker validation session took approximately nine
minutes; this includes reconstruction and validation and is not isolated race latency.
No engine behavior, inputs, validation tolerance or quarantine decision changed.

The registered corpus now contains 537,088 unique races: 524,727 practice and
12,361 Career. The existing 799 quarantined captures remain unchanged. The previous
470,350 captures were not rerun in this intake; their v76 comparison is recorded
separately. Two active practice discrepancies now remain. Existing Career findings
are unchanged. These counts describe observed agreement, not universal game parity.

[Machine-readable evidence](../evidence/new-practice-20261024.json) includes the
full discrepancy breakdown, provenance receipts and reproduction artifact paths.
Local original inputs, expected observations, outputs and diagnostic traces are
under `<race-dataset>/new-practice-20261024/audit/failures/`.
