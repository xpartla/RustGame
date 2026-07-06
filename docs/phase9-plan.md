# Phase 9 Plan — Content Pass: Remaining Classes, Real Rosters & Bosses

_Written 2026-07-06, after Phase 8. Plan portion first; **as-built notes** get appended after each
sub-phase lands (the template every phase doc follows). Source of truth for scope, decisions, and the
work breakdown. Read `docs/architecture-plan.md` §3 (all subsystems), §5 (scalability check), §8.1
(mechanics with no home), §8.2 (Phase-9 correction: "split into content pass vs. boss design"), §8.5
(the last open debt row), and `Mechanics.md` (the full class/enemy/theme design) first._

---

## 0. Scope

The migration plan's §7 "Phase 9" is three lines:

> 1. Add remaining heroes (each is one RON file + ability/talent RONs).
> 2. Fill in enemy ability kits per theme + the real per-theme rosters.
> 3. Multi-phase boss AI for boss rooms + act boss fights.

A read of `Mechanics.md` against the shipped engine shows those three lines hide **~6 workstreams**,
and most need **new engine code, not just data** — the architecture plan already flagged this
(§8.1(10) "multi-phase boss design … realistically its own phase, not one Phase 9 line"; §8.2
"split into content pass vs. boss design"). What is genuinely missing today:

**Missing ability behaviors** (`BehaviorRegistry` has only `melee_cone`, `projectile`, `self_nova`,
`contact_melee`, `dropped_zone`):
- `summon` — **the DK's level-1 Companion is currently inert** (its `behavior: "summon"` is
  unregistered → a no-op `AbilityInstance`); also Druid's Spawn Ent.
- `leap_to_target` — Druid Ferocious Bite / Primal Pounce.
- `channel_while_moving` — Druid Heal, Paladin Flash of Light, Mage Frost Impale.
- `orbiting` — Paladin Spinning Hammer.
- the `InputSlot::Movement` dash (Mechanics lists Shift/Space) — nothing implements the Movement slot.

**Missing engine systems** (§8.1 still-open rows): shields/absorbs (5) — bone shield, ice barrier,
Paladin overheal, Purgatory immunity; forced movement (6) — Abomination grip, knockback shockwaves; a
**class-resource / charge model** (Mage frost charges, Druid enhanced/combo charges — `ResourceModel`
only has `None`/`HealthBased`); the Druid **enhanced-attack state machine**; **holy-mark consumers**
(the status exists, nothing reads it); and the general **crit % / attack-speed** stat sheet (§8.1(4)
deferred the general sheet).

**Missing AI:** a `Boss` behavior — the `AiBehavior` enum has `MeleeChaser`/`RangedCaster`/`Stationary`
only; the "boss" hook was always TBD.

**Content volume:** four class kits to finish (BDK and Mage are only partially built — see below), not
"two new heroes"; ~20 new abilities + ~80–100 talents; and 5 themes × ~5 enemies (~26) + ~15 bosses to
replace the single `warlord` placeholder.

**Plus** the last open §8.5 row — per-hero `base_stats` application (`spawn_player` still uses the
shared `PLAYER_HEALTH`/speed constants, so the DK plays with 100 HP, not its designed 200) — deferred
out of Phase 8 as "a second golden regen + a balance call, → Phase 9".

### The four classes are all unfinished, not two

`Mechanics.md` designs four heroes; only fragments ship:

| Class | Shipped today | Missing (this pass) |
|---|---|---|
| Blood Death Knight | Death Strike, D&D (zone), Blood Boil (auto-nova), AMZ (block zone). **Companion inert.** | Companion (summon), Heart Strike, Abomination Limb (forced-move grip), Purgatory (cheat-death/immunity), the class-passive + per-ability talents, base_stats. |
| Mage | Fireblast, Frostbolt, Fire/Ice stance-swap statuses. | Blaze, Flamewrath, Flamestrike, Frostbite passive, Frost charge (charges), Frost Impale (channel), the cross-cutting fire/frost talents. |
| Paladin | Consecrated Ground demonstrator only (no hero). | The whole hero: Hammer of Justice, Flash of Light (channel+overheal shield), Spinning Hammer (orbit), Smite (holy mark), holy-mark consumers, talents. |
| Druid | Bleed status, Scratch demonstrator, Tree Conduit marker (no hero). | The whole hero: forms, enhanced-attack state machine, Scratch/Ferocious Bite/Primal Pounce (leap), Roots/Heal (channel)/Tree Conduit/Bloom/Spawn Ent (summon), talents. |

So the honest scope is **"finish all four class kits + real enemy/boss content + the deferred engine
primitives,"** which is why it is delivered as an arc, not one landing.

### Owner decisions (resolved 2026-07-06, before implementation)

