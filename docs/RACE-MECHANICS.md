# GlobalSim Race Mechanics

Version 77 · Complete implementation reference · Reviewed 3 November 2026

GlobalSim models a race as interacting runners advancing through a seeded sequence of timed updates. Speed, acceleration, stamina, positioning and skills influence one another; a formula alone does not determine the finishing order. This reference brings the existing mechanics guides together and explains the calculations, state changes and ordering used by the validated configuration.

The subject is GlobalSim's implementation. Agreement with recorded official races supports particular behaviours and boundaries; it does not establish every hidden operation of the official server. Read the calculations with the precision notes, because rounding and the moment a value is observed can change a later decision.

## 1 Scope and configuration

The official-priority default build uses the normal Cargo feature set, with no `experimental-umamoe` or other third-party comparison features enabled. The validated race configuration is supplied by [the v77 example](../examples/v77-race.jsonl). Its request enables `experimentalRetailInitialization`, `experimentalRetailSections`, `experimentalRetailRuntime` and `experimentalRetailGeometry`, and supplies the validated skill table. Those historical request-field names do not mean that the optional third-party Cargo features are enabled.

Omitting `experimentalRetailRuntime` selects the conventional runtime, whose randomness, pacing, movement and spurt implementation differ. The separate startup option is not enabled by the example. Later stamina mechanics also have their own switch. A plain release build and a correctly configured race request are both needed to reproduce the documented validation profile.

This reference describes that validated runtime unless a section explicitly identifies a conventional or optional path. Decimal constants in explanatory equations are readable approximations of stored parameters. Exact parameter text is retained in [the parameter catalogue](PARAMETERS.md), and the source links at the end identify the operation order.

### Evidence

The completed full practice audit matched 696,777 of 696,778 active official captures, or 99.99986% of tested races matching all checked observations. One movement discrepancy remains, with matching raw finish times and placements. The v77 release checks passed 167 Rust tests, 39 Python validation tests and all 84 included official regression fixtures.

The full inventory audit used the frozen 28 October collection and completed on 1 November. Previously verified comparisons were reused only for identical capture, input and complete-output bytes. The 3 November documentation review is not another corpus run. [The validation summary](VALIDATION-SUMMARY-20261103.md) separates those results from third-party comparisons and narrower tests.

## 2 Units clocks and runner identity

Distance is measured in metres, elapsed time in seconds, longitudinal speed in metres per second and acceleration in metres per second squared. HP is a stamina resource. Lane coordinates are course-width coordinates, not metres, and course geometry converts them into world-space positions.

Let D denote course distance. S, T, P, G and W denote adjusted speed, stamina, power, guts and wisdom. B is the course base speed. Raw stats, base stats and adjusted stats are distinct values; the correct one depends on the condition or calculation.

The simulation step is **0.0666 seconds**, not exactly one fifteenth of a second. The retail runtime stores its clock and many state variables in float32. A zero-time startup update does not grant a movement tick. Start delay can leave only a fraction of the first active tick available for movement.

Runner identity, input-list position, gate identity and cached rank are separate. Gate-dependent skills must use the intended gate, while arrays and event records may require a mapping back to the original input order. A live rank condition cannot use final placement. Rank-sensitive mechanics sometimes intentionally read the preceding field pass.

The course is divided into 24 sections. The principal phase boundaries are at sections 4, 16 and 20, followed by the finish. The implementation first stores `D / 24` as float32 and then multiplies by the boundary section. Algebraically equivalent direct fractions can round differently. The phase cached for AI and the phase selected for the speed table can differ on a crossing tick.

## 3 From input stats to race stats

### Overcap mood and team rank

Stats above 1,200 retain half of the excess. An odd excess retains a half-point.

$$
\operatorname{overcap}(x)=\begin{cases}x&x\leq1200\\1200+(x-1200)/2&x>1200\end{cases}
$$

Mood is bounded from −2 to +2. Its coefficient is formed from the stored master-rate arithmetic, approximately 1 + 0.02 times mood. The team-rank bonus is added after overcap handling and before mood. Base stats are bounded from 1 to 2,000.

$$
\operatorname{base}(x)\approx\operatorname{clamp}((\operatorname{overcap}(x)+b_{team})(1+0.02m),1,2000)
$$

| Team rank | Base stat bonus |
|---|---:|
| Omitted or zero | 0 |
| 1 to 5 | 50 |
| 6 to 10 | 30 |
| 11 to 15 | 20 |
| 16 to 20 | 10 |
| 21 and above | 0 |

The lookup uses the first strictly greater threshold from the race parameter table. It affects all five base stats and associated base-stat consumers, while the saved raw stats remain unchanged. Nonzero team-rank boundaries have synthetic tests but still need official capture validation.

For example, raw 1,201 at mood +2 and no team bonus becomes approximately 1,200.5 × 1.04 = 1,248.52 before later adjustments. The engine uses the float32 representation of the mood coefficient rather than an exact decimal 1.04.

