# GlobalSim documentation

The guides describe v77, the official-priority default engine. Start with the
[project README](../README.md)
for setup and features. Accuracy measurements apply to their stated datasets
and engine configurations.

## Guides

| Guide | Contents |
|---|---|
| [Complete race mechanics](RACE-MECHANICS.md) | Consolidated v77 calculations and race behaviour; [Word edition](GlobalSim-Race-Mechanics-v77.docx) |
| [Interface](INTERFACE.md) | JSON requests, seed sampling and result fields |
| [In Process](IN-PROCESS.md) | Prepared Rust, C and Python integration |
| [Binary Replay](BINARY-REPLAY.md) | Lossless binary replay protocol |
| [Mathematics](MATHEMATICS.md) | Stats, speed, acceleration, stamina, spurt and precision |
| [Field Mechanics](FIELD-MECHANICS.md) | Pacing, passing, blocking, ranking and competition |
| [Skills](SKILLS.md) | Conditions, lotteries, effects, recipients and timing |
| [Architecture](ARCHITECTURE.md) | Data preparation, execution order and independent seed state |
| [Parameters](PARAMETERS.md) | Loaded parameters and selected source constants |
| [Source Index](SOURCE-INDEX.md) | Production modules, tools and data |
| [Accuracy](ACCURACY.md) | Current validation, inventory and known limits |
| [Validation summary](VALIDATION-SUMMARY-20261103.md) | All current test scopes, latest findings and unresolved cases |
| [Testing](TESTING.md) | Build, fixture and integration checks |
| [Experimental profile](EXPERIMENTAL.md) | Optional uma.moe comparison rules and validation tradeoffs |
| [Performance](PERFORMANCE.md) | Measured release latency and benchmark scope |
| [Career Validation](CAREER-VALIDATION.md) | Career mechanics, coverage and outstanding evidence |
| [Dataset Quarantine](DATASET-QUARANTINE.md) | Selection policy and recorded quarantine scope |

The Word mechanics reference is generated from `RACE-MECHANICS.md`. To rebuild
it, install Pandoc and `python-docx`, then run
`python tools/export_mechanics_document.py` from the repository. Review the
rendered document after changing the source or exporter.

## Third-party comparison

[Experimental profile](EXPERIMENTAL.md) reports the latest paired comparisons
and official-capture differences. The
[official boundary investigation](OFFICIAL-PRIORITY-20261101.md) documents why
the default retains its duel-stamina and fractional entry rules. The
[31 October comparison](UMAMOE-MOVEMENT-20261031.md)
retains the earlier production and research measurements. Third-party agreement
is not official-game accuracy.

## Release validation

- [Version 77 validation](V77-VALIDATION.md)

## Historical reports

These reports retain the findings, test counts and open issues of their
stated audit periods. Later fixes do not rewrite their original measurements.
Use [Accuracy](ACCURACY.md) for current status. Reproduction bundles are
maintained in the separate race dataset; machine-readable audit summaries
are linked from each report.

- [Original v76 release validation](V76-VALIDATION.md)
- [Earlier performance investigation](PERFORMANCE-AUDIT.md)
- [Career quarantine — 25 October 2026](CAREER-QUARANTINE-20261025.md)
- [Passing-corridor boundary correction](CORRIDOR-FIX-20261023.md)
- [Discrepancy investigation — 20 October 2026](DISCREPANCIES-20261020.md)
- [Investigation of the 21 October failures](DISCREPANCIES-20261021.md)
- [Full corpus validation](FULL-VALIDATION-20261022.md)
- [Practice movement investigation — 25 October 2026](MOVEMENT-INVESTIGATION-20261025.md)
- [Practice intake validation — 23 October 2026](NEW-PRACTICE-20261023.md)
- [Practice intake validation — 24 October 2026](NEW-PRACTICE-20261024.md)
- [New race findings — 18 October 2026](NEW-RACE-FINDINGS-20261018.md)
- [New race audit — 20 October 2026](NEW-RACES-20261020.md)
- [New-race validation and performance — 21 October 2026](NEW-RACES-20261021.md)
- [New race validation — 22 October 2026](NEW-RACES-20261022.md)
- [Race intake validation — 25 October 2026](NEW-RACES-20261025.md)
- [Race intake validation — 26 October 2026](NEW-RACES-20261026.md)
- [Race intake validation — 28 October 2026](NEW-RACES-20261028.md)
- [Pacemaker restoration correction — 21 October 2026](PACING-FIX-20261021.md)
- [Pacemaker retention correction — 24 October 2026](PACING-FIX-20261024.md)
- [Practice edge-case follow-up](PRACTICE-EDGE-CASES-20261018.md)
- [Practice competition recording and Career quarantine](PRACTICE-FIXES-20261022.md)
- [Unresolved race follow-up — 20 October 2026](UNRESOLVED-FOLLOWUP-20261020.md)
- [Validation coverage audit](VALIDATION-AUDIT-20261022.md)
- [Capture validation and reproducibility](VALIDATION-HARDENING-20261022.md)
