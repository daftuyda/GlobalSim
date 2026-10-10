# Benchmark results and coverage

GlobalSim was evaluated against **13,187 official Global races**, covering
125,215 runner observations. Every finish time matched exactly in **99.76%**
of races, with **99.92% winner agreement** and **99.91% exact placement agreement**.

## Results

| Metric | Collected corpus | Original corpus | Combined |
|---|---:|---:|---:|
| Races | 12,420 | 767 | 13,187 |
| Runner observations | 112,065 | 13,150 | 125,215 |
| All finish times exact, per race | 12,396/12,420 (99.8068%) | 760/767 (99.0874%) | 13,156/13,187 (99.7649%) |
| Winner agreement | 12,412/12,420 (99.9356%) | 765/767 (99.7392%) | 13,177/13,187 (99.9242%) |
| Exact placement agreement | 111,995/112,065 (99.9375%) | 13,105/13,150 (99.6578%) | 125,100/125,215 (99.9082%) |
| All audited frame samples exact, per race | 12,396/12,420 (99.8068%) | 760/767 (99.0874%) | 13,156/13,187 (99.7649%) |
| All skill starts exact, per race | 12,400/12,420 (99.8390%) | 763/767 (99.4785%) | 13,163/13,187 (99.8180%) |
| Matched skill starts | 1,257,899/1,258,749 (99.9325%) | 117,572/117,774 (99.8285%) | 1,375,471/1,376,523 (99.9236%) |
| Mean absolute finish error | 0.000321828 s | 0.000950089 s | 0.000387808 s |
| Worst finish error | 1.308906555 s | 0.851173401 s | 1.308906555 s |

Combined means are runner-weighted, not the average of two corpus means.
See [combined numeric totals](../evidence/v70-combined-summary.json).
The collected corpus is overwhelmingly nine-runner racing (12,387/12,420).
These results do not imply equivalent coverage across every course, roster,
skill combination, career mode or oversized field.

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
or a guarantee for every race configuration. The benchmark includes 31 races
with finish-time differences and ten with a different winner.

Remaining differences include pacemaker selection in fields without Front
Runners and duel-entry timing. Conserve Power fallbacks and optional later
stamina mechanics include approximations. Results also depend on the selected
runtime settings and course and skill data.

GlobalSim is an unofficial implementation and does not claim complete 1:1
equivalence with the game. See [testing](TESTING.md) for the current test status.