### Course ground aptitude and Career adjustments

Adjusted speed uses the course modifier and ground-speed adjustment. Power uses the ground-power adjustment. Wisdom uses strategy aptitude. Stamina and guts receive their ordinary base values before any Career adjustment. With `singleMode`, the Career adjustment is +400 for every entrant and each adjusted stat is finally bounded from 1 to 2,000.

```text
S = clamp(baseSpeed * courseModifier + groundSpeed + careerBonus, 1, 2000)
T = clamp(baseStamina + careerBonus, 1, 2000)
P = clamp(basePower + groundPower + careerBonus, 1, 2000)
G = clamp(baseGuts + careerBonus, 1, 2000)
W = clamp(baseWisdom * strategyAptitude + careerBonus, 1, 2000)
```

These statements describe the dependency order. Speed and wisdom retain wider intermediate arithmetic across their product and additions before the float32 store. Storing an intermediate product earlier can change a boundary result.

| Adjustment | Good | Yielding | Soft | Heavy |
|---|---:|---:|---:|---:|
| Turf speed | 0 | 0 | 0 | −50 |
| Dirt speed | 0 | 0 | 0 | −50 |
| Turf power | 0 | −50 | −50 | −50 |
| Dirt power | −100 | −50 | −100 | −100 |

For each nonzero course-designated threshold stat, using its base value before Career adjustment, the course-speed contribution is 0.05 up to 300, 0.10 up to 600, 0.15 up to 900 and 0.20 up to 2,000; values outside those ranges follow the source lookup. Starting from float32 1, the engine adds each contribution divided by the number of designated stats, in order. No designated stats gives a modifier of 1.

Raw-stat conditions do not acquire the Career +400 bonus. Green skills and other stat effects have their own activation and receiver lifecycle. A later effect does not imply that every quantity precomputed by every runtime path is rebuilt.

Initial guts and wisdom rankings compare bounded adjusted stats with a relative float32 near-equality rule and a runner-identity tie break. They do not simply sort the raw displayed integers.

## 4 Strategy and aptitude

The five strategies are Front Runner (`Nige`), Pace Chaser (`Senkou`), Late Surger (`Sasi`), End Closer (`Oikomi`) and Runaway (`Oonige`). Runaway has its own physics coefficients but shares the Front Runner base-style group in several field queries. Grouping is mechanic-specific.

| Strategy | Opening speed | Middle speed | Late speed | HP coefficient |
|---|---:|---:|---:|---:|
| Front Runner | 1.000 | 0.980 | 0.962 | 0.950 |
| Pace Chaser | 0.978 | 0.991 | 0.975 | 0.890 |
| Late Surger | 0.938 | 0.998 | 0.994 | 1.000 |
| End Closer | 0.931 | 1.000 | 1.000 | 0.995 |
| Runaway | 1.063 | 0.962 | 0.950 | 0.860 |

| Strategy | Opening acceleration | Middle acceleration | Late acceleration |
|---|---:|---:|---:|
| Front Runner | 1.000 | 1.000 | 0.996 |
| Pace Chaser | 0.985 | 1.000 | 0.996 |
| Late Surger | 0.975 | 1.000 | 1.000 |
| End Closer | 0.945 | 1.000 | 0.997 |
| Runaway | 1.170 | 0.940 | 0.956 |

The captured End and Last table values are equal. The engine loads speed entries as percentages and converts them using the stored coefficient; the printed decimals above are not replacements for that conversion.

| Aptitude use | S | A | B | C | D | E | F | G |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Distance speed | 1.05 | 1 | .9 | .8 | .6 | .4 | .2 | .1 |
| Surface acceleration | 1.05 | 1 | .9 | .8 | .7 | .5 | .3 | .1 |
| Distance acceleration | 1 | 1 | 1 | 1 | 1 | .6 | .5 | .4 |
| Strategy wisdom | 1.1 | 1 | .85 | .75 | .6 | .4 | .2 | .1 |

## 5 Seeded initialization and the start

Initialization constructs runner state, skill lotteries and sampled activation windows in an ordered sequence. The conventional path uses Rule30 with derived per-runner and per-skill streams. Retail-style random consumers share an ordered stream. Replacing an integer draw with a floating draw, skipping a draw or changing runner preparation order can alter later events even if the seed is unchanged.

Standard starting-grid coordinates come from `data/starting_grids.json`. Gate identity remains distinct from dense stall order. The conventional normalized-grid formula and optional oversized-grid arrangements are separate configurations; larger custom fields are outside the claimed official validation scope.

The loaded starting-speed parameter is 3 m/s. Start dash adds an acceleration term of approximately 24 m/s² until its speed threshold is reached. Initial gate delay uses the loaded maximum of approximately 0.1 seconds and applicable initialization effects.

