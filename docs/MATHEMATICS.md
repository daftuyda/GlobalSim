# Race mathematics

These are implementation equations, not a claim that every operation is the
official server's equation. Distances are metres, time seconds, speed m/s and
acceleration m/s². HP is the simulator's stamina resource, not a distance.
Lane coordinates use course-width units. Let `D` be course length; `S,T,P,G,W`
are adjusted speed, stamina, power, guts and wisdom. `clamp(x,a,b)` bounds x.
Exact float operation order lives in the linked source.

## 1. Raw, base and adjusted stats

In [`race_solver_builder.rs`](../src/race_solver_builder.rs):

```text
overcap(x) = x                         if x <= 1200
             1200 + (x - 1200)/2       otherwise
moodFactor = f32(10000 + 200*clamp(mood,-2,2)) * f32(.0001)
base(x) = clamp(f32(overcap(x))*moodFactor, 1, 2000)
careerBonus = 400 if singleMode else 0
S = clamp(baseSpeed*courseModifier + groundSpeed + careerBonus, 1, 2000)
T = clamp(baseStamina + careerBonus, 1, 2000)
P = clamp(basePower + groundPower + careerBonus, 1, 2000)
G = clamp(baseGuts + careerBonus, 1, 2000)
W = clamp(baseWisdom*strategyAptitude + careerBonus, 1, 2000)
```

Odd excess stats retain half-points. For example, raw 1201 at mood +2 becomes
approximately 1200.5 × 1.04 = 1248.52 before subsequent adjustments;
The engine uses the float32 master-rate coefficient, not exact decimal arithmetic. Raw-stat conditions and
special conserve-power inputs are separate from these adjusted values.
For speed, multiply the float32 base stat and course coefficient in wider
arithmetic, add the ground adjustment, then store that combined value as
float32. Add the career bonus afterward. Rounding the course product before
the ground adjustment can change the result when the penalty crosses a float
exponent boundary.
`raw_stamina = rawStamina*moodFactor`; the conserve-power input is
`base(rawPower) + max(rawPower-1200,0)/2`.

For each nonzero course-designated threshold stat x, the contribution is .05
when x <= 300, .10 when x <= 600, .15 when x <= 900, .20 when x <= 2000,
and 0 otherwise. Starting from float32 1, add each contribution divided by the
number of nonzero designated stats, in order. No designated stats means 1.
See
[`course_speed_modifier`](../src/course_data.rs).

Ground conditions are ordered Good, Yielding, Soft, Heavy:

| Adjustment | Good | Yielding | Soft | Heavy |
|---|---:|---:|---:|---:|
| Turf speed | 0 | 0 | 0 | -50 |
| Dirt speed | 0 | 0 | 0 | -50 |
| Turf power | 0 | -50 | -50 | -50 |
| Dirt power | -100 | -50 | -100 | -100 |

Career +400 applies to all entrants when requested, after ordinary adjustments.
It does not alter saved raw stats. Green skills and runtime stat effects have
their own activation/receiver lifecycle; not all precomputed default-path
quantities are recalculated after a later stat change.

## 2. Phases and strategy

The course has 24 sections. Boundaries are section 4 (D/6), 16 (2D/3),
20 (5D/6), then the finish. The reconstructed path calculates `section=D/24`
in float32 and multiplies by 4,16,20. Directly calculating the fractions can
round differently. Cached AI phase and current-position speed-table phase
are separate concepts at a crossing.

Default phase coefficients, opening/middle/late:

| Style | Speed coefficients | Acceleration coefficients | HP coefficient |
|---|---|---|---:|
| Nige / Front Runner | 1 / .98 / .962 | 1 / 1 / .996 | .95 |
| Senkou / Pace Chaser | .978 / .991 / .975 | .985 / 1 / .996 | .89 |
| Sasi / Late Surger | .938 / .998 / .994 | .975 / 1 / 1 | 1 |
| Oikomi / End Closer | .931 / 1 / 1 | .945 / 1 / .997 | .995 |
| Oonige / Runaway | 1.063 / .962 / .95 | 1.17 / .94 / .956 | .86 |

