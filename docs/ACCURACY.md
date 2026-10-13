# Benchmark results and coverage

GlobalSim v72 was evaluated against **13,187 official Global races**, covering
125,215 runner observations. Every tested race matches the recorded finish
times, winner, placements, skill starts and compared replay samples.

## Results

| Metric | Result |
|---|---:|
| Races with every finish time exact | 13,187/13,187 (100% in this corpus) |
| Winner agreement | 13,187/13,187 |
| Exact placement agreement | 125,215/125,215 |
| Races with every compared frame sample exact | 13,187/13,187 |
| Races with all skill starts exact | 13,187/13,187 |
| Matched skill starts | 1,376,523/1,376,523 |
| Mean absolute finish-time error | 0 ms |
| Worst observed finish-time error | 0 ms |

Time errors are weighted by runner count. The benchmark contains 12,405
nine-runner races and 782 races with 12, 14, 16 or 18 runners. Coverage is not
uniform across course, roster or skill combinations. Custom oversized fields
are outside this benchmark. [Numeric results](../evidence/accuracy-validation.json).

The latest corrections address pacemaker restoration, the publication timing
of healer history, and transitions at shared slope boundaries. They resolve
the four previously differing races. The other 13,183 races retain identical
serialized output, with no execution errors or regressions in the full check.

## What exact means

- `finishExact`: the entire vector of decoded official raw finish times equals
  the simulated `finishTimeRaw` vector, without a tolerance.
- Winner: simulated finish-time ordering selects the official winner.
- Placement: corresponding identities in sorted official and simulated orders
  match; denominator is runners.
- Skill starts: multiset of runner, skill ID and timestamp rounded to four
  decimal places (0.1 ms). `skillExact` compares the full multiset; matched-start
  recall alone does not measure recipient, value or duration correctness.
- Frame exactness: official replay sample times matched to simulation times
  rounded to five decimals; positions compared as float32; speed, HP and lane
  compared through the audit's wide wire quantization (scales 100,1,10000).
  Samples after either official or simulated finish are excluded. It is not
  proof of all hidden state or all unsampled ticks matching.

## Coverage and limitations

These are retrospective measurements, not an independent held-out evaluation
or a guarantee for every race configuration. The same corpus informed the
corrections, so complete agreement here does not establish accuracy on unseen
courses, rosters or skill combinations.

Conserve Power fallbacks and optional later stamina mechanics include
approximations. Results also depend on the selected
runtime settings and course and skill data.

GlobalSim is an unofficial implementation and does not claim complete 1:1
equivalence with the game. See [testing](TESTING.md) for the current test status.