The delay countdown subtracts the float32 tick. A still-positive remainder suppresses movement. Crossing below zero gives a movement fraction equal to the negative remainder divided by the tick, using wider division. Exactly zero gives a whole movement tick. Successful gate-delay skill multipliers apply during initialization; a later activation records the skill without applying the delay multiplier again. Good-start and bad-start classification therefore retain their original basis.

## 6 Desired speed and actual speed

### Course speed and late speed

Course base speed depends on race distance:

$$
B=20-\frac{D-2000}{1000}
$$

The late speed addition is approximately:

$$
L=0.002\sqrt{500S}\,a_{distance}
$$

Base target speed is B times the strategy's phase coefficient, with L added in the late phase. At D = 2,000 m, S = 1,000 and distance aptitude A, B is 20 m/s and L is about 1.4142 m/s. A Pace Chaser's late base target is about 20.9142 m/s. Pacing, random variation, skills, terrain, HP and blocking still affect movement after this calculation.

Section variation is prepared for 24 sections from wisdom and random draws. The conventional explanatory form uses a wisdom term proportional to `(W / 5500) * log10(0.1 * W)` and a random contribution spanning 0.65 percentage points. The validated initialization preserves its own draw order and float32 stores; the conventional expression is not a substitute for replaying those consumers.

### Target speed pipeline

The retail target starts from base speed plus section variation, multiplied by the current pacing coefficient, then adds the force-in term. Active target-speed effects are added in receiver order. A moving-lane skill contributes approximately the square root of `0.0002 * P`.

Uphill correction is approximately `−slope * 200 / P`. Active downhill mode adds approximately `0.3 + abs(slope) / 10`. Being on a downhill segment alone does not enable downhill mode. Slope correction is followed by Spot Struggle and other late speed terms in their defined order.

If HP is depleted, the target becomes the minimum-speed result. The retail target is then bounded between 13 and 30 m/s. Blocking applies its own cap. Current-speed skill effects are separate from target-speed effects, so the runner's observed speed is not interchangeable with its desired speed.

The racing minimum-speed calculation is approximately:

$$
v_{min}=0.85B+0.001\sqrt{200G}
$$

The first term alone supplies the start-dash threshold. Finished runners use a distinct minimum of 5 m/s. An exhausted runner still racing can encounter the target minimum of 13 m/s while blocked by a finisher; the finisher's 5 m/s minimum must not be substituted for it.

## 7 Acceleration and movement

The explanatory acceleration equation is:

$$
a=K\sqrt{500P}\,c_{strategy,phase}\,a_{surface}\,a_{distance,accel}
$$

K is approximately 0.0004 uphill and 0.0006 otherwise. In the retail calculation, the multiplication order is power-root, hill coefficient, surface aptitude, distance aptitude and strategy-phase coefficient. Ordinary skill acceleration modifiers are then added in order, followed by duel and Conserve Power bonuses. Start dash adds its term, and acceleration is floored at zero.

The ordinary phase deceleration magnitudes are approximately 1.2, 0.8 and 1 m/s². Approaching a higher target, slowing from an excess speed and receiving an instantaneous current-speed effect are distinct operations. A target-speed bonus does not immediately teleport speed to the target.

Longitudinal movement follows the solver's velocity-Verlet-style sequence with explicit retail rounding. It must not be replaced with a single position-plus-speed formula if exact trajectories matter. Lateral movement, overlap resolution and course geometry also affect the final longitudinal advancement.

Geometry maps course distance and normalized lane into a world transform. Curvature, changing track width, slope regions and quaternion interpolation affect displacement. The geometry correction compares actual world travel with permitted movement. World-vector length is not the same quantity as course distance, and moving outward can alter the physical path.

## 8 HP consumption and recovery

Initial HP is approximately:

$$
HP_{max}=D+0.8T\,c_{HP}
$$

The engine keeps wider intermediate products and the sum before storing float32 HP. Its loaded coefficient is not an exact decimal 0.8.

The per-second consumption relationship is:

$$
q=20\frac{(v-B+12)^2}{144}\,r_{state}\,r_{ground}
$$

From the late phase onward, multiply by the guts factor:

$$
r_{guts}=1+\frac{200}{\sqrt{600G}}
$$

Tick consumption is the resulting rate times the effective elapsed time. Starting mid-tick uses the available fraction, not an extra full tick.

| HP state | Approximate rate |
|---|---:|
| Ordinary running | 1.0 |
| Pace Down | 0.6 |
| Rushing | 1.6 |
| Front Runner competition | 1.4 |
| Rushing Front Runner competition | 3.6 |
| Runaway competition | 3.5 |
| Rushing Runaway competition | 7.7 |

The engine selects one applicable state entry. It does not obtain combined competition rates by multiplying independent rushing and competition values. Active downhill mode multiplies the selected rate by approximately 0.4.

Ground multipliers in Good, Yielding, Soft and Heavy order are 1, 1, 1.02 and 1.02 on turf; on dirt they are 1, 1, 1.01 and 1.02. HP recovery and cost skills use their effect receivers. A negative recovery value is a cost and is not automatically a successful healing activation for conditions or history.