The reconstructed path reads corresponding Start/Middle/End/Last arrays from
`global_parameters.json`; speed table values are percentages and multiplied
by .01. The [parameter reference](PARAMETERS.md) includes every captured value.

Aptitude multipliers in grade order **S,A,B,C,D,E,F,G**:

| Use | Multipliers |
|---|---|
| Distance, speed | 1.05, 1, .9, .8, .6, .4, .2, .1 |
| Surface, acceleration | 1.05, 1, .9, .8, .7, .5, .3, .1 |
| Distance, acceleration | 1, 1, 1, 1, 1, .6, .5, .4 |
| Strategy, wisdom | 1.1, 1, .85, .75, .6, .4, .2, .1 |

## 3. Speed and acceleration

[`race_solver.rs`](../src/race_solver.rs) defines the default equations:

```text
B = 20 - (D - 2000)/1000
lateBonus = sqrt(500*S)*distanceSpeedAptitude*.002
baseTarget = B*stylePhaseSpeed + (late phase ? lateBonus : 0)
maxSpurt = (lateBaseTarget + .01*B)*1.05
           + lateBonus + (450*G)^.597*.0001
baseAcceleration = K*sqrt(500*P)*stylePhaseAcceleration
                   *surfaceAccelerationAptitude*distanceAccelerationAptitude
K = .0004 uphill, .0006 otherwise
```

For example D=2000 gives B=20. At S=1000 and distance aptitude A,
lateBonus is about 1.4142 m/s; a Pace Chaser's late base is about 20.9142 m/s.
This is not final speed: pacing, hills, section variation, skills, HP and
blocking still apply.

The reconstructed [`retail_physics.rs`](../src/retail_physics.rs) reads the
coefficients rather than hard-coding the above decimal approximations. Its
target-speed pipeline is:

```text
target = (base + sectionVariation)*paceMultiplier + forceIn
target = ordered_add(target, active target-speed modifiers)
if movingLaneSkill: target += sqrt(clamp(P,1,2000)*.0002)
if uphill: target -= slope*(UpSlopeAddSpeedVal1/clamp(P,1,2000))
if downhill mode: target += DownSlopeAddSpeedVal1 + abs(slope)/DownSlopeAddSpeedVal2
target += spotStruggleTerm
target += otherLateBonus
if HP depleted: target = minimumSpeed
target = clamp(target, TargetSpeedMin, 30)
```

The minimum is `B*MinSpeedRate + sqrt(G*MinSpeedGutsCoefSqrt)*MinSpeedGutsCoef`.
Start dash uses the first term as its threshold. A front blocker separately
limits speed using `r=clamp(gap/FrontBlockDistanceGap,0,1)` and
`max(minimumSpeed, blockerSpeed*(rateMin+r*(rateMax-rateMin)))`.

Reconstructed acceleration multiplies power-root, hill coefficient, ground
aptitude, distance aptitude and style/phase coefficient in that order, adds
ordered skill modifiers, adds StartAccelAdd during start dash, and floors at 0.
Default phase deceleration values are -1.2, -.8, -1 m/s². Acceleration toward a
target, deceleration from an excess speed and instantaneous current-speed
effects are separate operations.

## 4. HP and stamina consumption

See [`hp_policy.rs`](../src/hp_policy.rs) and `retail_physics.rs`.
Initial HP in the ordinary model is `D + .8*T*styleHpCoefficient`.
The reconstructed form retains double intermediate products/sum until a float32
store, and uses captured `HpInitialVal1` and `HpMaxCoef*`.

The exact parameterized consumption equation is:

```text
gap = (currentSpeed-B) + SpeedGapParam1
lossPerSecond = gap² / SpeedGapParam1Pow * HpDecBase * stateRate * groundRate
if phase >= 2:
    lossPerSecond *= 1 + HpGutsCoef / sqrt(G*HpGutsCoefSqrt)
lossThisTick = lossPerSecond * effectiveDt
```

