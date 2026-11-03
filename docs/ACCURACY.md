# Accuracy and coverage

GlobalSim is an unofficial implementation of Umamusume race mechanics.
Official race captures are its primary accuracy reference. Validation measures
agreement with recorded observations in specific configurations; GlobalSim does
not claim universal 1:1 equivalence or parity with every official race.

## Validated configuration

The measurements below apply to the default v77 build using the runtime
settings and skill data supplied by the [v77 example](../examples/v77-race.jsonl).
Different input data, runtime settings or optional Cargo features can change
results.

| Check | Recorded result |
|---|---:|
| Official practice corpus | 696,777 / 696,778 races match all checked observations |
| Observed corpus agreement | 99.99986% |
| Included official regression fixtures | 84 / 84 pass |
| Rust unit tests | 167 / 167 pass |
| Python validation tests | 39 / 39 pass |

The corpus measurement covers the frozen 28 October 2026 inventory, audited on
1 November. It is a validation snapshot, not a live count of collected races.
The fixture and unit-test results cover smaller, distinct checks; they are not
additional races to add to the corpus total. The corpus result includes an
unresolved mismatch.

## What is compared

- **Results:** raw finish times, placements and exposed result fields.
- **Movement:** position, speed, stamina, lane and exposed state at recorded
  reference samples, including available post-finish observations.
- **Skills:** identities, activation branches, timing, ordering, durations and
  exposed recipient masks.
- **Competition events:** recorded event identities, times and stream order.
- **Output integrity:** valid identities, finite numbers, consistent structure
  and temporal ordering throughout the simulator's output.

Strict comparisons use the precision represented in each reference channel.
Official replay samples do not expose every simulation tick or hidden decision.
Running a full-frame local simulation does not fill gaps in the reference data.
An exposed visual skill mask also does not establish every secondary gameplay
recipient.

The corpus audit executed complete simulations. A previous strict result was
reused only when the capture, input and complete output were unchanged; new or
changed outputs received a fresh comparison. Complete-output equality establishes
repeatability, while comparison with official observations establishes agreement
on the recorded channels.

## Coverage and limitations

The corpus covers collected courses, runner builds and field sizes. It includes
cases used to develop corrections and is not an independent random sample of
every possible race. A high observed match rate does not guarantee an unseen
configuration will match.

Nonzero team-rank bonuses have targeted implementation tests and limited official
capture coverage. Optional later stamina mechanics and some Conserve Power
fallback paths contain approximations. Custom oversized fields are outside the
documented official validation scope.

Third-party simulators provide useful comparison data but are not authoritative
references. Their agreement is reported separately, and a conflict with official
captures must not be treated as an improvement to official accuracy. Optional
[comparison features](EXPERIMENTAL.md) can change established behavior and have
separate validation requirements.

## Reproducing checks

The repository includes regression fixtures, strict comparison tools and
repeatability checks for the worker and prepared API. See [Testing](TESTING.md)
for commands and [the mechanics reference](RACE-MECHANICS.md) for the modeled
calculations and precision rules.

Behavior changes require comparisons against the same inputs, data and reference
observations. Missing coverage is not a pass, and an unchanged failing result
remains a failure. Targeted tests should be followed by a broader regression run
before claiming an accuracy improvement.