At D = 2,000 m and T = 1,000, the approximate initial HP is 2,760 for a Front Runner and 2,712 for a Pace Chaser. At v = B in ordinary conditions before the late phase, the explanatory consumption rate is 20 HP per second. Actual stored values follow the precision rules rather than decimal-only arithmetic.

## 9 Last spurt planning

Maximum spurt speed combines the late base, a course-speed addition, the speed-derived late addition and a guts term:

$$
v_{spurt}\approx1.05(v_{late}+0.01B)+L+0.0001(450G)^{0.597}
$$

The late addition L appears both within the late base and as the separate term shown. The implementation preserves the distinct stores and wider arithmetic in this calculation.

The validated planner evaluates what remaining HP can afford rather than starting every runner at the same distance. Its remaining-distance budget is `max(D − position − 60, 0)`. It first tests normal-pace affordability, then immediate maximum-speed affordability.

If a full spurt is not affordable, the search decreases sprint distance in 30 m steps and requires a planned start before D − 100. Speed is reduced in 0.1 m/s steps through the source's two separately bounded loops. A candidate pays the normal-pace portion and the sprint portion of its travel cost. The normal-pace product is stored as float32 before being widened and added to the wider sprint-cost product; changing this store can admit a different plan.

Feasible candidates are sorted by a comparator that truncates `(timeA − timeB) * 10000` to an integer. In that order, a plan is accepted when an integer draw from 0 through 99 is strictly below approximately `15 + 0.05 * W`. If every lottery fails, the last candidate is retained. Sorting and tie behaviour therefore affect which plan receives which draw.

Planner result codes distinguish immediate maximum spurt, selected alternatives and failures; failure to afford the normal baseline uses code 4 and failure to find a feasible candidate uses code 8. A recovery can request recalculation. An unsuccessful later attempt does not erase an already valid plan, and a retained start is handled separately from searching for a replacement.

## 10 Position keeping and the pacing reference

Position keeping is a state machine with a reference runner, timers, travel limits and a speed multiplier. The reference is often called the pacemaker. It is not necessarily the current race leader or the first Front Runner in the input list.

The normal entry window uses `floor(D / 24)` and the configured start and end sections, with entry before ten of those floored section lengths. This flooring belongs to position keeping; it must not be transferred to the distinct Spot Struggle entry cutoff. Existing modes have their own exit conditions and travel limits.

| Mode | Purpose and principal exit condition |
|---|---|
| Extra | Respond to a less-forward style ahead; ends when that circumstance disappears or its travel limit is reached |
| Speed Up | Increase separation from behind; checks the configured behind-gap condition |
| Overtake | Seek the lead; after leading, check the required gap behind |
| Pace Up | Close an excessive gap to the reference; leave when the exit gap is reached |
| Pace Down | Ease off when too close; leave on sufficient gap, self-reference or an applicable speed skill |

Ordinary checks have a two-second interval and a one-second cooldown after exit. Their clocks are updated sequentially; the interval does not count down through a still-positive cooldown. Crossing residue is retained. A rushing transition clears the mode and resets the relevant timers.

For Pace Chasers, Late Surgers and End Closers, base minimum/maximum gaps are 3/5, 6.5/7 and 7.5/8 metres. The distance scale is approximately `max(1, 1 + 0.0008 * (D − 1000))`; the Pace Chaser minimum stays unscaled. The multiply-and-add is evaluated with wider arithmetic before storage. At 2,500 m, the Pace Chaser maximum is exactly 11 m in the supported calculation. Equality does not enter the Pace Up lottery or consume its random draw.

Speed Up, Overtake and Pace Up use wisdom-dependent lotteries when applicable. The default pacing logarithm retains its established double-to-single rounding. The optional third-party logarithm calculation is not part of this default. Typical multipliers are approximately 1.04 for Speed Up/Pace Up, 1.05 for Overtake and 0.915 or 0.945 for Pace Down according to phase. The Extra multiplier is approximately 2. For applicable non-rushing checks, a floating draw in [0, 100) must be strictly below `log10(0.1 * boundedWisdom)` times the loaded mode coefficient: normally 20 for Speed Up/Overtake and 15 for Pace Up. The logarithm argument and return have explicit float32 stores. Rushing bypasses that lottery and its draw. Pace Up and Pace Down sample an exit gap from their applicable interval.

### Reference selection and restoration

Runaway shares the Front Runner base-style group for pacemaker selection. The field uses the preceding rank order with live positions. At section transitions, a less-forward leading style increments the counter used for recalculation. A section led by the most-forward style does not automatically reset that counter.

In a field without Front Runners, a detached forward-style runner can yield to the most-forward style within the configured range of the overall leader. After a handover, distance-based refresh also applies to fields with Front Runners. If the original forward group remains outside the configured range at a section boundary, selection can move to the leading runner of the most-forward style still in range. That refresh clears the counter before the current section is counted.

