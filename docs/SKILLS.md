# Skills and conditions

## Data and preparation

[`race_solver_builder.rs`](../src/race_solver_builder.rs) interprets
`data/skill_data.json`, or the request's replacement `skillData`. A skill can
contain alternative detail branches, prerequisites, condition expressions,
effect arrays, activation lottery flags, cooldowns and duration/value usages.
Skill level and unique-skill metadata accompany a runner's skill ID. Bundled
tables are a snapshot, not a live master database.

The parser in [`condition_parser.rs`](../src/condition_parser.rs) builds
expressions; [`activation_conditions.rs`](../src/activation_conditions.rs)
binds named checks. Static filters use course, roster and raw/base-stat facts.
Dynamic checks use current position, cached phase/rank, HP, lane, overtaking,
blocking, competition and skill history. [`region.rs`](../src/region.rs)
represents eligible distance intervals. OR alternatives retain their own
regions and prerequisites rather than sharing one global eligibility flag.

[`activation_sample_policy.rs`](../src/activation_sample_policy.rs) selects
activation locations for sampled conditions. A chosen location still needs
its runtime predicates. Recognizing a condition name does not establish that
all of its game semantics are implemented; inspect that name's handler.

## Lottery and order

The conventional wisdom probability is `max(.2,1-90/baseWisdom)` for skills
whose definition requests it. At wisdom 900 that is .9. `wisdomChecks:false`
disables those checks for controlled experiments. Retail initialization uses
its own ordered lottery/trigger construction. Random-skill reservations can
override a failed ordinary lottery; the same seed on different paths is not
expected to imply the same sequence.

Runtime processing expires effects, evaluates armed/pending skills, checks
cooldowns and prerequisites, then applies activations according to priority
and field order. A skill ID, a detail branch, an additional activation and an
individual recipient are different identities. The output records each where
supported. Per-runner activation history feeds subsequent prerequisites,
recovery counts, green-tag counts and overtake-related additional activations.

## Effect values

Master amounts are converted with their usage and level multiplier; ordinary
fixed effect values use the master's 1/10000 scale. Level curves differ for
target speed, acceleration, stats and other effects. The exact arrays are
`LEVEL_MULTIPLIER_*` in the builder. The reconstructed value converter preserves
separate float32 rounding of the master conversion and level multiplication.

Dynamic usages are not all simple multipliers. Team-stat, fans and career-win
thresholds are prepared in the builder. Raw-stat conditions must not use the
career +400 adjusted stat. Runtime random usages 8/9 use values 0/.02/.04 with
weights 60/30/10; an amount -10000 with these usages is a random 0/2/4% cost,
not automatically a fixed full-HP loss. Usage 14 depends on distinct activated
green-tagged skills. See `apply_activation` for dispatch and operation order.

Effects include base stats, target speed, current speed, acceleration, HP,
vision, lane preference/movement, rushing-related changes and random skill
activation. A target-speed effect changes the desired speed; a current-speed
effect alters the current-speed receiver. They are not interchangeable.
Stat effects refresh relevant state according to the chosen runtime; some
ordinary-path base arrays were precomputed at construction.

## Recipients and timing

Self, field and selected recipients are dispatched through the field layer,
with team/ally exclusions and target immunity checks where implemented. Target
selection can itself consume RNG before ability-value randomization.
`skillTargetEvents` records selections per ability; additional events can have
a runtime-subset ability index scope rather than the original full array.

Ordinary distance-scaled durations/cooldowns use the course distance in
kilometres. Special duration usages are evaluated from their runtime inputs,
including remaining HP and leader distance, rather than always using the
nominal base duration. Negative duration is permanent. Expiration removes its
receiver, and cooldown bookkeeping can begin on expiry according to the
branch. Repeated corner windows and second-stage cooldowns are not identical
to a once-per-race flag.

Ordered float32 modifiers are recalculated from surviving receivers after
expiry. Removing an effect by subtracting its old value from an accumulated
total can round differently. Preserve source order for faithful reproduction.

## How to investigate a skill

1. Find its ID in the bundled or supplied skill table, including every detail.
2. Resolve condition names in `activation_conditions.rs` and sampling policies.
3. Follow `build_skill_data`, value/level conversion and `prepare`.
4. Follow `process_pending_*`, `apply_activation`, recipients and expiry.
5. Run a fixed-input simulation with full frames and inspect detail/target
   events together with positions and HP.

A successful unit test shows that the tested implementation behavior works.
It does not prove the official game handles every branch the same way. The benchmark compares skill starts rounded to 0.1 ms, not all effect values,
recipients or durations.

## Order-up extension

Receivers age in PreUpdate. Ability-time usage 4 can extend a running detail
and its surviving abilities by `OrderUpAddAbilityTime*D/1000`, up to
`OrderUpAddAbilityTimeMaxCount` (3) increments. The engine applies the extension after
receiver aging but before running-detail expiry. It must not revive an already
expired main receiver merely because the detail survives. Earlier and newly
added order-up receivers can remain.
