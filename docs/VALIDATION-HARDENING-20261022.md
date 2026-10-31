# Capture validation and reproducibility

> Historical report: counts, open issues and test results below describe
> the stated audit and its follow-ups. See [current accuracy](ACCURACY.md)
> and [testing](TESTING.md) for the committed engine.

The subsequent [full corpus audit](FULL-VALIDATION-20261022.md) applies these
checks to all 435,216 registered races, with active and quarantined results
reported separately. The smaller panel results below remain their original
coverage and verification results.

Validation now checks capture identity, internal output consistency and process
repeatability alongside strict agreement with official observations. These checks
improve defect detection and investigation; they do not establish universal game
parity or interpret replay fields whose meaning remains unknown.

## Added checks

| Area | Validation |
| --- | --- |
| Capture inputs | Seed, course, field size and mode agree with the frozen job; gate identities form a complete roster; player index and numerical inputs are valid; equipped skills have definitions; replay and input runner counts agree |
| Source integrity | The capture panel verifies reference, replay, master database and skill projection hashes; where supplied, provenance hashes and seed/reference/replay/master identities must agree |
| Output consistency | Finish ranks form a permutation and agree with raw finish-time ordering; echoed seed/player identity matches the input; reported branch indices exist in the supplied definitions |
| Cast accounting | Detail and target streams contain the same cast identities, including branch indices and duplicate multiplicity |
| Repeatability | Original warm worker, three fresh binary processes with changed batch order, repeated inputs, ordinary JSON output and JSON replay-worker output |
| Failure handling | Per-request deadlines cover writes and reads; failed workers are killed and reaped before another request starts; capture-panel failures save full inputs, expected observations and available binary output |
| Collection planning | Unobserved branch entries include the exact condition, precondition, effect types and definition hash |

Cast accounting is an internal consistency check. It does not replace the
independent comparison against recorded branch and visual-recipient observations.
Finish ties are allowed; the validator does not invent a different tie-break rule.
Input preflight validates the normalized capture contract, not every possible
public engine API request. Consistent hashes cannot establish that a server capture
faithfully represents its stated seed.

## Verification

| Check | Result |
| --- | --- |
| Captured regression fixtures | 78/78 pass |
| Latest-intake stratified panel | 265/265 pass: 209 practice and 56 Career |
| Coverage | 105 courses; field sizes 9–18 |
| Dense output integrity | 344,945 frames; 4,951,577 runner samples |
| Official samples compared | 533,819, including 83,818 post-finish samples |
| Repeatability | 15 selected inputs; 15 warm checks and 80 fresh-process/output-mode checks pass |
| Focused Python tests | 76 pass across strict comparison, coverage, worker failure controls, protocol, capture intake, reference decoding and quantization |

Negative controls include changed manifest identity, conflicting provenance with
otherwise valid hashes, duplicate gates/ranks/casts, nonexistent skill branches,
nonfinite input values, a hung worker, a crashing worker, malformed replies and an
intentional JSON replay-worker divergence. Failure-bundle tests verify that full
input definitions and output bytes survive compression unchanged.

The panel selects one active capture per corpus/runner-count/course stratum from
the frozen 22 October intake. The explicit quarantine registry excludes its 799
records; this is a panel audit, not a rerun of the full corpus. The engine binary
is checked for changes during the audit. All changes are in validation tooling;
race physics and the simulation hot path are unchanged.

[Panel report](../evidence/validation-hardening-20261022.json) and
[fixture report](../evidence/validation-hardening-20261022-fixtures.json) contain
the results. The panel observes 548 of 607 equipped branch versions. The 59
unobserved versions are listed with their conditions in
[collection targets](../evidence/validation-hardening-20261022-collection-targets.json).
These are gaps in this selection, not proof of missing simulator behavior.

## Use

Run the captured fixtures from this repository:

```powershell
python tools/check_accuracy_regressions.py --engine path/to/engine.exe --timeout 60 --report accuracy.json
python tools/audit_validation_coverage.py --report coverage.json
python -m unittest discover -s tests -p "test_validation*.py"
```

From the capture repository, run the broader panel against frozen manifests:

```powershell
python tools/audit_validation_layer.py --sim-repo ../GlobalSim --engine path/to/engine.exe --practice path/to/practice/manifest.json --career path/to/career/manifest.json --quarantine .local/race-datasets/quarantine.json --repeat-rounds 3 --timeout 60 --output path/to/new-audit-directory
```

The output directory must be new. `results.jsonl` distinguishes input-integrity,
engine-execution and output-validation failures. `failures/<replay hash>/` retains
full reproduction bundles, and `worker.log` retains engine diagnostics.
`repeatability/` preserves the selected inputs and dense binary baselines even
when all probes pass, plus each process log and any differing replies. A compressed
input contains one complete JSON-line request, ready to replay after decompression.
Private capture inputs remain in the dataset, not in the public evidence report.

The regular Career auditor and fixture checker also apply capture preflight and
the input-aware output checks. Deadlines and automatic worker recovery are
implemented in the capture-panel and fixture runners; historical frozen scripts
retain their original behavior.

## Remaining limits

Repeated agreement is not proof that nondeterminism is impossible. JSON/binary
agreement is internal output agreement, not an additional official-game audit.
Branch activation observations do not measure both outcomes of every condition.
The global order across separate event streams, activation flags, presentation
finish fields, additional ability recipients and official frame-selection cadence
retain the limitations described in the [coverage audit](VALIDATION-AUDIT-20261022.md).
No unknown field is counted as validated merely because it was retained.
