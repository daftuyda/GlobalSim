# Field mechanics and randomness

## Starting stalls and delay

Gate identity is kept distinct from dense stall order: gate-dependent skills
must not be evaluated against an array index accidentally. Standard starting
grids are stored in `data/starting_grids.json`. The generic starting coordinate
uses `(column + (column>=9 ? .6 : 0))/18` course-width units. The experimental
large grid uses 20 columns and starts each later row 3 m farther behind.

[`retail_initialization.rs`](../src/retail_initialization.rs) reconstructs
seeded initialization, wisdom checks, activation windows and opening values.
[`retail_physics.rs`](../src/retail_physics.rs) subtracts dt from the float32
start-delay countdown. Positive remaining delay suppresses movement; crossing
zero gives fraction `-remaining/dt` (double division). Exactly zero gives a
whole tick. A zero-time update suppresses AI/movement even without delay.

## Section variation and rushing

The default solver samples 24 section modifiers:

```text
maxVariation = W/5500 * log10(.1*W)
sectionBonus = B*(maxVariation-.65+.65*U)/100
```

Here U is a random draw in the generator's unit interval. The ordinary rushing
chance is `(0.65/log10(.1*W+1))²`, plus applicable skill modifiers; start is
sampled in sections 2–9. Its constructor consumes duration draws even if the
runner does not rush. The reconstructed path has different ordered consumers
in initialization and [`retail_states.rs`](../src/retail_states.rs).

Rushing affects pacing and HP state, not just an animation. Its timers,
exit checks and shared random draws can influence later skill outcomes.
Downhill acceleration mode likewise has enter/exit checks and timers; being
on a negative slope does not by itself guarantee the speed/HP benefit.

Slope changes are processed as events, with an active gradient and end point.
The gradient comes from the event, while its end point comes from the first
same-direction slope region containing the runner, including both boundaries.
At the exact shared boundary between two uphill regions, this can select the
previous region's end point. The new gradient then expires as soon as the runner
passes that point and stays inactive until another slope event. A position-only
lookup would incorrectly reactivate it on every tick.

## Position keeping and pacemaker

[`retail_position_keep.rs`](../src/retail_position_keep.rs) implements a state
machine with selected target, check timers, traveled distance, speed multiplier
and mode. Decisions depend on strategy, rank, leader gap, rushing and nearby
runners. The conventional path uses `update_position_keep*` in `race_solver.rs`.

In the ordinary path, base minimum/maximum gaps for Senkou, Sasi, Oikomi are
(3,5), (6.5,7), (7.5,8) metres. The distance scale is
`max(1,.0008*(D-1000)+1)`; Senkou's minimum alone is unscaled. Different modes
accelerate to catch up, ease off when too close, or handle front-runner rivalry.
Reconstructed thresholds and multipliers come from the parameter tables.

The detailed runtime groups Runaway with Front Runner and selects among active
runners using the preceding rank pass and current positions. At each new
section, a lead by a less-forward style increments a counter. A section led by
the most-forward style does not by itself clear that counter. Recalculation
uses `PaseMakerChangeTopHorseNotMostForwardStyleCount` and
`PaseMakerChangeForwardRunningStyleRange` from the parameter table.

In a field without Front Runners, a detached forward-style runner can yield
to the most-forward style within range of the overall leader. After a handover,
the forward group's leader passing the incumbent pacemaker restores the forward
pacemaker and clears handover state, even if a different running style still
leads the race. A change of overall leader alone does not trigger restoration
while the forward group's leader remains behind the incumbent. Selection stops
at the configured end section (section 10); the incumbent
remains available to subsequent position-keep exit checks.

## Vision, overtaking, blocking and lanes

[`overtaking.rs`](../src/overtaking.rs) distinguishes a target ahead from an
actual recorded overtake. [`lane_selection.rs`](../src/lane_selection.rs) and
[`retail_lane.rs`](../src/retail_lane.rs) evaluate visible runners, catchability,
front overlap, side room, reachable lanes and route preference. Passing requires
a corridor; a faster target speed does not permit movement through a runner.

The reconstructed lane controller collects pre-update context, then may query
live targets. It retains targets/cooldowns, accelerates lateral motion and
applies side constraints. Overlap resolution is a separate field pass. Lane
values and lane-gap constants are normalized course widths, not metres.
The final-corner outward preference does not disable passing or force an
unconditional fixed lane. Finished runners are excluded where the relevant
blocking/competition checks require active rivals.