A section boundary retains the handed-over incumbent until the normal rule replaces it. Restoration selects the nearest eligible forward-style runner ahead of that incumbent in the retained ordering, rather than always selecting the forward group's leader. The optional staged-restoration profile changes when some runners observe a new reference and has known official regressions.

## 11 Rushing slopes and course events

Rushing changes HP state and pacing behaviour. It is not merely a displayed status. Initialization and subsequent entry/exit consumers must retain their order. The conventional wisdom chance uses a squared logarithmic expression; the validated path uses its own seeded preparation and runtime state transitions.

Retail rushing entry compares the float32 section quotient `distance / (D / 24)` with `startSection − 1`. Replacing the quotient with a multiplied metre boundary can shift entry by a tick. In a captured 1,600 m case, position 399.9999694824219 m produces a quotient that rounds to exactly 6. An absent rushing section must not start rushing or consume an activation draw.

The loaded rushing parameters use sections 2 through 9 for the sampled start, three-second exit checks, a 55-percent exit parameter and a 12-second forced end. Temporary pacing style depends on the rushing mode. Its effect on reference lookup differs in the optional staged profile and should not be silently applied to the default.

Downhill mode has its own enter/exit checks and timers. Its loaded wisdom-entry coefficient is approximately 0.04 and its exit parameter is 20 percent. While active it contributes the downhill speed and HP adjustments described earlier.

Slope and straight transitions are events. The gradient comes from a slope event, while its end point is selected from a same-direction region containing the current position, including boundaries. At a shared uphill boundary, that can select the preceding region's end. After passing it, the gradient expires until another event. A position-only lookup on every tick would reintroduce a gradient incorrectly.

A straight-entry event clears current corner membership even if nominal corner geometry overlaps it. The persistent flag recording that the final corner has been reached remains set. Being in a corner, having reached the final corner and entering the final straight are different conditions.

## 12 Passing lanes and blocking

Lane selection uses cached and live observations at specific points in the tick. Candidate passing routes are evaluated in target order, inward then outward, and scored by weighted lateral distance. Lower cost wins. The inward weight is 100 in the opening phase, 1 in the middle phase and approximately 1.15 later; the outward weight is 1. Applicable lane-preference effects use a weight of 1.5.

An exact float32 cost tie is not a generic instruction to choose the outer lane. A later route replaces the retained route only when its original target has a farther cached longitudinal position, both routes move toward their respective obstacle side, and the candidate fits available body clearance while the retained route does not. Ties between sides of the same target retain their existing visitation behaviour.

Body clearance uses the loaded `SideBlockLaneGap`, subtracting it from neighbor-limited space and bounding at zero. An unobstructed edge retains its available space. This preference is distinct from deciding whether a corridor is reachable.

For an inward corridor, the stored lane and constants are widened before completing each bound, then the finished bounds are rounded to float32. The constants are approximately 0.04444444924592972 and 0.0555555559694767. Rounding the first subtraction separately is a different calculation. Peer-lane comparisons include the rounded boundaries.

### Lateral movement

The approximate base lateral rate is `(0.001 * P + 0.3) * 0.02` in lane units per second. In the late phase it is multiplied by `1 + 0.01 * (rank − 1)`, so the leader receives a factor of 1. Earlier, before the first-move point, the factor is approximately `1 + 0.05 * lane / maxLane`; otherwise it is 1.

Stored lateral speed approaches that rate using the loaded lateral acceleration. Ordered skill modifiers are applied and the magnitude is bounded from 0 to 0.6. Inward movement negates the result and multiplies it by `1 + lane`, using wider arithmetic before storage. The lane update is then bounded by the selected target and course limits. Separate collision and overlap checks can restrict the movement.

### Front blocking

The front-block envelope uses a positive longitudinal gap strictly below the loaded front distance, normally 2 m. Its lane limit narrows with distance, approximately as follows:

$$
g_{lane}\leq\left(1-0.6\frac{g_{distance}}{d_{front}}\right)l_{front}
$$

The stored front-lane parameter is approximately 0.0416666716 course-width units. Position, lane and ratio stores use float32, but the complement and final width multiplication retain wider precision until the final limit is stored.

For an accepted blocker, define r as the longitudinal gap divided by the front distance, bounded from 0 to 1. The speed cap is the greater of minimum speed and blocker speed multiplied by a rate interpolating from approximately 0.988 to 1 as r increases.

Side-blocked timers exclude finished runners; front-block checks include them. Finished runners can also influence passing decisions. After their one-second lateral grace period, lateral speed becomes signed tick duration, so an unobstructed lateral step is tick duration squared, capped at the target lane and course limits. Ordinary lateral acceleration and skill modifiers do not apply to that branch.

## 13 Competition and dueling

### Spot Struggle

Spot Struggle coordinates nearby runners of the appropriate leading style. Its loaded start-distance threshold is 150 m, its entry cutoff is section 6 and its end cutoff is section 9. Initial proximity uses a distance gap of 3.75 m and a normalized lane gap of approximately 0.165. Continued-state checks use their own thresholds and timers.

