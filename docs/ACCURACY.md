# Benchmark results and coverage

GlobalSim v71 was evaluated against **13,187 official Global races**, covering
125,215 runner observations. Every finish time matched exactly in **99.97%**
of races, with **99.98% winner agreement** and **99.98% exact placement agreement**.

## Results

| Metric | Result |
|---|---:|
| Races with every finish time exact | 13,183/13,187 (99.9697%) |
| Winner agreement | 13,185/13,187 (99.9848%) |
| Exact placement agreement | 125,196/125,215 (99.9848%) |
| Races with every compared frame sample exact | 13,183/13,187 (99.9697%) |
| Races with all skill starts exact | 13,183/13,187 (99.9697%) |
| Matched skill starts | 1,376,407/1,376,523 (99.9916%) |
| Mean absolute finish-time error | 0.0593 ms |
| Worst observed finish-time error | 1.045456 s |

Time errors are weighted by runner count. The benchmark contains 12,405
nine-runner races and 782 races with 12, 14, 16 or 18 runners. Coverage is not
uniform across course, roster or skill combinations. Custom oversized fields
are outside this benchmark. [Numeric results](../evidence/v71-summary.json).

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
or a guarantee for every race configuration. The benchmark includes four races
with finish-time differences and two with a different winner.

A small number of pacing and movement differences remain. Conserve Power fallbacks and optional later
stamina mechanics include approximations. Results also depend on the selected
runtime settings and course and skill data.

GlobalSim is an unofficial implementation and does not claim complete 1:1
equivalence with the game. See [testing](TESTING.md) for the current test status.
