# Unresolved race follow-up — 20 October 2026

> Historical report: counts, open issues and test results below describe
> the stated audit and its follow-ups. See [current accuracy](ACCURACY.md)
> and [testing](TESTING.md) for the committed engine.

The largest discrepancy group now has a substantially clearer explanation:
**all 492 Career startup cases have recorded start-delay sequences consistent
with a seed different from the captured seed**. Using that inferred seed in
isolated diagnostic runs makes **491 of the 492 races pass every strict
observation check** with the existing engine.

This is diagnostic evidence, not 491 new validation passes. Inferring a seed
from the replay uses information unavailable before simulation. The canonical
inputs retain their captured seeds, and the new batch still has **94,566 strict
matches and 496 unresolved races out of 95,062**.

## Seed investigation

The diagnostic reconstructs possible legacy `System.Random` seeds from the
exact recorded start delays of runners without delay-modifying skills.
It accounts for float quantization and constructor overflow branches, searches
the first 512 possible delay-block offsets, and verifies candidate sequences
with the exact generator implementation.

Candidate seeds are then tested against the complete recorded race, including
movement, results, skills, durations, targets, and mechanic events. Those later
observations are not used to derive the seed.

| Check | Result |
| --- | ---: |
| Startup cases examined | 492 |
| Cases with one inferred seed magnitude | 492 |
| Inferred seeds equal to captured seeds | 0 |
| Complete strict diagnostic matches using the inferred seed | 491 |
| Previously passing control races examined | 32 |
| Controls recovering their captured seed and retaining a strict match | 32 |
| Independent generator seeds checked against the four affine families | 128 |

The initial representative search covered 4,096 offsets. The group-wide search
covered 512 and found a candidate for every case. Seed sign is ambiguous under
the generator's absolute-value initialization; the diagnostic records magnitude.
Every accepted candidate was checked using the actual generator, not only the
affine approximation.

The original seed from every one of the 492 raw start responses was verified
against the corpus. None of those start or end responses exposes the inferred
seed as an integer value. These checks used hash-verified original packets.

An independent race-end cross-check also supports the recorded replay:

- Player placement agrees with the official end response in **492/492** cases.
- Scaled player finish time agrees after integer truncation in **490/492**.
  The other two differ by one reported time unit (0.0001 s).
- All 32 controls agree on both checks.

This rules out an import-time seed conversion error for these cases. It strongly
supports a mismatch between the captured seed and the random stream represented
by the replay. It does **not** establish the server's seed-selection mechanism.
A retry, conditional initialization, or another server-side path remains a
hypothesis. No predictive seed rule has been established.

Do not replace the captured seed, mark these cases as canonical passes, or use
replay-derived seeds as production simulator inputs.

## Additional pacing example

The one startup case that remains nonexact even with the inferred seed has
matching start delays and exposes another later pacing discrepancy:

| Property | Value |
| --- | --- |
| Replay hash | `8a664ff9d9a35614e5a7bdebbe59eee17e5621f76425d753d83b7caaf5945934` |
| Captured seed | 1012072510 |
| Diagnostic inferred seed | 1180502289 |
| Course and field | 10501, 1,200 m, 16 runners |
| First recorded difference | 8.524791718 s, gate 2 |
| Official position | 151.45883178710938 m |
| Simulated position | 151.46923828125 m |

This is a third Career example at the same first-difference time as the two
previous pacing cases. It remains part of the 492 startup failures, so it must
not be added again when counting distinct unresolved races.

## Pacing hypotheses tested

The isolated builds tested:

1. Restoring the Front Runner pacemaker only at section boundaries.
2. Disabling the between-section restoration.
3. Choosing the restored runner from the current rather than previous ranking.
4. Continuing to refresh the restored runner until the next section boundary.
5. Combining that refresh window with the current ranking.
6. Continuously following the leading runner of the forward group, using either
   the current or previous ranking.

Keeping the restoration window open makes the Career race with captured seed
488112673 pass its complete strict comparison. However, it fails the existing
`pacemaker-restoration-after-selection-window` regression: **68/69** fixtures
pass, compared with **69/69** for the validated engine. Other probes leave
discrepancies or introduce earlier differences, including in the rounding case.

No pacemaker variant was adopted. A fix must explain the timing and persistence
of pacemaker selection across section boundaries while retaining the established
regression cases.

## Rounding hypotheses tested

Two further isolated arithmetic variants changed the movement correction:

- Multiply before dividing in the wider correction calculation.
- Retain the wider corrected movement through the addition to position.

Neither resolves the remaining rounding case. The wider addition introduces
earlier position differences in all five focused cases. These probes also
remain outside the validated engine.

## Next useful evidence

For the startup group, continue retaining complete original start/end packets
and preceding Career state. Keep the captured seed and any diagnostic inferred
seed in separate fields with explicit provenance. Recovered-seed runs can help
investigate mechanics, but cannot establish predictive parity.

For pacing, prioritize multiple Front Runners exchanging the lead while Pace
Chasers have active PaceDown or PaceUp effects. The three Career examples on
course 10501 concentrate the useful observation window around 7.5–9.6 seconds.
Retain positive controls and the existing restoration-after-selection-window
fixture when testing any proposed rule.

The practice collection check found **no appended completed manifest rows**
relative to the prior frozen snapshot. This was a manifest check, not a new
Career inventory or an additional full-corpus audit.

## Evidence and scope

The [follow-up evidence](../evidence/unresolved-followup-20261020.json) links the
per-case inferred-seed checks, raw-packet cross-checks, private probe builds,
regression failure, source snapshots, and reproduction scripts. Detailed local
artifacts are under:

`<race-dataset>/discrepancies-20261020-followup`

The validated production Rust source and engine are unchanged by this follow-up.
The prior full-batch validation remains applicable to that unchanged build.
No inferred seeds were promoted into the canonical corpus or production code.