The group's cached leader must lead the eligible field. Ordinary Front Runners exclude Runaways from that eligible ranking but must lead other styles; Runaways must lead the whole field. The 150 m test reads the live position of the cached overall leader, even when that leader is a Runaway. Passing the leader during movement does not substitute a new cached leader until ranks update.

Each invited partner must be strictly before the section-6 entry cutoff. A cached group leader can already be beyond it when an eligible partner starts competition; this does not admit additional late partners. The default calculation retains fractional metres:

$$
d_{entry}=6(D/24)
$$

The expression has the section-length rounding described earlier; it must not be replaced by arbitrary algebraic reassociation. For a 1,150 m course, the boundary is 287.5 m. An observed participant at 287.1053466796875 m is eligible. Flooring this cutoff to 287 m removes an official entry and changes the race.

Spot Struggle adds approximately `0.0001 * (500 * G)^0.6` to target speed. Its timer duration is approximately `0.012 * sqrt(700 * G)` times the strategy-proficiency coefficient. The retail speed term preserves a float32 root-power result and modifier store. Spot Struggle also changes HP state. Its speed term is added after slope correction. When both base styles start on one tick, Runaway event records precede ordinary Front Runner records. Each group records its cached leader first, followed by partners in the prepared base-style roster order. Sorting ties and larger rosters can affect that order.

### Duel entry and exit

Dueling requires the relevant final-corner/final-straight state. The principal entry checks include proximity within 3 m longitudinally and 0.25 lane units, two seconds of eligible proximity, an observed speed difference no greater than the stored approximately 0.6 m/s threshold, at least 15 percent HP for both runners, and at least one participant in the rounded top half of the field.

The retail path selects the longest-standing eligible nearby partner, preserving ascending-index ties, and then checks that partner's duel history. If the selected partner's earlier duel has ended, the search does not fall back to a newer partner. A valid invitation commits both participants together and increments actual duel counts.

The approximate target-speed and acceleration additions are:

$$
\Delta v=0.0001(200G)^{0.708}
$$

$$
\Delta a=0.0001(160G)^{0.59}
$$

Loaded coefficients and intermediate stores determine the precise values. An active duel ends under its separation or low-HP conditions. The default low-HP comparison widens the stored values and evaluates `remainingHP / maximumHP < 0.05`. It does not calculate a float32 percentage first. At an observed Kyoto boundary, the wider ratio is below 0.05 while the percentage rounds to exactly 5; using the percentage delays exit by a tick.

Crossing the finish does not itself end an active retail duel. An eligible racer may invite a finished partner that still meets the other checks. A finisher can also complete its own warmup while the cached pair retains a partner on the final straight. Finishing does not grant another activation to a runner that has already participated.

## 14 Conserve Power and optional stamina systems

The supported Conserve Power interval ledger requires power above 1,200, a supported style and unchanged power during accumulation. Unsupported cases retain a separately identified fallback rather than silently using the ledger.

The clock starts at the first AI update after gate delay. Accumulation requires position below `floor(D / 24) * 10`. Each due second credits the preceding interval: approximately 6.7 in Pace Down or 4.2 otherwise, multiplied by 0.75 for Pace Chasers, 0.7 for End Closers and 1 for other supported styles.

Blocking, rushing and Spot Struggle suppress gains. Leaving suppression introduces a 2.5-second first-credit wait. Rushing entry multiplies the gauge by approximately 0.8; Spot Struggle entry uses approximately 0.95. The cutoff tick can still credit its completed interval. Float32 clocks and threshold ordering matter.

The supported release duration is approximately:

$$
t_{release}=\frac{gauge\sqrt{D}\,c_{distance}}{1450}
$$

Short, mile, middle and long coefficients are respectively 0.45, 1, 0.875 and 0.8. Release acceleration is approximately:

$$
a_{release}=0.001\sqrt{\max(130(P_{conserve}-1200),0)}\,c_{style,distance}
$$

The conserve-power input is prepared separately from ordinary adjusted power. It is the mood- and team-adjusted base power plus half of the original excess over 1,200. It must not be replaced with ordinary adjusted power by assumption. The complete style/distance matrix remains in the builder and parameter catalogue. The fallback duration constant of 0.0185 belongs to a different fitted formula.

Secure Lead, stamina keeping and Compete Before Spurt include separately gated later-stamina behaviour. `experimentalLaterStaminaMechanics` controls that optional path. Its approximations include a flat Secure Lead coefficient of 3 and synthetic builder effects. Their presence in the source is not evidence that all later official mechanics are reconstructed or active in the validated example.

## 15 Skill definitions conditions and lotteries

A skill definition contains one or more detail branches, prerequisites, conditions, effects, priority, cooldown and duration/value usages. The supplied `skillData` table takes precedence over the bundled fallback. The v77 example supplies the validated table; omitting it can select a different set of definitions.

