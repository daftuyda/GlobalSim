# Official practice validation — 1 November 2026

Official captures rule out two proposed changes that improved agreement with
uma.moe. The default engine retains its duel-stamina calculation and fractional
lead-competition entry cutoff. No physics change, tolerance change or quarantine
change is justified by these comparisons.

## Complete official practice audit

The frozen 28 October inventory contains 696,779 practice captures. The default
engine matches recorded observations in **696,777 of 696,778 active races**
(99.99986%). The separately quarantined capture also matches but remains
quarantined. Career races were not rerun.

Both engines executed every race with full simulation frames. A previous strict
pass was reused only for an identical capture, projected input and complete
output; failed or changed outputs received fresh strict checks. The previously
unresolved movement case reproduces with its unchanged complete-output hash.
This method preserves strict verification without claiming that every old
comparison was recomputed or that hidden official frames were observed.

The whole-metre-cutoff plus logarithm candidate matches **696,775/696,778**
active captures. It fixes no official case and produces two new failures: the
reproducible entry-cutoff regression and the retained one-off timing mismatch.
Its complete output agrees with the default on 696,777 of the 696,779 registered
practice captures. The isolated duel-percentage regression was established in a
separate component check, not this two-component audit.

## Focused regression checks

Two official captures extend the passing regression suite from 82 to 84 cases.
The separately retained unresolved movement case is not included in that passing
suite.

| Build | Official regression fixtures |
|---|---:|
| Default | 84/84 |
| Inward-gap refresh alone | 74/84 |
| Pacemaker staging alone | 83/84 |
| Participant cutoff alone | 83/84 |
| Duel percentage alone | 83/84 |
| Whole-metre entry cutoff alone | 83/84 |
| Pacing logarithm alone | 84/84 |
| Whole-metre entry cutoff plus pacing logarithm | 83/84 |
| Complete follow-up profile | 72/84 |

Passing the focused screen does not establish whole-corpus non-regression.
In particular, the isolated logarithm experiment has only a partial official
screen of 163,912 passing races. It remains disabled by default.

## Duel stamina boundary

The official Kyoto 1,800 m capture, seed `78993715`, matches the default engine.
The percentage experiment keeps one participant in a duel for an extra tick.
At the local decision boundary, remaining HP is `112.32778930664062` and maximum
HP is `2246.555908203125`. Their double-precision ratio is below 0.05, whereas
the float32 percentage rounds to exactly 5.

The resulting raw finish time differs by `0.00096893310546875` seconds. There
are also 37 position, five speed and one HP differences at recorded observations.
Official HP is quantized; this evidence supports the resulting race behavior,
not a claim that every internal HP bit or the official implementation is known.

A fresh uma.moe replay follows the percentage experiment at this boundary.
It matches the checked earlier observations, but the browser cannot preserve a
popularity-dependent skill input. It is therefore a context-limited comparison,
not an interchangeable official reference. Whole-second telemetry alone misses
the short difference; the detailed replay exposes it.

## Fractional competition-entry boundary

The official Fukushima 1,150 m capture, seed `556894197`, has 12 runners.
Two participants enter lead competition at `14.518777847290039` seconds.
One participant is at `287.1053466796875` m, before the default `287.5` m cutoff
but beyond the experimental `287` m cutoff.

Flooring the cutoff removes both entry events. It changes all 12 raw finish
times, six placements, skills and many recorded movement observations. The
default and isolated logarithm builds pass; the isolated floor and combined
floor-plus-logarithm builds fail.

The new browser replay cannot isolate that cutoff. It already diverges from the
default at `3.19680118560791` seconds. The complete uma.moe follow-up profile
matches its report and recorded frames, while failing the official capture.
There is also a later input limitation: the browser omits career wins that
scale two equipped skill effects. The local original and projected inputs
agree through 30 seconds and first diverge at about 38.36 seconds. Neither the
early movement conflict nor the later projection limitation should be concealed
by treating the browser race as an exact official reproduction.

The collection projection now checks career-win-dependent effect values as
well as condition tokens. A skill can depend on career wins without mentioning
them in its activation conditions.

## Remaining movement discrepancy

The Tokyo dirt 1,600 m capture, seed `580983021`, remains an active unresolved
case. It has 16 runners, 85 differing sampled positions with a maximum difference
of approximately 1.221 mm, two HP differences and two spurt-distance differences.
Raw finish times and placements match. The first recorded movement difference
is around 35.165 seconds.

The reproducible input and official observations are included under
`tests/fixtures/unresolved`. Running strict validation against that directory
is expected to fail. The case has not been quarantined or reclassified as a
match. Fresh component and geometry-arithmetic checks did not produce a supported
correction.

## Repeatability and audit limitations

The paired official audit also retained a one-off event-timing mismatch in the
candidate for course `10707`, seed `1360631271`. Twelve subsequent runs across
four profiles produced identical passing outputs. The original failing output
remains evidence; successful repeats neither erase it nor establish its cause.

The secondary 400,040-input uma.moe audit did not complete cleanly. Its original
run hit a disagreement between fast and canonical telemetry comparisons. A
continuation encountered another such disagreement and a Python access
violation. A separate partial official experiment also suffered a Python
allocator abort. These are unresolved execution or validation incidents; they
are not evidence that the underlying captures are invalid.

One saved packet from the secondary comparison passed 100 checks in a fresh
process without loading the simulator library. That limits the reproduction,
but does not resolve the earlier error. Partial counts and successful follow-ups
must not be advertised as an uninterrupted complete audit.

The missing checks subsequently completed with external worker processes. The
two identifiable checker-error cases remain excluded from the qualified totals,
even though their follow-up checks match:

| Secondary panel | Default | Cutoff plus logarithm |
|---|---:|---:|
| Sampled exports, excluding two inconclusive cases | 354,484/399,998 | 354,487/399,998 |
| Targeted boundary probes | 3/40 | 13/40 |

The candidate improves three sampled comparisons and ten targeted probes, with
no regression in this qualified secondary panel. These improvements do not
offset the failures against official captures. The original errors and the
process abort remain unresolved; these are not grounds for quarantining races.

The [machine-readable record](../evidence/official-practice-validation-20261101.json)
binds the full audit, fixture screens and follow-up evidence. The
[accuracy guide](ACCURACY.md) reports the whole-practice scope and
keeps official results separate from third-party agreement. The
[experimental guide](EXPERIMENTAL.md) retains historical measurements with their
original denominators.
