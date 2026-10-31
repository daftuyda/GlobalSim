# Investigation of the 21 October failures

> Historical report: counts, open issues and test results below describe
> the stated audit and its follow-ups. See [current accuracy](ACCURACY.md)
> and [testing](TESTING.md) for the committed engine.

Follow-up: the [pacemaker restoration correction](PACING-FIX-20261021.md)
resolves the practice case and four older pacing examples. The 167 Career
captures have now been quarantined. The findings below describe the initial
investigation; its evidence and original audit counts are retained.

The 168 failures in the latest 43,982-race batch separate into **167 Career
captured-seed/replay mismatches** and **one unresolved practice pacing case**.
Both groups already fail on released v75 with byte-identical output to the
optimized audited build. This investigation does not change the original
43,814 strict matches, canonical seeds, or quarantine membership.

## Career: 167 captured seeds do not reproduce their replays

For each failing Career replay, invert the observed unmodified start delays
through the engine's random-number generator and test the resulting candidate
against the entire replay. The search found exactly one seed magnitude per
case within the tested draw-offset range, and it differs from the captured seed
in **167/167** cases. Using that candidate with otherwise unchanged inputs and
the unchanged audited engine gives **167/167 strict diagnostic matches**.

As controls, 32 passing Career races recover their original seed magnitudes and
remain strict matches. These are full strict checks of exposed observations,
not just matches of start delays or finish order. Seed signs are indistinguishable
under the generator's absolute-value initialization; the inference reports
magnitudes. The search covers the first 512 possible delay-block offsets and
uses at least six gates unaffected by start-delay skills, then verifies all
eligible gates before running the complete simulation.

An independent check of the raw packets establishes:

| Check on the 167 failing captures | Result |
| --- | ---: |
| Captured seed agrees with raw race-start response | 167/167 |
| Inferred seed appears as an integer in start/end responses | 0/167 |
| End-response player rank agrees with replay | 167/167 |
| End-response scaled player finish time agrees with replay | 166/167 |

The remaining finish-time comparison differs by one scaled unit, 0.0001 s.
All 32 controls agree on the captured seed, rank and scaled finish time.

This strongly identifies the same seed/replay mismatch family as the earlier
quarantined group. It does **not** establish why the official response carries
a different seed, prove a server reseeding mechanism, or show that the replay
itself is corrupt. Inferring a seed from the completed replay is a diagnostic
tool, not a predictive simulator fix. Keep the original captures intact and
these cases out of claims of observed agreement under captured inputs.

## Practice: an early pacing slowdown

Replay: `1f1d07e558a3fb810f70a2cbc5a83d0c7a7980b0baf391643f6883c554b21b8f`.
Course 10801, 1,200 m, 12 runners, captured seed 636656617.

Gate 9 is a Senkou runner. Its observed start delay and sampled movement agree
through 11.721584 s. Private instrumentation of the frozen optimized source
reproduces the original binary output byte for byte, so the trace does not
alter the behavior being investigated.

At **12.320982933 s**, the simulator's gate-9 pace check interval expires:

- Position: 235.15005 m; previous manager rank: sixth.
- Pacemaker: gate 11, Nige, ranked first; live gap: 2.4424133 m.
- No rushing or active speed modifier is reported to the pace controller.
- The controller changes from idle to **PaceDown**, with coefficient **0.945**.
- It draws an exit gap of 3.8581736 m and sets the distance endpoint to
  285.15005 m. This consumes one random draw (786 to 787).

The next official observation exposes the divergence:

| Gate 9 at 12.787181854 s | Official | Simulator |
| --- | ---: | ---: |
| Position | 245.922714 m | 245.834229 m |
| Speed | 20.21 m/s, quantized | 19.953026 m/s |
| HP | 1,346, quantized | 1,350.306 |

The simulated runner is **8.85 cm behind** and has spent less HP. This is
consistent with an inappropriate pacing slowdown. The exact official internal
mode and transition time are not recorded, so the trace identifies the
simulator's mechanism, not yet the missing official rule. Later differences
spread to skills, lanes, other runners and finish order; the largest finish-time
error is 0.546677 s. These downstream symptoms should not be treated as separate
proven root causes.

Pacemaker tracing shows a preceding restoration from gate 4 (Senkou) to gate 11
(Nige) around 10.52 s. Seven existing experimental variations of restoration
timing and forward-runner following were tested. **None passes this race**:
six retain the first movement discrepancy, while disabling restoration moves
the first discrepancy earlier, to 11.721584 s. Forward-follow variants reduce
the largest finish error to 0.437881 s but leave the initial error unchanged;
that is not a correction. No experimental mode was adopted.

The next useful mechanics evidence is a matched set of short races in which a
Senkou briefly leads, a Nige regains the pacemaker role near a section boundary,
and another Senkou is close behind when its pace check becomes due. Compare
passing and failing captures around the restoration, speed-effect expiration,
pace-check clocks, and live-versus-cached pacemaker context. This is a collection
hypothesis, not an established trigger.

## Evidence and scope

The [machine-readable investigation record](../evidence/discrepancies-20261021.json)
links the retained per-case seed results, raw-packet checks, original practice
fixture, diagnostic traces and experimental-mode comparisons under
`<race-dataset>/discrepancies-20261021`.

Production Rust source was not modified by this investigation. The diagnostic
source and executable are separate from the live working tree. No additional
captures were quarantined or rewritten, and no release, commit or push was made.