Ground rates for turf are 1,1,1.02,1.02; dirt 1,1,1.01,1.02.
The state-rate selector distinguishes ordinary running, pace-down, rushing,
front competition and rushing plus competition (including Runaway variants).
Downhill multiplies the selected rate. Do not multiply independent rushing and
competition rates when the implementation selects their combined entry.
Recovery/cost skills change HP through their effect receivers; a negative
recovery is a cost, not evidence of a successful healing activation.

## 5. Last-spurt planning

The ordinary HP policy and the reconstructed search are different algorithms.
The latter is fully specified in [`retail_spurt.rs`](../src/retail_spurt.rs):

1. Budget remaining distance as `max(D-position-60,0)`.
2. If HP cannot pay `remaining/baseSpeed * consumption(baseSpeed)`, return
   failure result 4. If maximum speed is affordable, start now (result 1).
3. Otherwise search sprint distance downward in 30 m increments. Each candidate
   pays `slowTime*baseCost + fastTime*sprintCost`, and must start before D-100.
4. Reduce speed in .1 m/s steps, with two separate loops bounded to 101 attempts.
   No feasible candidate gives result 8.
   Travel times are float32. Store the normal-pace cost `slowTime*baseCost`
   as float32, then widen it before adding the double-precision
   `fastTime*sprintCost` product. Compare that wider sum against current HP.
   This rounding point can admit or reject a plan at an HP boundary and change
   the ordering of feasible plans even when the random draws are unchanged.
5. Sort feasible candidates by a comparison based on truncating
   `(timeA-timeB)*10000` to integer. Sort implementation/ties matter.
6. In sorted order accept when an integer draw in [0,100) is strictly below
   `15+.05*W`; if all fail, use the last candidate (result 2).

Recovery can mark a retry pending. A later unsuccessful retry does not erase
a valid existing plan; start retention is handled by `SpurtState::update`.

## 6. Integration, geometry and finish time

Default movement uses the solver's velocity-Verlet-style sequence; the
reconstructed path deliberately rounds intermediate speed and movement values
to float32. See `step_forward` and `step_forward_retail`, not just the target
equations, when reproducing a trajectory.

With geometry enabled, sampled course transforms map longitudinal distance
and lateral position into world position. Actual world displacement can limit
allowed course advancement. Lane width, curve/slope segments, quaternion
interpolation and rounding affect the correction; track distance is not simply
world-vector length. [`retail_geometry.rs`](../src/retail_geometry.rs) and the
bundled course geometry supply those operations and assets.

For a crossing between adjacent tick positions x0 and x1:

```text
finish = frameTime - clamp((x1-D)/(x1-x0),0,1)*dt
```

Only the actual adjacent crossing ticks are valid. Interpolating between
downsampled replay frames changes the answer. Finish-time error, exact rank
agreement and frame-by-frame position agreement measure different properties.

## 7. Precision and boundaries

The runtime is mixed precision. Base target speed, minimum speed, blocking
rates, spurt speed, initial HP, acceleration, slope correction and spurt
feasibility have explicit float32 store boundaries and wider intermediate
expressions. The exact expression order is in `retail_physics.rs`.

Inward passing corridors also retain a wider final boundary comparison.
`retail_lane.rs` stores the two `lane +/- corridorHalfWidth` edges as
float32, then subtracts the inward offset without another float32 store
before comparing peer lanes. Lane candidates and ordinary corridor bounds
retain their existing float32 stores.

Geometry interpolation retains unrounded x/z differences. Corner-end handling,
Spot Struggle bonuses after slope correction, and skill-duration extension
order can affect boundary outcomes. Spurt affordability uses double-precision
HP products and sums. Finished runners have a minimum speed of 5 m/s.
A still-racing exhausted runner blocked by a finished runner can resolve to
the target minimum 13 m/s; this differs from the finished runner's minimum 5.
