# Pacemaker restoration correction — 21 October 2026

> Historical report: counts, open issues and test results below describe
> the stated audit and its follow-ups. See [current accuracy](ACCURACY.md)
> and [testing](TESTING.md) for the committed engine.

The practice race on course 10801 with captured seed 636656617 now matches
every strict replay check. The same correction also resolves the older practice
pacing case, both older Career pacing cases, and the later pacing discrepancy
in one quarantined Career replay when run with its diagnostic inferred seed.
That inferred-seed result remains diagnostic and is not a canonical pass.

## Cause and correction

When the forward running-style group regained the pacemaker role, the simulator
selected that group's leader. The corrected restoration selects the nearest
runner of that group ahead of the incumbent in the preceding ranking.
Section boundaries, handover eligibility and ordinary pacemaker selection are
unchanged.

In the new practice case, gate 4 had become the incumbent. Both gates 6 and 11
subsequently passed it. The old restoration selected gate 11; the corrected
restoration selects gate 6, the nearer returning runner in the ranking.

At gate 9's pace check at 12.320982933 s, gate 6 has already moved this tick,
whereas gate 11 has not: movement runs in gate order. Consequently, selecting
gate 6 gives a live gap of **3.5130768 m**; selecting gate 11 gives
**2.4424133 m**. These lie on opposite sides of the Senkou pace-down threshold
of 3 m. The old choice triggers PaceDown, reduces its pacing coefficient to
0.945, and consumes an extra random draw. That changes subsequent movement,
skills and results. The correction avoids both the slowdown and the extra draw.

A private counterfactual first isolated the cause: suppressing that one entry
and its random draw makes the entire race match; suppressing the movement effect
while retaining the draw still fails later. Neither counterfactual is present
in production. The production change contains no seed, course, gate, or time
exceptions. The selection rule is supported by captured-race regressions;
the retained older client binary does not independently establish the current
server's implementation.

## Validation

The clean candidate passes **149/149 Rust library tests**, **73/73 strict
regression fixtures**, and **42,283/42,283 races in the new practice batch**.
Four official captures were added as regression fixtures: the new practice
case, the older practice pacing case, and both Career pacing cases. All retain
their original captured seeds and expected official observations.

The broader Career check covers all **10,261 non-quarantined Career captures**
in the frozen registry: **9,851 match**, while **410 older cases still fail**.
All 410 also fail on the pre-fix optimized engine using identical inputs; 409
produce byte-identical output. None is a new passing-to-failing regression.
These older failures are separate from the 167 newly quarantined captures.

Historical practice checks also pass: **21,422/21,422** completed captures from
the previous batch and **2,100/2,100 nine-runner races**, selected as 20 replay
hashes per course across 105 courses. The 21,422 cases are a prefix sample of
the older 90,283-race batch; that batch was not rerun in full. Retained results
and artifacts are recorded in the
[validation evidence](../evidence/pacing-fix-20261021.json). The previous rounding
case remains unresolved; this change addresses pacemaker restoration.

## Career quarantine

The requested **167 Career captures** were added to the reversible quarantine,
bringing it from 492 to **659 races**. Original captures, seeds, provenance and
audit results remain intact. Each new quarantined case includes its original
simulation input and seed-inference diagnostic. The combined manifest retains
the previous quarantine's records and points to their preserved copies.

Quarantine location:
`<race-dataset>/quarantine/captured-seed-replay-mismatch-20261021`.
Normal Career rechecks exclude all 659 IDs; `--include-quarantined` includes
them with their original seeds. Both selection paths were verified. The frozen
corpus contains 386,895 captures, of which **386,236 are active**; these are
snapshot counts, not a new live-intake count.

For the latest 43,982-race batch, 167 are now quarantined and **all 43,815 active
races match** with the correction. This is a scoped observed result, not a claim
that every historical capture or every possible race matches.
