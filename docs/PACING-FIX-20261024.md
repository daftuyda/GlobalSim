# Pacemaker retention correction — 24 October 2026

> Historical report: counts, open issues and test results below describe
> the stated audit and its follow-ups. See [current accuracy](ACCURACY.md)
> and [testing](TESTING.md) for the committed engine.

The 16-runner practice capture on course 10501 with seed 1717357352 now matches
all checked official observations. The second capture from the same intake,
on course 10611 with seed 580983021, remains an active numerical discrepancy.

## Cause and correction

A section-boundary branch replaced a handed-over pacemaker with the leader of
its own running-style group when a forward-style runner had passed the incumbent.
That replacement could put the new incumbent ahead of the returning Front Runner,
preventing the ordinary restoration pass from restoring the forward group.

In the failing race, gate 16 is the incumbent. At 7.925393105 seconds, the previous
ranking is headed by gates 6, 12 and 16. Gate 12 is a Front Runner; gates 6 and 16
are Pace Chasers. The old branch switches the incumbent from gate 16 to gate 6.
The returning gate 12 is then behind that replacement, delaying restoration until
8.990990639 seconds.

The correction retains gate 16 through the section-selection pass. On the next
tick, at 7.991992950 seconds, the existing restoration rule selects gate 12, the
nearest forward-style runner ahead of the incumbent. Ordinary section selection,
range checks, counters and the restoration order remain in place.

The old selection makes gate 4 leave Pace Down at 8.524791718 seconds: its live
gap to gate 6 is 3.173172 m, exceeding the stored 3.1589794 m exit threshold.
That is the first recorded speed and position discrepancy. Retaining the correct
incumbent restores the complete captured race, including later skills, stamina,
positions and finish results. No input, seed, gate, course or time exception was added.

Three diagnostic alternatives that immediately restore the forward group also
match the new capture, but each breaks four existing pacemaker fixtures. They
were rejected. Retaining the incumbent matches all existing fixtures, providing
a stronger distinction than fitting this single race alone.

The new permanent fixture is
`pacemaker-retains-incumbent-at-section-boundary.json.gz`. It contains the original
input and official observations and fails the preceding engine.

## Remaining movement case

Capture `b0dd63b3423947afcbfcec686944f3810c5f8e4c45c6c80b3bf6d572b7fec337`
is unchanged by the pacing correction. It has 85 sampled position discrepancies
(maximum 1.221 mm), two encoded stamina discrepancies and two final-spurt distance
discrepancies. Every raw finish time and placement matches.

The first recorded differences are gates 8 and 10 at 35.165004730 seconds; gate 7
follows at the next recorded sample. Gate 8 is briefly blocked by gate 11 at
34.698799133 seconds, and gate 10 subsequently observes gate 8 as its blocker.
This is a useful investigation window, not proof that the blocking formula is wrong.

The diagnostic screen covered geometry arithmetic, lateral movement arithmetic,
blocking-cap arithmetic, single-tick lane-direction changes across all 16 runners,
and controlled speed, position and hidden-lane perturbations. No tested general arithmetic alternative
restored every checked observation. Some arbitrary perturbations reduce mismatch
counts, but they do not establish a general mechanic and were not adopted.

Higher-precision reconstructions also checked the original geometry and lateral
movement intermediates. They distinguish occasional earlier world-coordinate
rounding differences, but do not establish the source of this capture's drift.

Further official captures should repeat the full saved lobby, particularly gates
7, 8, 10 and 11 on course 10611 (1,600 m dirt, good ground, rainy weather). Preserve
raw observations around 30–37 seconds and 600–730 m, spanning lane decisions,
corner entry and the brief blocking interactions. Exact positions, stamina and
spurt distances are required; finish-only checks would pass this case.

## Movement follow-up

