# Validation coverage audit

> Historical report: counts, open issues and test results below describe
> the stated audit and its follow-ups. See [current accuracy](ACCURACY.md)
> and [testing](TESTING.md) for the committed engine.

The later [validation hardening report](VALIDATION-HARDENING-20261022.md) adds
capture preflight, output consistency, bounded failure recovery, expanded
repeatability probes and actionable branch collection targets. The results below
remain the original panel's results.

The expanded validator found gaps in what we checked, but no new simulation
deviation in the selected capture panel. Matching captured observations remains
a narrower claim than complete game parity. A third-party claim that simulators
omit important mechanics does not identify which mechanic, version or race mode
to test.

## Checks added

| Previous gap | Current check |
| --- | --- |
| Bulk replay sampling could discard invalid states between recorded timestamps | Check every supplied engine tick for finite numeric state, runner counts, valid blocker identity and integer state values; callers explicitly declare whether they supplied dense output |
| Skill branch telemetry existed but was not compared | Compare the complete detail-index multiset and within-runner sequence against official event parameter 3 |
| Separate branch and target comparisons could hide swapped recipients on simultaneous casts | Compare branch and first-ability visual recipient mask together for each cast |
| Unknown symbolic skill IDs and competition event types could disappear through filters | Reject unsupported output event identities/types; retain the known synthetic `kakari` event with an explicit coverage limitation |
| Some official event parameters and extension data disappeared from expected observations | Retain every event parameter and extension size/hash; inventory unhandled layouts and populated extensions |
| A boolean pass could be read as comprehensive validation | Report observation agreement, output integrity, checked-frame scope and coverage gaps separately |
| Equipped skills were easy to mistake for tested skills | Inventory actual activations and observed branch indices against the equipped definition versions |

The earlier target lookup also overwrote duplicate same-skill/time casts. It now
retains multiplicity. Mutation tests exercise wrong branches, swapped recipients,
invalid states on unrecorded ticks, unknown events and malformed output.

The audit does not invent constraints on acceleration, HP consumption or lateral
motion. An unusual but finite state is not declared incorrect without an oracle.
Dense ticks have integrity checks; exact gameplay comparisons still use the
timestamps and quantization present in official captures.

## Results

| Check | Result |
| --- | --- |
| Existing captured regression fixtures | 73/73 pass |
| Stratified capture panel | 464/464 pass: 402 practice, 62 Career |
| Panel scope | 105 courses; field sizes 9 through 18 |
| Dense output checked | 617,683 frames; 8,169,370 runner samples |
| Official observations compared | 877,712 runner samples, including 135,653 post-finish samples |
| Official skill branch records compared | 58,227, including 406 nonzero detail indices |
| Process/worker repeatability probes | 16 inputs each agree on initial run, warm repeat after intervening races, and a fresh process |
| Focused Python tests | 67 pass across validator, coverage, intake, selection, decoder, quantization and binary protocol tests |

The panel selects the lowest replay hash per corpus/runner-count/course from
frozen practice inventory and the latest frozen Career intake, excluding the
existing explicit quarantine. It is a coverage-oriented selection, not a random
accuracy estimate or a full-corpus rerun. Repeatability probes do not rule out
rare nondeterminism.

[Panel results](../evidence/validation-audit-20261022-panel.json),
[per-race results](../evidence/validation-audit-20261022-panel-results.jsonl),
[regression results](../evidence/validation-audit-20261022-fixtures.json),
[fixture coverage](../evidence/validation-audit-20261022-fixture-coverage.json).

## What remains unproved

- **Skill activation flag:** official parameter 5 is retained, but the engine does
  not expose an equivalent field. All 58,227 sampled records contain zero. This
  does not establish behavior when it is nonzero.
- **Additional result fields:** scaled/presentation finish time, finish difference
  and `defeat` are retained but not compared. They must not be conflated with the
  raw finish time that is already checked.
- **Replay ordering and cadence:** separate engine streams do not expose a global
  cross-stream event order. Matching recorded timestamps does not prove that we
  reconstruct the official frame-selection schedule.
- **Other gameplay recipients:** the official visual mask describes the first
  ability. It cannot independently verify every ability's recipients or every
  later target reselection. Movement/HP observations can expose consequences,
  but do not supply the missing target oracle.
- **Branch coverage:** the panel equips 575 skill IDs and observes 541 activating.
  Of 618 equipped branch versions, 565 are observed: 53 remain unobserved. The
  smaller regression suite equips 529 IDs, with 78 never seen activating, and its
  older fixtures lack raw detail observations. Neither inventory measures
  coverage of every condition outcome or every combination of effects.
- **Optional approximate mechanics:** source inspection confirms that Stamina
  Keep, Secure Lead and Stamina Syoubu include approximations behind
  `experimentalLaterStaminaMechanics`. That flag is disabled in this panel and
  the regression fixtures. These results do not validate that optional mode or
  establish that its mechanics should be enabled for the captured game version.

The panel contains no unknown event kinds, populated extension sections or
nonzero activation flags. New occurrences will appear in the surface inventory
and coverage gaps rather than being silently discarded.

## Targeted collection priorities

The following branch indices were unobserved in this panel. Conditions below
come from the captured installed definitions, not inferred race behavior. Use
matched control lobbies and preserve the original seeds and raw replays.

| Skill / zero-based detail | Capture target |
| --- | --- |
| `202331` / 1 | Dirt races with the skill's `base_power` condition in [1000,1200); collect controls just below 1000 and at/above 1200 |
| `110151` / 1 | Reach the 399–401 m remaining window in the top 40% with at most six prior activations; pair against seven or more |
| `100491` / 1 | Final-corner overtaking prerequisite, then at most 400 m remaining and 30–50% order band, with popularity below 4; pair against popularity 4+ |
| `910221` / 1 | Enter the final-corner, phase-2+, 20–40% order condition after the earlier branch's 401 m threshold is unavailable |

Repeat rank-band cases in both 9-runner and larger Career fields. Prioritize
simultaneous activations, blocked runners and opponent effects around these
boundaries. The full missing-branch list is in the panel report; absence here
does not mean absence from the entire stored dataset.

## Running the checks

```powershell
python tools/check_accuracy_regressions.py --engine path/to/engine.exe --report accuracy.json
python tools/audit_validation_coverage.py --report coverage.json
python -m unittest discover -s tests -p test_accuracy_regressions.py
```

The capture repository's `tools/audit_validation_layer.py` accepts frozen
practice/Career manifests, an explicit quarantine manifest, the simulator repo
and engine paths, and a new output directory. It checks full binary replies and
runs the repeatability probes. The live Career auditor also uses dense checks
and retains the expanded replay surface. Historical frozen audit scripts and
their reports remain historical evidence; their sampled-frame passes are not
retroactively upgraded to full-tick checks.