Front blocking caps speed according to minimum speed, blocker speed and gap
(see mathematics). Lane movement can also change world-path length. Therefore
longitudinal position, lane, blocker identity and speed must be compared
together when diagnosing a mismatch.

## Competition and later stamina mechanics

`race_solver.rs` contains Spot Struggle, Compete Before Spurt, Secure Lead,
stamina keeping and dueling. These have strategy, position, lane, proximity,
timer and HP eligibility rules. For example Spot Struggle's normalized lane
gap is .165, its entry cutoff is section 6 and end cutoff section 9; the duel
lane gap is .25. Duel proximity must persist and real starts increment the
duel count used by skill conditions. A finish is not a duel event.

In the detailed runtime, duel entry first selects the longest-standing eligible
nearby partner, then checks that partner's duel history. A selected partner
whose earlier duel has ended prevents entry; selection does not fall back to a
newer partner. A valid invitation activates both runners together, including a
runner whose own partner search failed. Group membership, eligibility and
participation counts are updated with the activation.

The later stamina behavior is explicitly gated by
`experimentalLaterStaminaMechanics`. Its approximations include the flat
Secure Lead coefficient (`SECURE_LEAD_COEF_APPROX=3`) and synthetic builder
effects for conserve power/stamina competition. Do not present these as a
complete reconstruction. The source index links each update and helper;
constants and applicable branches remain in the source rather than being
silently generalized across versions.

Competition distance-cost coefficients for the conventional formulas are .3
below 1801 m, .5 below 2101, .8 below 2201, 1 below 2401, 1.1 below 2601,
and 1.2 thereafter. Strategy-specific coefficients further modify these costs.

## Randomness and ranking

The conventional engine uses [`random_gen.rs`](../src/random_gen.rs), a Rule30
generator with derived per-runner/per-skill streams. The reconstructed engine
uses [`retail_random.rs`](../src/retail_random.rs) and the shared ordered
consumers in initialization, lane targeting, pacing, hills, skills and spurt
planning. It has integer and floating draw paths; replacing one with the other
changes consumption and potentially results.

[`retail_sort.rs`](../src/retail_sort.rs) reconstructs comparison sorting,
including behavior around equal/near-equal values. Generic stable sorting is
not necessarily equivalent. Cached ranks may intentionally reflect the
previous field pass. Never substitute final rank in a live rank condition.

## Score observations

[`stadium_score.rs`](../src/stadium_score.rs) records style-leading phase
transitions and rushing observations. Style leaders use cached minimum order
among equal base styles (Runaway is grouped with Front Runner). The rushing
counter pauses when not rushing and resets its one-second countdown after a
count, discarding overshoot. These observations do not implement the server's
complete reward or Team Trials scoring system.

## Conserve Power accounting

[`conserve_power.rs`](../src/conserve_power.rs) is a separate observed interval
ledger, not just the legacy builder approximation. Its supported path needs
power above 1200, style 1–5 and unchanged power during accumulation. Unsupported
cases are marked and retain the legacy fallback.

The clock starts at the first AI update after gate delay. Accumulation requires
`position < floor(D/24)*10`. Each due second credits the previous interval's
mode: 6.7 in pace-down or 4.2 otherwise, multiplied by .75 for Senkou, .7 for
Oikomi, and 1 otherwise. The elapsed clock and thresholds are float32.
Blocking, rushing or Spot Struggle suppresses gains. Leaving suppression starts
a 2.5-second first-credit wait. Rushing entry multiplies gauge by .8; Spot
Struggle entry by .95. The first cutoff tick can credit its completed interval.

Supported release duration is `gauge*sqrt(D)*distanceTypeCoefficient/1450`,
rounded to float32. Coefficients for short/mile/middle/long are .45,1,.875,.8.
Release acceleration is `sqrt(max((power-1200)*130,0))*.001*styleDistanceCoef`:
float32 subtraction/product/root, then wider coefficient products before the
final rounding. The complete matrix is in `race_solver_builder.rs`.
The legacy fallback's fitted .0185 duration constant is a separate formula.
Exact hidden server timer code is not established by benchmark agreement.