A further causal screen tested 2,496 single-step position interventions across
gates 7, 8, 10 and 11 between 30 and 35.165 seconds. Two interventions restored
all checked observations: reduce gate 8's committed position by four float32
steps (0.244140625 mm), once at either 34.232593536 or 34.365795135 seconds.
Both reproduced identically in five fresh processes; the unmodified diagnostic
engine also reproduced the original failure in five fresh processes.

These are artificial interventions, not fixes or proof of the official hidden
trajectory. They show that one small upstream movement difference can account
for the later position, stamina and spurt-distance discrepancies. The official
recording does not expose every intervening simulation tick.

Another 1,024 perturbations of gate 11's final lane movement did not improve the
baseline. Nine general geometry-arithmetic variants (sample calculation order,
intermediate precision, fused quaternion interpolation and final coordinate
rounding) failed to restore the capture. None was adopted. The remaining case
stays active, with unchanged inputs and strict tolerances.

[Diagnostic evidence](../evidence/movement-causal-followup-20261024.json).

A second follow-up tested five speed-update/modifier rounding variants and four
acceleration variants. None passed: three speed variants and all four acceleration
variants retained the baseline output; the other two speed variants increased
position mismatches to 118. Another 1,848 earlier lane interventions had a best
result of 18 position mismatches. Of 528 stored-world-coordinate interventions,
the best left two position mismatches (gate 7 at 37.296230316 and 38.361843109
seconds), with other checked observations matching. These remain diagnostic
interventions, not a general correction. A limited exact-arithmetic comparison
of 53-bit and 64-bit significands found no differences in 2,000 integer-power
lateral base rates or the screened nearby-power lane factors.

[Second diagnostic evidence](../evidence/movement-causal-followup-2-20261024.json).
Production behavior, the active failure and strict validation remain unchanged.

The third follow-up found no general correction among eight interpolation
alternatives, 1,024 combinations of ten geometry precision choices, three
acceleration-stacking alternatives, 12,227 one-tick movement-permission probes
across all 16 runners, 1,188 corner-window hidden-lane probes, and 994 hidden-speed
probes. The best speed intervention leaves one earlier position mismatch; it is
still artificial and is not a fix. One- and two-step float speed interventions
in the screened 30–34.7 second window did not improve the 85-position baseline.

The installed course's animation and overrun curves exactly match the bundled
capture. Its event entries match after disregarding their listing order. This
checks installed assets, not inaccessible server state. Inspection of the raw
reference confirmed 136 recorded frames and no populated extension sections
containing additional per-tick observations. Reconstructed managed 32/64-bit
geometry programs were also measured; they are not official game evaluators
and supplied no validated engine correction.

The latest isolated diagnostic build reproduces the original output byte for
byte in five fresh processes. Production source and the active failure remain
unchanged by these experiments. A trusted full-tick reference for the exact
saved lobby and seed would allow comparison of the currently hidden state.

[Third diagnostic evidence](../evidence/movement-causal-followup-3-20261024.json).

## Validation

- 160 Rust library tests pass.
- All 82 captured strict regression fixtures pass.
- All 105 final-build worker repeatability checks pass across the pacemaker
  fixture panel. An earlier 35-check panel also included the unresolved capture.
- The entire 66,738-race intake was re-simulated: 66,737 full outputs are identical
  to the audited baseline, and the one changed output now passes strict validation.
  There are 66,737 strict matches, one remaining discrepancy and no regressions
  or execution errors.

A broader reference-path-ordered prefix check reproduced 145,920 older practice
outputs byte for byte, with no regressions or execution errors. That check used
the earlier build of the same correction; the final rebuild only follows comment
line-ending normalization. The final executable was independently checked on the
entire new intake, and all 66,738 outputs match that earlier correction build.
The historical run was bounded after a completed batch: 324,430 earlier captures
were not rerun with either fix build. This is not a complete historical corpus audit.
Career intake and existing quarantine decisions remain unchanged.

[Validation evidence](../evidence/practice-fix-20261024.json) records the exact
executable identities, comparison scopes and diagnostic results.
Agreement on recorded observations is not proof of universal game parity.