Conditions are parsed into expressions and eligible distance regions. Static checks use course, roster and appropriate raw/base-stat facts. Dynamic checks read position, cached phase/rank, HP, lane, overtaking, blocking, competition and skill history. OR alternatives retain separate regions and prerequisites. A sampled activation position still requires its remaining runtime predicates.

Prepared races can reuse immutable expressions and decoded definitions. Every seed must receive new sampled windows, prerequisite latches, activation observers and mutable state. Reusing a stateful prerequisite across seeds can change later races in a persistent worker even when a single-race rerun looks correct.

The ordinary wisdom-lottery relationship is approximately `max(0.2, 1 − 90 / baseWisdom)` for definitions that request it. Wisdom 900 gives 0.9. Retail initialization has an ordered lottery and trigger-construction sequence, and random-skill reservations can override an ordinary failure. `wisdomChecks:false` is an experimental control, not the validated setting.

### Conditions with boundary-sensitive semantics

The retail `hp_per` condition stores the remaining/max ratio as float32, multiplies by 100 in wider arithmetic and truncates to an integer. A stored `0.71_f32` is slightly below 0.71 and therefore produces 70, not 71, through this path. This is distinct from the duel-exit calculation.

The final-corner random trigger searches backward for a corner numbered at least 4 whose start is within the last 1,000 m. Without a qualifying corner, it has no activation window and consumes no draw. A one-time final-straight condition is eligible on the entry tick after the final corner; a later rank change does not recreate that entry event. A cornerless course does not generate it.

Continuous rank-band conditions retain history accumulated from five seconds onward. The rank pass records the preceding rank before publishing the new order. Leaving the band breaks continuity when that departure reaches the history update; a later return does not erase it.

Overtake-related additional effects read cached phase and corner state. A pass registered after movement on a boundary tick belongs to the state at the beginning of that tick. A pass on the tick entering the final corner is not yet a pass after final-corner entry.

## 16 Skill values targets and lifetimes

Fixed master effect amounts ordinarily use a 1/10,000 scale. Level curves differ by effect family, and the retail converter rounds master conversion and level multiplication separately. Stats, target speed, current speed, acceleration, HP, vision, lane preferences, rushing and random activation have different receivers.

Dynamic value usages are not all multiplicative. Team-stat sums, fans and Career wins can affect prepared values. Random usages 8 and 9 select 0, 0.02 or 0.04 with weights 60, 30 and 10. A master amount of −10,000 with those usages is a random 0/2/4-percent cost, not automatically a full-HP loss. Usage 14 depends on distinct activated green-tagged skills.

Recipient selection is dispatched through the field, including implemented team exclusions and target immunity. Target selection can consume RNG before ability-value randomization. The official visual recipient mask describes the first ability's targets, not the union of every gameplay recipient. Per-ability target events and additional-activation subsets must retain their own identity.

Healer-targeting skills read the most recent distinct healers from history cached after the preceding field tick. A heal during the current skill pass becomes visible to that selection on the next tick. Combining current-pass history with cached movement state changes the result.

Ordinary distance-scaled durations and cooldowns use course distance in kilometres. Special duration usages depend on runtime inputs such as HP or leader gap. Negative duration represents a permanent effect. Expiry removes the receiver; cooldown can begin on expiry according to the detail branch.

Leader-gap duration scaling uses the coefficient `min(1.6, 0.8 + max(0, gap) / 62.5)`. The normalized gap and resulting coefficient are stored as float32 before multiplying the distance-scaled duration. Keeping the division wider can change the recorded wire duration.

Additional activations are applied during the running skill's ordered ascending-ID turn before the detail stops. Its final turn can include queued earlier bonuses and qualifying activations earlier in the same pass, even if the original receiver expired during preparation. A bonus lasts for the detail's remaining time plus one tick; it does not extend the original receiver. Caps and the final ordered turn still constrain activation.

Order-up ability-time usage 4 can extend a running detail and surviving abilities by `OrderUpAddAbilityTime * D / 1000`, up to three increments. Extension follows receiver aging but precedes running-detail expiry. It must not revive a main receiver that already expired.

After expiry, additive modifiers are recalculated in order from surviving receivers. Subtracting the removed amount from a running total can round differently. Recorded duration is also distinct from lifetime: the observed conversion widens a float32 duration, multiplies into 0.1 ms units and truncates; time-zero skill events use the sentinel −1.

## 17 The complete field tick

The manager's ordering determines what each runner can observe. A faithful implementation cannot replace it with an arbitrary fully-update-one-runner loop.

1. Capture the pre-movement roster and prepare duel proximity, geometry width and lane caches.
2. Prepare each runner's tick, including runtime state and receiver expiry.
3. Process skill priority 1 across the field, then priority 0, dispatching effects.
4. Capture the post-skill roster and refresh front blockers and blocking state.
5. Advance runners in the retained update order. Replace each live roster entry immediately and assign overtaking timers; later runners can see earlier movement updates.
6. Resolve lateral overlaps, prepare duel participants and latch active Spot Struggle cutoffs.
7. Update eligible field conditions using each runner's effective elapsed time.
8. Refresh the field, flush effects and publish activation and healer history for the next tick.