| # | Decision | Choice | Consequence |
|---|---|---|---|
| **D1** | Phase-9 shape | **Ordered sub-phase arc (9.1–9.7)** — plan the whole arc; implement/test/document each sub-phase green and separately, like Phases 3–8. Owner approves each in turn. | Lowest risk; the test ladder stays green throughout; each golden-master move (if any) is cleanly attributable to one sub-phase. |
| **D2** | Class scope & order | **Both new heroes, Paladin → Druid.** (And, by extension, finish the BDK + Mage kits — see the table above.) | Paladin de-risks the shared engine work (shields, orbiting, holy-mark, channel) before Druid's heavier machinery (forms + enhanced-attack state + summons + leap). |
| **D3** | Boss AI ambition | **Multi-ability "elites" now; scripted multi-phase later.** Real theme bosses get 2–3 abilities on existing AI in 9.6; the data-driven `Boss` phase machine is scheduled as the arc tail (9.7, a natural Phase-10 seam). | Matches §8.1(10). Keeps 9.6 shippable without the largest new system blocking the roster. |
| **D4** | Golden-master policy | **Allow declared regeneration(s) + a rebalance pass.** Apply per-hero `base_stats` (§8.5) and activate the DK Companion; treat balance as tuning. Each regen is isolated + CHANGELOG-declared (as Phase 8's 8A was). | The arc's **only** golden-master movement is confined to **9.2 (BDK closeout)**; every other sub-phase re-verifies byte-identical against the new baseline, exactly like Phases 4–7.5. |

### Proposed defaults for finer decisions (confirm or override during 9.1)

These are lower-level and do not reshape the arc; the plan assumes the default and flags each so the
owner can veto without a re-plan:

- **DP1 — Complete BDK + Mage kits, don't just add Paladin/Druid.** The content pass finishes all four
  classes (the table above). _Default: in scope._ (Alternative: ship only the two new heroes; leave
  BDK/Mage partial. Not recommended — it leaves two half-classes.)
- **DP2 — Engine primitives land "with their first consumer," but genuinely shared ones are batched in
  9.1.** Shields, forced movement, charges, crit/attack-speed, and the dash are cross-cutting, so 9.1
  builds them once (each inert until content uses it). Class-*specific* behaviors (`orbiting`, `leap`,
  `channel`, `summon`, enhanced-attack) land in the class sub-phase that first needs them. _Default: as
  described._
- **DP3 — Charges/enhanced state are transient (not serialized).** Resume restores at a room boundary
  (Phase 8's model), and charges/combo are mid-encounter state, so they reset on resume — no new
  `RunState` fields. _Default: transient._
- **DP4 — All 5 themes get real rosters in 9.6** (not a 1–2-theme slice), since D1 plans the whole arc.
  Bosses are multi-ability elites (D3). _Default: all five._
- **DP5 — Crit rolls consume `RunRng`.** Any random crit chance must draw from `RunRng`, not
  `thread_rng`, or it breaks the reproducibility contract (docs/testing.md). Deterministic crits
  (Ferocious Bite vs. bleeding) need no roll. _Default: RunRng for random crit._

Standing constraints unchanged from the architecture plan: **no meta-progression** (§6 Q2 — power
resets each run; talents/abilities are the only power), **local-only persistence** (§6 Q3), **DK/Paladin
have no Q** (§6 Q4), and **audio/art stay out of scope** (§8.1(10)).

### Definition of done (whole arc)

1. All four class kits are playable end-to-end (every `Mechanics.md` ability for BDK/Mage/Paladin/Druid
   either implemented or explicitly deferred with a trigger, like every prior phase's §7).
2. The five themes spawn their **designed** enemy rosters + multi-ability bosses; the `warlord`
   placeholder is retired.
3. Every new engine primitive (shields, forced movement, charges, crit/attack-speed, dash, `summon`,
   `orbiting`, `leap`, `channel`, enhanced-attack) has a golden scenario for its mechanic.
4. The golden master moves **only in 9.2** (declared base_stats + Companion + rebalance regens); every
   other sub-phase is byte-identical vs. the then-current baseline. `campaign_is_reproducible_within_a_build`
   stays green throughout. Build stays warning-free; `/compat-check` green after each sub-phase.
5. Docs updated per sub-phase: this plan's as-built notes, architecture §8.12+ (one subsection per
   sub-phase), CHANGELOG, `docs/testing.md`, `Mechanics.md`, `CLAUDE.md`, and `MEMORY.md`.

---

## 1. The arc at a glance

Each row ships independently, ends green, and updates docs. Golden-master column is relative to the
baseline **at the start of that sub-phase**.

| Sub-phase | Deliverable | New engine | Golden master |
|---|---|---|---|
| **9.1 Foundations** | Shared primitives: shields/absorbs, forced movement, class-resource/charges (+HUD bar), crit%/attack-speed sheet, `InputSlot::Movement` dash. Each inert until content uses it. | `Absorb`, `ForcedImpulse`, `Charges`+`ResourceModel::Charges`, crit/AS in `resolve`/damage, `blink` behavior. Register `bone_shield_on_kill` hook. | **byte-identical** (nothing on the DK campaign uses them) |
| **9.2 BDK closeout** | Finish the Blood Death Knight: apply `base_stats`; `summon` behavior + activate Companion; Heart Strike, Abomination Limb (forced-move), Purgatory (immunity/shield); the BDK talent trees. **Rebalance pass.** | `summon`; wire forced-move/shield consumers. | **REGENERATED (declared)** — base_stats + Companion + tuning; isolate each regen for attribution |
| **9.3 Paladin** | New hero: Hammer of Justice, Flash of Light (channel + overheal shield), Spinning Hammer (orbit), Consecrated Ground, Smite (holy mark) + holy-mark consumers; talents. | `orbiting`, `channel_while_moving`, holy-mark consumers. | **byte-identical** (runless-neutral, like Phase 4–7.5) |
| **9.4 Druid** | New hero: forms, enhanced-attack state machine, Scratch/Ferocious Bite (leap)/Primal Pounce (leap), Roots/Heal (channel)/Tree Conduit consumer/Bloom (pickup)/Spawn Ent (summon); talents. | `leap_to_target`, enhanced-attack state, Bloom pickup kind, Ent minion AI. | **byte-identical** |
| **9.5 Mage completion** | Finish the Mage: Blaze, Flamewrath, Flamestrike, Frostbite passive, Frost charge (charges), Frost Impale (channel), cross-cutting fire/frost talents. | Charge generation/consumption content; reuse `channel`. | **byte-identical** *(watch Frostbolt — the campaign bot casts it; a Frostbolt behavior change is a declared regen)* |
| **9.6 Rosters + elite bosses** | Real per-theme enemy rosters (5×~5) with distinct abilities + AI (incl. enemy DoT kits); multi-ability theme bosses (2–3 abilities). Retire `warlord`. | Any new enemy AI/ability behaviors the rosters need; enemy status appliers. | **byte-identical** (campaign is runless) |
| **9.7 Scripted boss phases** _(arc tail / Phase-10 seam)_ | Data-driven `Boss` AI: health-gated phase transitions + telegraphed specials, layered on the 9.6 bosses. | `AiBehavior::Boss` + `BossPhaseSpec`. | **byte-identical** |

**Sequencing rule (mirrors Phase 8's "8A alone"):** land **9.2's declared regens in isolation** —
base_stats first (a pure HP/speed number change), then Companion (attributable to the new active
summon), then any tuning — regenerating + committing the baseline with a CHANGELOG entry per step, so
`git log tests/golden/` stays a clean audit trail. Every sub-phase after 9.2 re-runs the full ladder
and must be byte-identical vs. that new baseline; a second unexplained move is a red flag to investigate,
not to accept.

---

## 2. Sub-phase 9.1 — Shared engine foundations

The cross-cutting primitives the classes + bosses need. **All land byte-identical** because no ability
or talent on the DK golden campaign exercises them (the campaign bot is BDK without bone shield, without
a movement binding, HealthBased). Each is a small, testable addition following an existing pattern.

### 2.1 Shields / absorbs (§8.1(5))

A generic damage-absorbing pool consumed before health. The `bone_shield_on_kill` Post-hook plumbing
already exists (registered-but-inert since Phase 6) — this makes it (and Ice Barrier / Paladin overheal
/ Purgatory) real.

```rust
// core/components.rs
/// A damage-absorbing pool. Consumed by apply_damage BEFORE Health; a hit larger than the pool
/// spills the remainder to Health. Multiple sources stack additively into one component (or one
/// entity per source — decide in 9.1a; single-component is simpler and enough for the shipped kit).
#[derive(Component, Default)]
pub struct Absorb { pub amount: f32 }
```

- **Integration point:** `core/systems/apply_damage.rs`. Today it subtracts `DamageEvent.amount` from
  `Health` (after the `DamageTakenModifier`). Insert an absorb-drain step **between** the modifier and
  the health write: `let after = drain_absorb(&mut absorb, amount); health.current -= after;`. Order is
  a scheduling contract (docs/testing.md) — absorb sits inside `CombatSet::Apply`, no new set.
- **Grant paths:** `GainShieldEvent { target, amount }` (or a direct component insert). Consumers:
  - `bone_shield_on_kill` (register the hook in `AbilityPlugin::build`; it counts Death-Strike kills via
    per-ability state and grants a 1-hit absorb at the threshold — architecture §4 Talent 2).
  - Paladin Flash of Light "overheal → shield" talent (heal past max spills into `Absorb`).
  - Mage Ice Barrier: replace the Phase-4 damage-reduction *status* stand-in with a real next-hit absorb
    (the Phase-4 note already flags this as a placeholder). _Optional in 9.1; can wait for 9.5._
- **Purgatory (cheat death + immunity):** model immunity as a large/timed `Absorb` or a dedicated
  `Invulnerable(Timer)` checked in `apply_damage`. Decide in 9.2 (BDK) — the primitive is the same drain
  hook. Cheat-death (restore to 5% on lethal) is a lethal-damage interceptor in `apply_damage`.
- **Golden master:** byte-identical — no campaign entity carries an `Absorb`.
- **Tests:** `tests/shields.rs` — "a shielded actor takes 0 health damage until the pool is spent, then
  spills"; "bone shield grants after N Death-Strike kills and blocks exactly one hit"; unit test
  `drain_absorb` math.

### 2.2 Forced movement (§8.1(6))

Pull (Abomination Limb grip) and knockback (shockwave talents).

```rust
// core/components.rs
/// A one-shot positional impulse applied over a short time, then removed. Distinct from Velocity so
/// AI/flow-field movement doesn't fight it. Resolved in MovementSet::Integrate.
#[derive(Component)]
pub struct ForcedImpulse { pub target: Vec2, pub speed: f32 }   // grip: move toward target
// or { pub velocity: Vec2, pub timer: Timer } for knockback — pick one shape in 9.1b.
```

- **System:** `resolve_forced_movement` in `MovementSet::Integrate` (positions settle before combat).
  Overrides/short-circuits the flow-field velocity while active. Respects `TileMap` blocking (reuse the
  axis-decomposed slide from `core/systems/movement.rs`).
- **Applied by:** a `leap`/grip ability behavior (9.2/9.4) or a talent hook (knockback shockwave).
- **Golden master:** byte-identical.
- **Tests:** `tests/forced_movement.rs` — "grip pulls an enemy toward the caster"; "knockback pushes and
  stops at a wall."

### 2.3 Class resource / charges

`ResourceModel` currently only has `None` / `HealthBased`. Add a charge model for Mage frost charges +
Druid enhanced/combo charges. The HUD already renders a `ClassResource` slot (`ui/screens/hud.rs`) — it
just needs data.

```rust
// hero/assets.rs
pub enum ResourceModel {
    None,
    HealthBased,
    /// A capped integer charge bar (Mage frost charges, Druid combo/enhanced charges). `max` and the
    /// generation/consumption rules are content — the component just holds the count.
    Charges { max: u32 },
}
// hero/components.rs (or a new resource module)
#[derive(Component)]
pub struct Charges { pub current: u32, pub max: u32 }
```

- **Generation/consumption:** driven by ability `effects` (a new `EffectSpec::GainCharge`/`SpendCharge`
  variant) or by hooks. Frostbolt-on-frostbitten → +1 charge; Frost Impale consumes all charges and
  scales damage per charge (a Pre hook reading `Charges`). Enhanced-attack "charges" for Druid are the
  same mechanism.
- **Transient (DP3):** not serialized into `RunState`; reset per encounter/resume.
- **Golden master:** byte-identical (DK is HealthBased; no campaign ability grants charges).
- **Tests:** `tests/charges.rs` (or folded into `tests/mage.rs`/`tests/druid.rs`) — "an ability grants a
  charge, capped at max"; "a spender consumes all and scales."

### 2.4 Actor stat sheet: crit % + attack speed (§8.1(4))

The general passives ("Gain X% crit", "Gain X% attack speed") and Ferocious Bite's crit-on-bleed need a
crit path and a cooldown-rate path. `resolve_params` already stacks modifiers; extend the *damage
application* and *cooldown* sides:

- **Crit:** a resolved `crit_chance` + `crit_mult` on a cast; on application, roll `RunRng` (DP5 — never
  `thread_rng`) and multiply damage. Deterministic-crit abilities (bleeding target) skip the roll.
  Add `crit_chance`/`crit_mult` as recognized param keys so talents feed them through the existing stack.
- **Attack speed:** a `cooldown_rate` multiplier applied to `resolved_cd` in `execute.rs`
  (`effective_cd = resolved_cd / (1 + attack_speed)`). This also finally exercises the `Override(0)`
  cooldown-guard debt row (§8.5) if a talent zeroes a cooldown — resolve that guard here (its owning
  trigger is "the first cooldown-manipulating talent").
- **Golden master:** byte-identical — the campaign bot has no crit/attack-speed talent, so the crit roll
  never fires and `cooldown_rate` defaults to 0.
- **Tests:** unit — crit multiplies on a forced roll; attack-speed shortens cooldown; `Override(0)`
  behaves. Scenario — a crit talent changes observed damage under a fixed seed.

### 2.5 Movement / dash behavior (`InputSlot::Movement`)

Mechanics lists Shift/Space as a dash. The slot exists in `HeroDef.stance_slots` (the `movement` field)
but nothing consumes it. Add a `blink`/`dash` behavior (a short `ForcedImpulse` along `Facing`) and wire
the Movement input in `hero/systems/input_slot.rs`.

- **Golden master:** byte-identical (no shipped hero binds `movement` on the campaign path; the DK RON
  leaves it `None`).
- **Tests:** `tests/hero_stance.rs` extension — "the movement slot triggers a dash."

### 2.6 Definition of done (9.1)

All five primitives compile warning-free, each with its scenario/unit test, `/compat-check` green,
golden master **byte-identical**. Docs: architecture §8.12 "Phase 9.1 delivered" + CHANGELOG "Phase 9.1"
(each primitive declared as a *new capability that is inert until content uses it*).

---

## 3. Sub-phase 9.2 — Blood Death Knight closeout (the arc's only declared regens)

Finish the BDK kit and pay the two deferred DK-campaign-affecting debts. This is the **only** sub-phase
that moves the golden master.

### 3.1 base_stats application (§8.5 — declared regen #1, isolated)

- `spawn_player` (and `respawn_player`, `resume_run`) read the active `HeroDef.base_stats` and set
  `Health::new(max_health)` + the move-speed source, instead of the shared `PLAYER_HEALTH`/`PLAYER_SPEED`
  constants. The HeroDef is async-loaded, so mirror the `grant_level_1_abilities` deferral pattern
  (apply once the HeroDef resolves; guard with a marker) — or seed with the constant and correct on
  HeroDef availability. **Land this alone**, regenerate `campaign_baseline.ron`
  (`UPDATE_GOLDEN=1 cargo test --test golden_campaign`), commit with the CHANGELOG entry (DK 100→200 HP).
- Verify `campaign_is_reproducible_within_a_build` stays green (a pure number change, no new
  nondeterminism).

### 3.2 Companion via `summon` (declared regen #2, isolated)

- Implement the `summon` behavior: on cooldown, spawn a short-lived **Friendly minion** entity carrying
  its own `AbilityInstance` for a mimicked ability (Companion mimics Death Strike with the companion's
  *own* stats — `companion.ability.ron` already declares them). Reuse the flow-field follower for the
  minion's movement (Friendly minions chase Hostiles) and the faction-aware execute path for its attack.
- Resolve the `mimicked_ability_id` string-param problem: `companion.ability.ron` has a
  `"mimicked_ability_id": 0.0` f32 hack with a TODO. The clean fix is a **typed summon spec** on
  `AbilityDef` (like `zone: Option<ZoneSpec>`) — e.g. `summon: Option<SummonSpec { mimic: AbilityId,
  duration, .. }>` — so the summoned ability id is a real string, not a float. This mirrors the Phase-6
  `ZoneSpec` precedent and removes the last stringly-float hack.
- Reap minions on expiry + on encounter teardown (extend `despawn_encounter_entities`, mirroring the
  Phase-8 orphan-instance fix — minions carry `AbilityInstance` children/owned entities too).
- **Regen #2:** the DK campaign bot now has an active Companion → more DPS → different trace. Regenerate,
  commit, CHANGELOG.

### 3.3 Remaining BDK kit (reuses 9.1 primitives)

Now that shields/forced-move/charges exist, finish the class:
- **Heart Strike** (band 2/3) — melee, hits N nearest, damage scales with missing health (a Pre hook
  reading the caster's `Health`).
- **Abomination Limb** (band 4/6) — periodic **grip** = `ForcedImpulse` pull on the nearest/ranged
  enemy; talent adds stun (existing status) + extra targets.
- **Purgatory** (band 4/6) — cheat-death interceptor in `apply_damage` + timed `Invulnerable`/`Absorb`.
- **BDK talent trees** — the per-ability + class-passive talents from `Mechanics.md` (mostly numeric
  `Modifier`s = pure data; the few behavior ones = hooks). Wire them into each ability's `talent_pool`
  and `HeroDef.class_passive_pool` so the offer generator picks them up automatically.
- These are band/talent content the campaign bot **may** roll, so land them **after** the two isolated
  regens and re-verify: if the bot's script now rolls Heart Strike/etc., that is an expected, declared
  trace change — regenerate once more with a CHANGELOG note, or (preferred) keep the bot's scripted
  build fixed so these stay runless-neutral. Decide the bot-script policy at 9.2 start.

### 3.4 Rebalance pass

With base_stats live (DK 200 HP) and the full kit, do a **tuning pass** on DK numbers (damage, cooldowns,
leech, enemy scaling feel) using the headless harness. Every tuning change is a declared CHANGELOG entry
+ (if it touches the campaign trace) a regen. Keep tuning changes batched and attributable.

### 3.5 DoD (9.2)

Full BDK kit playable; base_stats applied; Companion active; golden master regenerated **only** for the
declared DK changes, each isolated; reproducibility green; `/compat-check` classifies every divergence
as one of the declared changes and nothing else. Docs: architecture §8.13 + CHANGELOG "Phase 9.2" +
`Mechanics.md` BDK section flipped to implemented + §8.5 base_stats row marked **RESOLVED** (closing the
last register row).

---

## 4. Sub-phase 9.3 — Paladin (first new hero)

Runless-neutral (the new hero is never on the golden campaign, like the Mage was in Phase 4). New
behaviors: `orbiting`, `channel_while_moving`, and holy-mark consumers.

- **`heroes/paladin.hero.ron`** — `has_stance: false` (§6 Q4, no Q), `ResourceModel::None`, base_stats,
  L1 abilities (Hammer of Justice, Flash of Light), band pools (Consecrated Ground / Spinning Hammer /
  Smite at 2/3/4), class passives.
- **Hammer of Justice** (L1 basic) — a cone-behind-primary-target variant: heavy single-target + 50% to
  a cone behind. A `melee_cone` variant or a small new behavior; talents add bounce, holy-mark shockwave
  (forced-move, 9.1), consecrated-ground explosion.
- **Flash of Light** (L1 special) — `channel_while_moving` self-heal; "overheal → shield" talent uses
  the 9.1 `Absorb`; consecrated-ground radiate talent.
- **Consecrated Ground** — already a demonstrator zone (Phase 6); wire it as the real band ability +
  its slow / per-enemy-scaling talents.
- **Spinning Hammer** — the new `orbiting` behavior: a persistent orbiting hitbox entity around the
  caster dealing periodic damage (double vs. holy-marked). Talents: extra hammer, stun, radius.
- **Smite** — applies **holy mark** (status exists) + the **holy-mark consumers** (Spinning Hammer
  double damage, Hammer shockwave). This is where the marked-target read path is built.
- **Talents** — mostly numeric data; the behavior ones (consecrated explosion, radiate) are hooks.
- **`channel_while_moving`** — a channel with a cast-time param that ticks while the caster keeps moving;
  interrupt rules TBD (default: no interrupt, just a cast-time gate). Reused by Druid Heal + Mage Frost
  Impale.
- **Tests:** `tests/paladin.rs` — orbit hits on cadence + double vs. marked; Smite applies holy mark;
  Flash-of-Light channel heals over its cast; overheal→shield. Unit: paladin.hero.ron parse, new
  ability RON parse. Golden master **byte-identical**.
- **Docs:** architecture §8.14 + CHANGELOG "Phase 9.3" + `Mechanics.md` Paladin section.

---

## 5. Sub-phase 9.4 — Druid (the hard class)

Runless-neutral. The most machinery of any hero: two forms, an enhanced-attack state machine, leaps,
summons, channel, and a pickup-driven enhancement.

- **`heroes/druid.hero.ron`** — `has_stance: true` (Animal/Human forms via Q, like the Mage's Fire/Ice),
  stance-swap casts (Scratch on →Animal, Roots on →Human, per Mechanics), base_stats.
- **Enhanced-attack state machine** — Animal attacks have an "enhanced" state (Scratch applies bleed when
  enhanced; Ferocious Bite cleaves when enhanced). Model as a `Charges`/flag consumed by the next Animal
  cast, granted by stance-swap / Bloom / Tree Conduit. Reuse the 9.1 charge primitive + a small
  per-caster `Enhanced` marker.
- **`leap_to_target`** — Ferocious Bite (jump to cursor-nearest target, crit if bleeding — uses 9.1 crit
  path deterministically) + Primal Pounce (auto-leap to highest-health enemy). A dash-to-entity +
  on-arrival effect. Reuses `ForcedImpulse`.
- **Roots** (Human basic) — `projectile` (exists) with stun/pierce/multi-shot talents.
- **Heal** (Human special) — `channel_while_moving` (reuse from 9.3).
- **Tree Conduit** — the marker zone exists (Phase 6); wire the "next Animal attack enhanced while in
  range" consumer (feeds the enhanced-attack state).
- **Bloom** — a new `PickUpKind` (extends `pickup/`): a flower that, on run-over, grants an enhanced
  charge (+ heal/speed talents). Reuses the pickup collection path.
- **Spawn Ent** — the `summon` behavior (from 9.2): an Ent minion that taunts (forces enemies to target
  it). Fiery/Earth Ent are `MutuallyExcludes` talents (the uniqueness constraint already supports this).
  Ent taunt = a targeting override in enemy AI (nearest Friendly = Ent, not player, within range).
- **Bleed** — status exists; wire Scratch/enhanced appliers + the bleed talent tree.
- **Talents** — the large Druid tree; numeric = data, behavior = hooks (enhanced-consume, bleed-transfer,
  form-swap cost).
- **Tests:** `tests/druid.rs` — form swap remaps slots + casts the swap ability; enhanced Scratch applies
  bleed; Ferocious Bite leaps + crits a bleeding target; Primal Pounce auto-leaps; Bloom pickup grants
  enhanced; Ent taunts an enemy off the player. Unit: druid.hero.ron parse, new ability RON parse.
  Golden master **byte-identical**.
- **Docs:** architecture §8.15 + CHANGELOG "Phase 9.4" + `Mechanics.md` Druid section.

---

## 6. Sub-phase 9.5 — Mage completion

Finish the fourth class. Charges (9.1) + channel (9.3) now exist. **Watch the golden master:** the
campaign bot casts **Frostbolt** (docs/testing.md), so a change to Frostbolt's *behavior/effects* is a
declared regen — but *adding new Mage abilities* (Blaze/Flamewrath/etc.) the bot doesn't cast stays
byte-identical. Keep Frostbolt's existing effect list stable; layer frost-charge generation as an
*additive* effect only if it doesn't perturb the existing Frostbolt trace (else declare + regen).

- **Frostbite passive** — status exists; wire the passive that keeps enemies frostbitten + its talents
  (3-stack, frost-charge-on-death heal).
- **Frost charge** — the `Charges` resource (9.1); Frostbolt-on-already-frostbitten → +1 charge; damage
  vs. frostbitten scales per charge (a Pre hook reading `Charges`).
- **Frost Impale** — `channel_while_moving` (reuse) that consumes all charges for a scaling icicle.
- **Blaze / Flamewrath / Flamestrike** — fire passives/special: Blaze DoT (status), Flamewrath consumes
  blaze for an explosion (a hook), Flamestrike a targeted AoE zone (`dropped_zone` at cursor, or a small
  `periodic_self_zone`/targeted-AoE behavior).
- **Cross-cutting fire/frost talents** — the damage-trade + "no longer removes blaze/frostbite" talents
  (mostly `Modifier` data + a couple of hooks).
- **Ice Barrier real absorb** — optionally replace the Phase-4 damage-reduction *status* stand-in with a
  real 9.1 `Absorb` (the Phase-4 note flags this).
- **Tests:** `tests/mage.rs` — frostbolt generates a charge on a frostbitten target; Frost Impale spends
  charges and scales; blaze DoT + Flamewrath consumes it; Flamestrike scales per blazed enemy. Golden
  master **byte-identical** (verify Frostbolt trace unchanged; declare + regen if a charge effect
  perturbs it).
- **Docs:** architecture §8.16 + CHANGELOG "Phase 9.5" + `Mechanics.md` Mage section.

---

## 7. Sub-phase 9.6 — Real per-theme rosters + multi-ability bosses

Replace the placeholder content so the campaign feels designed. Runless-neutral (the golden campaign
uses `Sim::new_arena` / explicit spawns, not the theme spawner). D4 → all five themes.

- **~26 enemies** — the designed per-theme rosters from `Mechanics.md` (Sand Dune: scorpion, vulture,
  tusken, flame demon, oil elemental; Forest: bear, wolf, corrupted ranger, hiker; Castle Ruins:
  animated armor, dancing sword, gargoyle, skeleton, banshee; Frozen Wasteland: frostbite zombie, ice
  elemental, white bear, snow troll, icy owl, sabretooth; Alpine Lakeside: stone golem, lake siren,
  mountain eagle, corrupted fisherman, mud golem). Each is an `.enemy.ron` (mostly data) + an ability;
  most reuse `contact_melee`/`ranged_caster`; a few want **enemy DoT/status appliers** (e.g. a caster
  applying bleed/frostbite — reuses `EffectSpec::ApplyStatus`, faction-aware) and maybe one or two new
  AI variants (a fleeing/kiting archer, a burrower). Update `EnemyDef::MANIFEST`.
- **~15 theme bosses** — multi-ability elites (D3): 2–3 abilities each on existing AI (a `Stationary`/
  `MeleeChaser`/`RangedCaster` with a bigger kit). Update `ThemeDef` `boss_pool`/`map_boss_pool` to the
  designed bosses; retire `warlord` from the manifest (or keep as a debug fallback).
- **Enemy DoT kits** (§8.7 deferral "enemy status/DoT kits") — land here as the ranged casters that
  apply bleed/blaze/frostbite/root to the player through the shared status path.
- **Scaling review** — with real rosters, re-check the `EnemyScaling` curves per rarity so the depth
  ramp feels right (a tuning pass; runless-neutral, uses the balance harness).
- **Tests:** `tests/roster.rs` — every theme's pools resolve to loaded enemy defs (extend the existing
  `all_five_themes_parse_and_reference_loaded_enemies` invariant to the real ids); a boss spawns with its
  full ability set; an enemy DoT applier lands its status on the player. Golden master **byte-identical**.
- **Docs:** architecture §8.17 + CHANGELOG "Phase 9.6" + `Mechanics.md` enemy/theme sections.

---

## 8. Sub-phase 9.7 — Scripted multi-phase boss AI _(arc tail / Phase-10 seam)_

The deferred ambition (D3). A data-driven boss phase machine layered onto the 9.6 bosses. Can slip to a
standalone **Phase 10** without blocking the rest of the arc.

- **`AiBehavior::Boss`** + a `BossPhaseSpec` on `EnemyDef` (or a sibling `.boss.ron`): ordered phases
  keyed on health thresholds, each enabling/disabling a subset of the boss's abilities and (optionally)
  a telegraphed special (a wind-up status + a big cast). Phase transitions emit a VFX-bus event
  (`CastVfxEvent` pattern) for the presentation layer.
- **ActBoss** gets the richest phase spec; BossRoom bosses a lighter one.
- **Tests:** `tests/boss.rs` — a boss transitions phase at a health threshold and swaps its active
  ability set; a telegraph resolves after its wind-up. Golden master **byte-identical**.
- **Docs:** architecture §8.18 + CHANGELOG "Phase 9.7" + `Mechanics.md` boss note.

---

## 9. Testing suite (definition of done per §8.3)

Every sub-phase lands with **golden scenarios for its mechanic** + unit tests, and re-runs the whole
ladder. New scenario files (one mechanic each, driven through the real systems via `Sim`):

| Sub-phase | New unit tests | New/extended scenarios |
|---|---|---|
| 9.1 | `drain_absorb`; crit multiply; attack-speed cooldown; `Override(0)` guard; charge cap | `tests/shields.rs`, `tests/forced_movement.rs`, `tests/charges.rs`, `tests/hero_stance.rs` (dash) |
| 9.2 | summon spec parse; Heart-Strike missing-health scale; Purgatory interceptor | `tests/combat.rs` (base_stats HP, Companion adds DPS), **golden regen** |
| 9.3 | paladin.hero + ability RON parse; orbit geometry; holy-mark read | `tests/paladin.rs` |
| 9.4 | druid.hero + ability RON parse; leap target pick; enhanced-state transition; Bloom pickup | `tests/druid.rs` |
| 9.5 | frost-charge gen/spend; blaze→flamewrath | `tests/mage.rs` |
| 9.6 | all real theme pools resolve; enemy DoT applier | `tests/roster.rs` (extend `tests/enemy.rs`) |
| 9.7 | boss phase-threshold transition; telegraph resolve | `tests/boss.rs` |

**Golden master policy (the contract):**
- The baseline moves **only in 9.2**, once per declared change (base_stats, then Companion, then any
  tuning), each committed with its CHANGELOG entry — `git log tests/golden/` stays a full audit trail.
- Every other sub-phase: `cargo test` passes with **no** baseline change; if a scenario asserts a tuning
  value from a RON, changing it intentionally updates the assertion in the same commit (docs/testing.md).
- `campaign_is_reproducible_within_a_build` stays green throughout — any failure = leaked
  nondeterminism (a `thread_rng` crit roll, an unordered `RunRng` consumer, iteration-order dependence);
  fix the source, never regenerate around it. **Watch this in 9.1 (crit rolls → `RunRng`, DP5), 9.2
  (summon spawn order), 9.4 (leap target selection tie-breaks), 9.6 (roster pick order).**
- `/compat-check` at the end of each sub-phase must classify every divergence as a declared change and
  find nothing else.

Balance testing (docs/testing.md "Stage 3"): 9.2's rebalance and 9.6's scaling review are the first real
uses of the sweep harness (enemies are data-driven with a scaling curve since Phase 5; encounters exist
since Phase 7). If not already built, a minimal `arena` sweep binary + `BotPolicy` can land alongside
9.2 to make the rebalance data-driven rather than eyeballed — optional, flagged.

---

## 10. Documentation updates (land with each sub-phase, same commit)

Per the non-negotiable contract (CLAUDE.md), every sub-phase updates:
- **This file** — append its "As-built" section (deviations, deferrals, per-step notes).
- **`docs/architecture-plan.md`** — a new `§8.1x "Phase 9.x delivered"` subsection; flip the relevant
  §7 Phase-9 bullet(s); when 9.2 lands, mark the §8.5 `base_stats` row **RESOLVED** (the last register
  row) and the §8.1(5)/(6) rows as resolved when shields/forced-move land.
- **`CHANGELOG.md`** — a "Phase 9.x" section (the behavior contract): each new ability/behavior/system,
  and any declared golden-master regeneration with its cause.
- **`docs/testing.md`** — add the sub-phase's scenarios to "Adding scenarios"; note the 9.2 baseline
  regens + their causes.
- **`Mechanics.md`** — flip each implemented ability/enemy/theme from designed to implemented (the inline
  `_Phase N (implemented): …_` convention already used throughout).
- **`CLAUDE.md`** (repo) — add `docs/phase9-plan.md` to the map table; update the debt highlights as each
  §8.5/§8.1 row closes.
- **`MEMORY.md`** — bump the phase pointer.
- **`src/lib.rs` / plugin registration** — register each new behavior/hook/AI in `AbilityPlugin::build`
  (`BehaviorRegistry`/`HookRegistry`) and the enemy plugin (`AiBehavior`); add new `.ron` files to the
  relevant `DefAsset::MANIFEST`.

---

## 11. Risks & mitigations

| Risk | Mitigation |
|---|---|
| 9.2's golden regen contaminated by accidental drift (not just base_stats/Companion) | Land base_stats **alone**, regen, commit; then Companion **alone**, regen, commit; then tuning. Every later sub-phase re-runs the ladder and must be byte-identical vs. the new baseline. A second unexplained move is a red flag. (Phase 8's 8A discipline.) |
| Crit / summon / leap / roster introduce nondeterminism | All randomness in gameplay systems draws from `RunRng`, never `thread_rng` (DP5). Tie-breaks (leap target, nearest-hit) are deterministic (distance then iteration order, as `nearest()` already is). `campaign_is_reproducible_within_a_build` is the tripwire — run it every sub-phase. |
| Scope creep — the arc is large and each class hides sub-mechanics | The arc is explicitly sub-phased; each sub-phase has its own DoD + tests + docs and ships independently. Defer aggressively with a documented trigger (the project's established pattern — every prior phase deferred a tail to the next). |
| Shield/absorb integration perturbs the damage pipeline | The absorb drain sits inside `CombatSet::Apply` between the `DamageTakenModifier` and the health write — a pinned order (docs/testing.md). No campaign entity carries an `Absorb`, so the pipeline change is byte-identical; the scenario tests exercise it in isolation. |
| Druid's complexity (forms + enhanced + summons + leap) balloons 9.4 | Build on the primitives proven in 9.1–9.3 (charges, leap uses `ForcedImpulse`, summon from 9.2, channel from 9.3, forms from the Mage's stance system). 9.4 is then mostly wiring + data, not new engines. If it still overruns, split Druid into 9.4a (forms + Animal kit) / 9.4b (Human kit + Ent) — the arc already tolerates this. |
| Mage completion moves the golden master via Frostbolt | Keep Frostbolt's existing `effects` stable; add frost-charge generation as a non-perturbing additive effect, or declare + regen if it changes the trace. Verify the campaign's Frostbolt casts are byte-identical before/after. |
| Boss phase machine (9.7) is the largest single new system | It is the arc tail and may slip to a standalone Phase 10 (D3 explicitly scheduled it "later"). 9.6's multi-ability bosses are already shippable without it. |

---

## 12. Out of scope (explicit, → later)

- **Meta-progression** — no currency/permanent power (§6 Q2 stands). Hero-**unlock triggers** may finally
  be defined here (Phase 8 shipped the mechanism, D3), but that is a small data/logic add, not economy.
- **Act-3 secret level** ("feats of strength", `Mechanics.md`) — still TBD/deferred.
- **Audio / art** — explicitly out until further notice (§8.1(10)).
- **Mouse input, damage numbers, minimap, tooltips, a Settings screen** — later UX/art (Phase 7.5
  carve-outs stand).
- **Projectile-vs-wall collision** — accepted-as-is by the owner (2026-07-05); revisit only if Mage/enemy
  projectile playtesting makes it feel wrong.
- **True multi-phase scripted bosses** if 9.7 is split into Phase 10 (D3 permits this).

---

_As-built notes get appended per sub-phase below, like every prior phase doc._
