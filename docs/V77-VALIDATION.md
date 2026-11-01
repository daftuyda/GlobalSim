# Version 77 release validation

Version 77 packages the official-priority default engine with updated validation,
examples and documentation. Its simulation source, data and default build settings
are unchanged from the audited build. Experimental comparison rules remain opt-in.

## Release checks

The fresh default Windows release build passes:

| Check | Result |
|---|---:|
| Rust unit tests | 167/167 |
| Python validation tests | 39/39 |
| Official captured-race regression fixtures | 84/84 |

The official fixture checks use strict recorded observations, including ordering
and post-finish samples. The v77 example and settings preserve the validated
inputs from the preceding example.

## Official practice coverage

The existing full practice audit matched **696,777/696,778 active official
captures**. One movement discrepancy remains; its finish times and placements
match. That result is carried forward from the audit of the unchanged engine;
the full corpus was not rerun for this version and documentation update.

The optional full uma.moe profile has known official regressions and is not the
default. See [accuracy and coverage](ACCURACY.md) for dataset scope and limits,
and [testing](TESTING.md) for reproduction commands. Historical performance
measurements retain their original version labels.

## Evidence

- [Release checks and build identity](../evidence/v77-release-validation.json)
- [Official fixture results](../evidence/v77-accuracy.json)
- [Fixture coverage](../evidence/v77-coverage.json)
- [Full official practice audit](../evidence/official-practice-validation-20261101.json)