Shared immutable preparation is an optimization. Random streams, counters, timers, sampled conditions and effect receivers belong to the individual seed. Independent batches must produce the same supported outputs regardless of the worker's previous requests. Concurrent handles do not share mutable race state.

## 18 Finishing results and replay observations

A finish time is interpolated only between the adjacent simulation ticks that straddle the finish. In explanatory notation:

$$
t_{finish}=t_{frame}-\operatorname{clamp}\left(\frac{x_1-D}{x_1-x_0},0,1\right)\Delta t
$$

Using two sparse replay samples instead of the adjacent simulation ticks changes the answer. The worker reports `finishTimeRaw` for the interpolated elapsed crossing and `finishTime` for the enclosing tick end. A display-clock conversion, if required by a consumer, is a separate operation. The presence of a `FinishTimeCoef` parameter does not mean the worker applies it to these fields. Final placement and later movement are separate observations. Post-finish movement does not rewrite an already recorded finish.

With ordinary output, the recorder generally retains the first 17 effective frames, every 16th frame and the final frame, with extra finish samples. A synthetic gate frame can shift the effective index. `retainAllFrames:true` retains the simulator's full frames; it does not create missing observations in an official reference. `omitFrames` changes the requested output surface, not the intended race mechanics.

Style-leading transitions and rushing counters also feed local scoring observations. These are not a complete implementation of official Team Trials scores, reward calculation or server persistence. Producing race results and replay observations is the engine's responsibility; account progression and playback presentation are separate systems.

## 19 Precision validation and known limits

The retail model is mixed precision. Many parameters and state fields are float32, while selected products, sums, divisions and interpolations remain wider until a specified store. Reassociating arithmetic, using an exact decimal in place of its stored float, changing sort stability or moving an RNG draw can alter a boundary outcome.

Official checks cover raw finish times, placements, recorded position/speed/HP/lane, exposed state, skill identities and branch/timing/order observations, durations, visual recipient masks and competition events. Integrity checks also inspect the complete simulator output for structural and temporal consistency. Sparse official frames cannot prove equality on intervening ticks or unexposed state.

The active unresolved practice case is Tokyo dirt 1,600 m, seed 580983021, with 16 runners. It has 85 sampled position differences, at most about 1.221 mm, two HP differences and two spurt-distance differences. Its raw finish times and placements match. It remains a strict failure.

Third-party simulations help generate hypotheses but do not override conflicting official captures. The full optional uma.moe profile matches 399,947/400,000 sampled reports but only 72/84 official fixtures. Recent restoration proposals fixed some selected third-party failures and regressed more controls, so none was retained. Report agreement does not establish equality of all internal decisions.

Nonzero Career team-rank boundaries, optional later stamina mechanics, custom oversized fields and unsupported Conserve Power cases retain the limits described above. The [accuracy guide](ACCURACY.md) and [current validation summary](VALIDATION-SUMMARY-20261103.md) report the tested scopes without combining incompatible denominators into one score.

## 20 Source map and companion references

| Subject | Implementation or reference |
|---|---|
| Inputs and validated example | [Interface](INTERFACE.md), [v77 example](../examples/v77-race.jsonl) |
| Stat construction and skill definitions | [Builder](../src/race_solver_builder.rs) |
| Course modifiers and segments | [Course data](../src/course_data.rs) |
| Physics and precision | [Retail physics](../src/retail_physics.rs), [mathematics](MATHEMATICS.md) |
| Field ordering and observations | [Retail field manager](../src/race_field_retail.rs), [architecture](ARCHITECTURE.md) |
| Pacemaker selection and shared queries | [Field](../src/race_field.rs) |
| Pacing state machine | [Position keeping](../src/retail_position_keep.rs) |
| Lanes geometry and blocking | [Lane logic](../src/retail_lane.rs), [geometry](../src/retail_geometry.rs) |
| Rushing and slope states | [Retail states](../src/retail_states.rs) |
| Spurt search | [Retail spurt](../src/retail_spurt.rs) |
| Skills and expression semantics | [Conditions](../src/activation_conditions.rs), [skill guide](SKILLS.md) |
| Competition and runner updates | [Solver](../src/race_solver.rs), [retail runner](../src/race_solver_retail.rs) |
| Conserve Power | [Interval ledger](../src/conserve_power.rs) |
| Randomness and sorting | [Retail RNG](../src/retail_random.rs), [retail sort](../src/retail_sort.rs) |
| Complete parameter inventory | [Parameter catalogue](PARAMETERS.md) |

This reference consolidates the GlobalSim mathematics, field mechanics, skill, architecture, parameter and validation guides. The code and parameter files determine the implemented operation order where a readable equation omits float stores. The Markdown source and accompanying Word document contain the same substantive reference text.
