# Controls

* WASD movement  
* Shift / Space for movement ability, i.e. dash  
  * _Phase 9.1 (implemented): the primitive only — Shift/Space now reach `InputSlot::Movement`, and
    a `blink` ability behavior (a short forced-movement impulse along facing) exists as an unbound
    demonstrator (`dash.ability.ron`). No shipped hero binds a real dash to the slot yet — that
    lands with whichever class's kit calls for one._
* Left Click for basic ability  
* Right Click for special ability  
* Q for stance swap

# Classes & Abilities

# Blood Death Knight

High durability, Leech, Increase damage as health lowers, melee, strong AoE, control

_Phase 9.2 (implemented): the full kit below is live for the default (only) hero. Per-hero
`base_stats` now applies too — the DK plays at its own 200 hp / 35 move speed, not the prototype's
shared player constants (architecture-plan §8.5, now empty)._

* Blood boil \- passive (X second cooldown) (Unlocked randomly at level 2/3)  
  * Periodic AoE, circle around character, leech  
    * (common) Increase damage by X%  
      * _Phase 9.2 (implemented): `blood_boil_damage_common`, Stack(3)._  
    * (common) Increase range by X%  
      * _Phase 9.2 (implemented): `blood_boil_range_common`, Stack(3)._  
    * (rare, unique\[3\]) Blood boil deals additional X% of damage to enemies based on their current health for Y seconds  
      * _Phase 9.2 (implemented as a simplification — `blood_boil_health_scaling_rare`): applies the
        existing "bleed" physical DoT status to every hit, instead of a true percent-of-current-health
        DoT (no such primitive exists in the status system yet). A targeted execute.rs special-case,
        not a generalized mechanic — Post hooks are deliberately read-only and can't emit a follow-up
        ApplyStatusEvent._  
    * (epic, unique) If an enemy affected by blood boil DoT dies, the DoT gets transferred to X additional enemies within Y range  
      * _Deferred (Phase 9.2): needs a new "on-death status transfer" mechanic (watch a death for a
        blood-boil-sourced DoT, reapply to nearby enemies) that doesn't fit any existing hook/behavior
        shape. No talent RON references it, so it stays invisible to the offer generator._  
    * (rare, unique) Blood boil has double range when cast inside D\&D  
      * _Phase 6 (implemented): a code-driven Pre hook (`blood_boil_dnd_range`) doubles Blood Boil's radius while the caster stands in the D\&D zone — the first zone-conditioned ability hook. Kept out of the offerable talent pool for now; validated by tests/zone.rs._  
* Death Strike \- Basic Attack (X second cooldown) (Unlocked at level 1\)  
  * Default attack, frontal melee cone, leech  
    * (common) Increase leech by X%  
    * (common) Increase range by X%  
    * (epic, unique) After Death Strike kills X enemies, gain bone shield that blocks 1 next attack / projectile  
      * _Phase 9.2 (implemented as a simplification): `ability::systems::bone_shield::bone_shield_on_kill`
        counts the killer's kills from **any** source, not specifically Death-Strike-attributed ones —
        `DamageEvent` carries no ability provenance to distinguish them. Grants the existing `Absorb`
        pool (Phase 9.1) via `GainShieldEvent` once the count wraps past the threshold._
* Heart Strike \- passive (X second cooldown) (Unlocked randomly at level 2/3)  
  * Melee, hit X nearest enemies within Y range, dealing Z damage,, the lower your health \- higher the damage  
    * _Phase 9.2 (implemented): a new `nearest_melee` behavior (up to `target_count` nearest within
      `range`); the missing-health damage scaling is an **innate** hook (always active, not a talent —
      `AbilityDef.innate_hooks`), up to +100% at 0 hp._  
    * (common) Hit \+1 more enemies  
      * _Phase 9.2 (implemented): `heart_strike_extra_target_common`, Stack(2)._  
    * (common) Increase range by X  
      * _Phase 9.2 (implemented): `heart_strike_range_common`, Stack(3)._  
    * (epic) Deal additional X% damage if you are under 25% health  
      * _Phase 9.2 (implemented): `heart_strike_execute_epic`, +50% damage, Exclusive._  
* Abomination limb \- passive (X second cooldown) (Unlocked randomly at level 4/5/6)  
  * Periodically grip enemy from X radius  
    * _Phase 9.2 (implemented): a new `grip` behavior wraps the Phase-9.1 `ForcedImpulse` primitive
      (its `toward_point` constructor) into a periodic auto-cast — pure crowd control, no damage._  
    * (common) Increase grip range by X  
      * _Phase 9.2 (implemented): `abomination_limb_range_common`, Stack(3)._  
    * (rare) Grip additional X targets  
      * _Phase 9.2 (implemented): `abomination_limb_targets_rare`, Stack(2)._  
    * (rare, unique) After gripping an enemy gets stunned for X seconds  
      * _Phase 9.2 (implemented): `abomination_limb_stun_rare` — a targeted execute.rs special-case
        (same reasoning as Blood Boil's health-scaling talent above)._  
    * (epic, unique) Grip only ranged enemies  
      * _Phase 9.2 (implemented): `abomination_limb_ranged_only_epic` — `Target` gained an `is_ranged`
        field (from the actor's `AiBehavior`) for this filter._  
* D\&D \- Special Attack (X second cooldown) (Unlocked at level 1\)  
  * Periodically drop an area where enemies take increased Death Strike damage and Heart Strike hits additional target \+ you heal X% more when standing inside  
    * _Phase 6 (implemented): D\&D drops a persistent "death_and_decay" zone (RMB Special). The owner-regen effect is live (heals % max health per second while inside). Still deferred past Phase 9.2: this bullet's own BASE cross-ability buffs (Death Strike/Heart Strike get stronger by simply standing in D&D, no talent required) — 9.2 landed the zone-conditioned Blood Boil range hook (Phase 6) and the separate `bdk_passive_dnd_damage_boost` **talent** (a different mechanic — a talent-gated damage tradeoff, not this baseline zone buff), but not this specific baseline effect. Needs a Heart-Strike-in-zone target-count hook + a Death-Strike-in-zone damage hook, same shape as the existing `blood_boil_dnd_range` one._  
* Purgatory (cheat death) \- passive (X second cooldown) (Unlocked randomly at level 4/5/6)  
  * _Phase 9.2 (implemented): a new core `Invulnerable(Timer)` component — `apply_damage` discards a
    hit entirely while present, before the Absorb shield even drains. A new
    `ability::systems::purgatory::purgatory_cheat_death` system (`CombatSet::Apply`, after
    `apply_damage`) reads `Health.current` **after** a lethal hit (rather than predicting lethality
    beforehand) and restores/grants immunity on a ready owned instance. Purgatory's own AbilityDef
    never fires through the normal cast pipeline — it exists only so this system can read its
    talent-modified resolved params and share `AbilityCooldown`._  
  * Restore to 5% health, immune to damage for 5 seconds, long CD (rare, unique)  
    * (rare) Increase restored health by X%  
      * _Phase 9.2 (implemented): `purgatory_restore_rare`, +2 percentage points per stack, Stack(3)._  
    * (epic, \[3\]) Increase damage immunity by X seconds  
      * _Phase 9.2 (implemented): `purgatory_immunity_epic`, +2s per stack, Stack(3)._  
    * (rare) lower cooldown by X seconds  
      * _Phase 9.2 (implemented): `purgatory_cooldown_rare`, -10s per stack, Stack(2)._  
* Companion \- passive (permanent, no cooldown) (Unlocked at level 1\)  
  * Each X seconds spawn a companion for Y seconds that is going to mimic death strike  
    * _Phase 9.2 (implemented): a new `summon` behavior spawns a `Minion` entity owning its own
      `AbilityInstance` (mimicking a standalone `companion_attack` ability, Death-Strike-shaped). The
      faction-aware ability engine needed zero changes for the minion to act as an independent
      attacker; it seeks the nearest hostile with its own straight-line logic (the shared FlowField
      is built from the player outward and is the wrong direction for a minion chasing enemies)._  
* AMZ \- passive (X second cooldown) (Unlocked randomly at level 4/5/6)  
  * Periodically drop a zone around character that blocks projectiles (rare) (if enemies emit projectiles from inside the zone it has no effect \- it acts as a barrier)  
    * _Phase 6 (implemented): base AMZ drops a fixed projectile-blocking zone that destroys enemy bolts entering it, except those emitted from inside (the barrier exception)._  
    * (common) Increase the size of the zone by X%  
      * _Phase 9.2 (implemented): `amz_size_common`, Stack(3)._  
    * (rare) Increase your movement speed by X% when inside the zone  
      * _Phase 9.2 (implemented): `amz_movespeed_rare` — a new, independent `ZoneSpeedModifier` core
        component (kept separate from the status-owned `MoveSpeedModifier` to avoid a two-writer
        race), +20% while standing inside._  
    * (rare) You regenerate X% health each second you are inside the zone  
      * _Phase 9.2 (implemented): `amz_regen_rare` reuses the existing D&D-style occupant regen
        (`ZoneEffects.regen_fraction`) purely by overriding AMZ's own params — zero new zone-tick code._  
    * (common) Increase the duration of the zone by Y seconds  
      * _Phase 9.2 (implemented): `amz_duration_common`, Stack(3)._  
    * (epic, unique) The zone gets attached to you as you move  
      * _Phase 9.2 (implemented): `amz_follow_epic` overrides a new `follow_caster` param that
        `spawn_dropped_zone` checks ahead of the ability's own static anchor, forcing
        `ZoneAnchor::Follow` — the follow-anchor mechanism itself was built + tested back in Phase 6._  
* Passive Talents  
  * (epic, unique) You can no longer heal above 35% max health, your leech is increased by 50%  
    * _Phase 9.2 (implemented): `bdk_passive_no_heal_cap` — the leech half is a Pre hook on Death
      Strike/Blood Boil; the cap half is a separate always-running clamp
      (`talent::systems::passives::enforce_heal_cap`), since it must catch every heal source
      uniformly, not just one ability's cast._  
  * (epic, unique) Your damage is lowered by 60%, your damage inside D\&D is increased by 500%  
    * _Phase 9.2 (implemented): `bdk_passive_dnd_damage_boost`, net ×2.4 inside D&D, ×0.4 outside._  
  * (rare, unique) 20% overkill damage is leeched  
    * _Phase 9.2 (implemented): `bdk_passive_overkill_leech` heals the killer for 20% of a kill's
      negative "overkill" Health.current._  
  * (common) Increase health by X% and healing taken by Y%  
    * _Phase 9.2 (implemented): `bdk_passive_health_and_healing`, +10% max health / +15% healing
      taken per stack, Stack(3) — recomputed from a new `BaseHealth` reference each time so
      re-acquiring a stack never compounds against an already-boosted value._  
  * (rare, unique) Blood boil automatically spawns D\&D zone  
    * _Phase 9.2 (implemented): `bdk_passive_blood_boil_spawns_dnd` — a targeted execute.rs
      special-case, same reasoning as the health-scaling/stun talents above._

# Druid

Human form \- ranged control, healing  
Animal form \- high damage, enhanced attacks, movement

_Phase 9.4 (implemented): the full kit below is live as its own hero (`druid.hero.ron`). `has_stance:
true` (Human/Animal via Q), `resource_model: Charges(max: 3)` (the Enhanced-attack state), base_stats
150 hp / 38 move speed (a reasonable default, same not-yet-balance-tested caveat every prior sub-phase
flagged). Unlike the Mage's swap_effect status model, entering a stance **casts that stance's own
Basic ability** (a new `cast_on_enter` flag on `StanceSlotMapping`) — Scratch on → Animal, Roots on →
Human, per Mechanics. All four Basic/Special abilities (Scratch/Ferocious Bite/Roots/Heal) are granted
at level 1, like the Mage's dual basics._

* Stance Swap \- Q (X second cooldown) (Unlocked at level 1\)  
  * Change from human to animal form and cast Scratch  
  * Change from animal form to human and cast Roots  
    * _Phase 9.4 (implemented): `cast_on_enter` — entering a stance emits a normal `TriggerAbilityEvent`
      for that stance's own Basic slot, so it respects the ability's own cooldown/aim gate like any
      other cast._  
* Bleed (passive) (passive, no cooldown) (Unlocked at level 1\)  
  * Enemies affected by bleed suffer X damage every second  
    * _Phase 9.4: the `bleed` status effect itself has existed since Phase 3 (Scratch's original
      demonstrator); this bullet's own talent tree is **deferred** — every one of its five talents
      rescales the STATUS's own magnitude (damage/duration/tick-speed/slow), and no primitive exists
      for a talent to modify a `StatusEffectDef`'s fields (the modifier stack only reaches ability
      `base_params`). A generalized "status-magnitude talent" is the trigger to revisit this._  
* Scratch (Animal) \- Basic Attack (X second cooldown) (Unlocked at level 1\)  
  * Hit all enemies in a cone in front of you for X damage  
  * Enhanced \- Scratch applies a bleed on all enemies hit  
    * _Phase 9.4 (implemented): bleed is NOT part of Scratch's unconditional `effects` (a correction to
      the Phase-3 demonstrator, which applied it on every hit) — it's the Enhanced-attack state's own
      consequence: a targeted execute.rs special-case spends one `hero::components::Charges` (if any
      are held) and applies bleed to the `bleed_target_count` nearest hits (default unlimited)._  
    * (common) Increase the size by X%  
      * _Phase 9.4 (implemented): `scratch_size_common`, Stack(3), scales `range`._  
    * (common) Increase the damage by X%  
      * _Phase 9.4 (implemented): `scratch_damage_common`, Stack(3)._  
    * (common) Scratch applies bleed to the closest X targets  
      * _Phase 9.4 (implemented): `scratch_bleed_closest_common` overrides `bleed_target_count` to 1 —
        narrows the Enhanced spread from "every hit" to the single nearest one._  
    * (rare, unique) Scratch deals X% more damage to enemies affected by root  
      * _Phase 9.4 (implemented): `scratch_root_bonus_rare` — a targeted per-hit top-up (the same
        execute.rs special-case shape as Spinning Hammer's holy-mark bonus, Phase 9.3)._  
    * (rare) Increase bleed duration by X seconds  
      * _Deferred (Phase 9.4): a status-magnitude talent — see the Bleed passive's own note above._  
    * (rare, unique) Scratch deals X% more damage to bleeding targets  
      * _Phase 9.4 (implemented): `scratch_bleed_bonus_rare`, the same top-up shape as the root bonus._  
    * (epic, unique) Scratch deals only 50% damage, bleed deals triple damage  
      * _Deferred (Phase 9.4): the "triple bleed damage" half is a status-magnitude talent (no
        primitive); the "50% damage" half alone isn't worth shipping without it._  
* Ferocious Bite (Animal) \- Special Attack (X second cooldown) (Unlocked at level 1\)  
  * Jump to the closest target near your cursor and deal X damage to the enemy. Always critically strikes if the target is bleeding  
    * _Phase 9.4 (implemented): a new `leap_to_target` behavior, cursor-nearest mode (nearest target
      within `leap_range` and `half_angle` of the caster's aim; the angle filter is skipped
      defensively with no aim, so a self-centred AutoCast can reuse the same behavior — see Primal
      Pounce). Requests a `ForcedImpulse` toward the target (the Phase-9.1 primitive) as the leap's
      visual/positional dash. "Always crits if bleeding" is BASE KIT identity (deterministic — no
      RunRng roll, matching DP5), not a talent: a flat top-up to `damage * bleed_crit_mult`._  
  * Enhanced \- Ferocious bite cleaves in a circle around you, applying X% of the damage as bleed  
    * _Phase 9.4 (implemented as a simplification): spends one Enhanced charge (if any) and applies
      plain "bleed" (not a true percent-of-damage DoT — no such primitive exists, the same
      simplification Blood Boil's health-scaling talent used in Phase 9.2) to every OTHER target
      within `cleave_radius` of the PRIMARY's landing spot (the caster's own post-leap position isn't
      known this frame — the leap's `ForcedImpulse` resolves next frame — so the primary's position,
      where the caster is about to land, stands in for "around you")._  
    * (common) Increase damage by X%  
      * _Phase 9.4 (implemented): `ferocious_bite_damage_common`, Stack(3)._  
    * (common) Increase range by X  
      * _Phase 9.4 (implemented): `ferocious_bite_range_common`, Stack(3), scales `leap_range`._  
    * (rare, unique) Ferocious bite consumes active bleed on the target, to deal X% increased damage per bleed stack  
      * _Deferred (Phase 9.4): bleed is a single `RefreshOnReapply` instance (0 or 1 present, never a
        counted stack), so "per bleed stack" doesn't map onto the current status model; a "consume and
        deal bonus" version without the stack-count language would work but wasn't judged worth
        shipping as a rename of the design._  
    * (epic, unique) If Ferocious bite kills an enemy, it resets the cooldown of your next stance swap, and grants 1 Enhanced charge  
      * _Deferred (Phase 9.4): needs per-ability kill attribution (`DamageEvent` carries none — the
        same gap Phase 9.2's bone shield and Phase 9.3's Hammer of Justice explosion talent hit)._  
    * (rare, unique) If cast while standing inside a Tree Conduit, the cleave applies bleed X times  
      * _Deferred (Phase 9.4): bleed has no stacking-count concept to apply "X times" onto (see the
        consume-bleed talent above)._  
    * (epic, unique) Ferocious bite deals no damage to the primary target, instead it deals all the remaining bleed from the current target instantly to enemies inside the cleave radius  
      * _Deferred (Phase 9.4): needs a "remaining DoT magnitude" read + instant-transfer mechanic, and
        a per-cast override to suppress the base declarative damage effect — neither exists._  
* Primal Pounce (Animal) \- Passive (X second cooldown) (Unlocked randomly at level 2/3)  
  * Every X seconds automatically leap towards the highest-health enemy within a radius, dealing X damage and applying a bleed  
    * _Phase 9.4 (implemented): `leap_to_target`'s highest-health mode (`select_highest_health: 1.0`) —
      ties broken by nearest distance, deterministic, no RunRng. Unconditional damage + bleed (not
      gated by Enhanced — Mechanics never calls Primal Pounce "Enhanced")._  
    * (common, unique \[5\]) If the target was Rooted, gain X% movement speed for Y seconds after leaping  
      * _Deferred (Phase 9.4): needs a new stacking movement-speed-buff status distinct from the
        simple Bloom Swiftness added this phase; time-boxed out._  
    * (rare, unique) Create a Bloom flower at the point you jumped from  
      * _Phase 9.4 (implemented): `primal_pounce_bloom_flower_rare` — a direct reuse of Bloom's own
        pickup-spawn primitive at the cast-time origin (the point jumped FROM)._  
    * (epic, unique) Primal pounce deals no direct damage, instead the bleed is applied to all targets in your path  
      * _Deferred (Phase 9.4): "all targets in your path" is a new line-AoE targeting shape with no
        existing analog to reuse._  
    * (common) Increase the range by X  
      * _Phase 9.4 (implemented): `primal_pounce_range_common`, Stack(3), scales `leap_range`._  
    * (rare, unique) If the target is rooted, deal triple damage  
      * _Phase 9.4 (implemented): `primal_pounce_root_triple_rare` — a flat +200% top-up (same shape
        as Ferocious Bite's bleed crit)._  
* Roots (Human) \- Basic Attack (X second cooldown) (Unlocked at level 1\)  
  * Shoot a projectile in front of you that deals X damage  
    * _Phase 9.4 (implemented): reuses the `projectile` behavior verbatim (Physical damage), same
      shape as Frostbolt/Fireblast/the Phase-3 demonstrator._  
    * (rare, unique) Projectile can pass through enemies  
      * _Phase 9.4 (implemented): `roots_pierce_rare` overrides `pierce` to 1._  
    * (common, unique) Enemies hit by roots are stunned for X seconds  
      * _Deferred (Phase 9.4): needs a talent-conditional `EffectSpec` — the existing "per-target
        conditional" special-case pattern lives in execute.rs's INSTANT-hit path, but Roots' effects
        resolve on projectile IMPACT (`projectile/systems/motion.rs`), a different code path with no
        talent/`ActiveHooks` access today._  
    * (common) Increase damage by X%  
      * _Phase 9.4 (implemented): `roots_damage_common`, Stack(3)._  
    * (rare, unique \[3\]) Shoot additional projectile towards the nearest enemy  
      * _Deferred (Phase 9.4): needs multi-projectile spawn — `CastOutcome.projectile` is a single
        `Option<ProjectileSpawn>`, not a list._  
* Heal (Human) \- Special Attack (channeled while moving) (X second cooldown) (Unlocked at level 1\)  
  * Heals you for X% max health  
    * _Phase 9.4 (implemented): reuses `channel_while_moving` (Phase 9.3) verbatim._  
    * (rare, unique) You heal for X% more per bleeding enemy within Y range  
      * _Phase 9.4 (implemented): `heal_bleed_bonus_rare` — counted at channel COMPLETION (the caster
        may have moved throughout), same reasoning as Flash of Light's radiate talent._  
    * (rare, unique) Your next attack in animal form is enhanced  
      * _Phase 9.4 (implemented): `heal_grants_enhanced_rare` grants 1 Enhanced charge on completion._  
    * (rare, unique) Your heal also heals your Ent  
      * _Phase 9.4 (implemented): `heal_heals_ents_rare` — the same flat heal amount to every owned
        `Minion`._  
    * (common, unique\[3\]) Lower cast time by X%  
      * _Phase 9.4 (implemented): `heal_cast_time_common`, Stack(3), scales `cast_time`._  
* Tree conduit (human) \- passive (X second cooldown) (Unlocked randomly at level 2/3)  
  * Spawn a tree for Y seconds, within X range of the tree, your next animal attack is enhanced  
    * _Phase 6 (implemented as a marker demonstrator); Phase 9.4 (implemented): promoted to the real
      Druid band ability (mirrors Consecrated Ground's Phase 9.3 promotion) — the mechanic itself is
      unchanged. The "enhanced next animal attack" consumer is
      `hero::systems::enhanced::tree_conduit_enhances_animal_attacks`: every frame the player stands
      inside the zone in Animal form with zero Charges, top up to one — which, since the top-up re-
      fires the instant a charge is spent, already provides continuous enhancement for as long as the
      player stands in range (see the epic talent below)._  
    * (common) Increase tree radius by X%  
      * _Phase 9.4 (implemented): `tree_conduit_radius_common`, Stack(3)._  
    * (rare) Reduce spawn range by X%  
      * _Deferred (Phase 9.4): doesn't map onto any existing mechanic — a dropped zone always spawns
        at the caster's own position; there is no "spawn range" to reduce._  
    * (rare) Increase duration by X seconds  
      * _Phase 9.4 (implemented): `tree_conduit_duration_rare`, Stack(2)._  
    * (epic, unique) All animal attacks are enhanced while in tree range  
      * _Deferred (Phase 9.4): the base consumer's per-frame top-up-to-one (above) already provides
        exactly this — "no per-attack limit while standing in range" — under the chosen model, so a
        separate talent implementing it would be a no-op on top of the base kit._  
* Bloom (Human) \- passive (X second cooldown) (Unlocked randomly at level 2/3)  
  * Periodically spawn a flower that can be picked up when ran over, upon pickup your next animal form attack is enhanced  
    * _Phase 9.4 (implemented): a new `bloom` behavior (drops a `pickup::components::PickUp` carrying
      `PickUpKind::Enhance` at the caster's position) + `collect_pickups.rs` grants
      `hero::components::Charges` on contact — unlike every other ability, the grant lands on PICKUP,
      not at cast time._  
    * (rare) After picking up you heal for X% health over Y seconds  
      * _Deferred (Phase 9.4): needs a heal-over-time primitive — `StatusEffectDef.tick` only supports
        damage, never healing._  
    * (rare, unique) Your next 2 attacks are enhanced  
      * _Phase 9.4 (implemented): `bloom_extra_charge_rare` adds +1 to `bloom_charges` (base 1 → 2)._  
    * (common) You gain X% movement speed after pickup  
      * _Phase 9.4 (implemented): `bloom_movespeed_common` — a targeted special-case in
        `collect_pickups.rs` applying the new `bloom_swiftness` status (a pure move-speed buff, same
        shape as the Mage's Boots of Fire)._  
* Spawn Ent (Human) \- passive (X second cooldown) (multiple ents can live simultaneously) (Unlocked randomly at level 2/3)  
  * Periodically spawn an Ent that runs towards the nearest enemy, forcing the enemy to attach the Ent instead of you  
    * _Phase 9.4 (implemented): reuses the `summon` behavior (Phase 9.2 — Companion) with its own body
      stats. Minion body params (`minion_health`/`minion_speed`/`minion_radius`) were generalized from
      the shared `MINION_*` constants into the summon ability's own resolved params (Companion declares
      the same numbers explicitly now — byte-identical), since the Ent needed a tankier/slower body
      than the DK's pet. The taunt itself is new: a positive `taunt_radius` param inserts an
      `enemy::components::Taunt` on the minion; `enemy::systems::taunt::apply_ent_taunt` (runs before
      the flow-field follower) marks any Hostile `MeleeChaser` within range as `Taunted`, which steers
      it straight-line toward the Ent instead of the flow field — mirroring the Companion minion's own
      straight-line seek (the shared `FlowField` only ever points toward the player, exactly wrong for
      "go fight the Ent"). Contact-range abilities needed no change: they already hit ANY opposing-
      faction target in range, not just the player. Scoped to `MeleeChaser` enemies only —
      `RangedCaster`/`Stationary` AI are untouched (a documented simplification)._  
    * (common) Ent lowers the max health of nearby enemies by X% while alive  
      * _Deferred (Phase 9.4): needs a new continuous-aura-debuff mechanic — no existing primitive
        applies a component to every enemy within range of an ARBITRARY entity (not the player)._  
    * (epic, unique\[Fiery Ent / Earth Ent\]) Fiery Ent \- Ent has 50% reduced health and explodes on death, dealing X damage to enemies around him  
      * _Deferred (Phase 9.4): needs an on-minion-death explosion hook + the whole Fiery/Earth
        `MutuallyExcludes` sub-tree below it — a substantial new mechanic, time-boxed out of this pass._  
      * (common) Increase Fiery Ent damage by X%  
        * _Deferred (Phase 9.4): depends on the parent talent above._  
      * (rare, unique) Spawn a mini Fiery Ent for each enemy killed by the explosion, mini Ents are unkillable and live for X seconds, dealing Y damage per second to nearby enemies per Ent \- adding up  
        * _Deferred (Phase 9.4): depends on the parent talent above._  
    * (epic, unique\[Earth Ent/ Fiery Ent\]) Earth Ent \- Ent has 200% increased health and casts entangling zone around himself, rooting the enemies in place   
      * _Deferred (Phase 9.4): needs a minion-owned zone (every zone today is owned by the player/an
        enemy actor, not a summoned minion) — time-boxed out of this pass._  
      * (rare, unique) After entangled enemy gets hit by roots, they transform into spiky roots, dealing X damage per second to enemies within Y range, multiplying  
        * _Deferred (Phase 9.4): depends on the parent talent above._  
    * (common) Reduce cooldown by X  
      * _Phase 9.4 (implemented): `spawn_ent_cooldown_common`, Stack(3)._  
    * (rare, unique) Ents can pick up bloom flowers, healing all summoned ents and granting them X% increased movement speed for Y seconds  
      * _Deferred (Phase 9.4): needs Ent-AI pickup-collection behavior (minions never interact with
        pickups today — only the player's own `collect_pickups` proximity check does)._  
* Passive Abilities  
  * (rare, unique) Mega bleed \- enemies can bleed from the same ability up to 3 times, each new application refreshes the old one  
    * _Deferred (Phase 9.4): needs a talent that rewrites a `StatusEffectDef`'s own `StackingRule` —
      no such primitive exists (talents only ever touch ability `base_params`)._  
  * (rare, unique) Unstable form \- your first 3 casts in animal form are always enhanced, you have a 10% chance of turning to human form after casting an enhanced ability  
    * _Deferred (Phase 9.4): "first 3 casts" needs a per-player counter distinct from `Charges`; the
      10%-chance-to-swap-form needs an `ActiveHooks`-gated random stance flip on cast — both new
      mechanics, time-boxed out._  
  * (epic, unique) Master of the Forest \- You can no longer turn into animal form, Spawn Ent spawns an additional 2 Ents, Blooming flowers explode after getting picked up by Ents, causing the flower to erupt, dealing X damage in a radius around the flower  
    * _Deferred (Phase 9.4): depends on the deferred Ent-picks-up-Bloom talent above, plus a way to
      DISABLE a stance from `handle_stance_swap` — a substantial new mechanic._  
  * (rare, unique) Swapping forms costs X% of your current health, for Y seconds after swapping forms, you deal Y% increased damage and your movement speed is doubled  
    * _Deferred (Phase 9.4): needs a "spend health as an ability cost" primitive — nothing in the
      engine currently deducts health outside the normal damage pipeline — plus a timed
      damage+speed buff on the stance-swap system itself._

# Mage

Ice form \- slow, control, damage spikes, combo  
Fire form \- DoTs, spreading damage, big AoE

_Phase 9.5 (implemented): the full kit below is live as its own hero (`mage.hero.ron`). Both basics
AND both specials are level-1 (mirrors the Phase-4 "own both stances' basics immediately" pattern,
extended to Special too). `resource_model: Charges(max: 10)` — frost charges, generated by
Frostbolt and consumed by Frost Impale. Blaze/Frostbite/Frost-charge have no standalone ability
entity (mirrors the Druid's Bleed) — nothing to unlock at band 2/3; only Flamewrath (band 4/5) is a
real unlockable ability, so `band_2_3_pool` is empty._

* Stance swap \- Q (active, no cooldown) (Unlocked at level 1\)  
  * Fire \-\> Ice \- gain ice barrier absorbing the next attack / projectile  
    * _Phase 4 (implemented as a stand-in): a \-40% damage-taken buff for 3s (a status), since the
      real absorb/shield system didn't exist yet (architecture-plan §8.6). Phase 9.5 (implemented):
      replaced with a REAL Absorb shield (40.0) via a new `swap_shield_amount` field on
      `StanceSlotMapping`, wired through the same `GainShieldEvent` path Flash of Light's
      overheal-to-shield talent uses (hero/systems/stance.rs) — resolving the §8.1 tracked debt row.
      The old `ice_barrier` status/RON is retired._  
  * Ice \-\> Fire, \- gain boots of fire, increase movement speed by Y% for X sec  
    * _Phase 4 (implemented): Boots of Fire is **\+30% movement speed for 3s** (a status), unchanged._  
* Fireblast \- fire basic attack (X second cooldown) (Unlocked at level 1\)  
  * Shoot a projectile, dealing X damage, setting enemy hit ablaze  
    * (common, unique) Projectile explodes on impact, dealing X damage to nearby enemies  
      * _Phase 9.5 (implemented): `fireblast_explode_on_impact_common` — baked into a new
        `ProjectilePayload.explode_on_impact` at cast time (an ActiveHooks flag read directly in
        execute.rs's projectile-spawn block, not a HookRegistry Pre hook) and consumed by
        `projectile_collision`: extra Fire damage to every OTHER opposing-faction actor within
        `explode_radius` of the impact point — the projectile-impact talent-effect gap the Phase
        9.4 as-built notes flagged for Mage completion._  
    * (common) Increase damage by X%  
      * _Phase 9.5 (implemented): `fireblast_damage_common`, Stack(3)._  
    * (common) Increase range by X  
      * _Phase 9.5 (implemented): `fireblast_range_common`, Stack(3)._  
* Blaze \- fire passive (passive, no cooldown) (Unlocked randomly at level 2/3)  
  * Enemies affected by blaze receive X damage every Y seconds, hitting an enemy affected by blaze with a frost spell removes blaze  
    * _Phase 3/4: the blaze status itself (DoT + `removed_by_tags: [Frost]`) has existed since
      Fireblast's own Phase-3 demonstrator, unchanged. This bullet's own talent tree is **deferred
      in full** (Phase 9.5): every one of its three talents rescales the STATUS's own magnitude/
      behavior, and no primitive lets a talent rewrite a `StatusEffectDef`'s own fields (the
      modifier stack only reaches ability `base_params`) — the identical gap Druid's Bleed passive
      tree deferred in Phase 9.4. No ability entity exists for "Blaze" itself (mirrors Bleed) —
      nothing needs unlocking; it's inherent to Fireblast._  
    * (common) Increase blaze damage by X%  
      * _Deferred (Phase 9.5): status-magnitude, see above._  
    * (rare, unique) Blaze deals double damage, enemies affected by blaze have movement speed increased by 50%  
      * _Deferred (Phase 9.5): status-magnitude, see above._  
    * (rare, unique) Blaze deals 50% reduced damage, after running out, blaze moves to a nearby target (one blaze can jump up to 3 times)  
      * _Deferred (Phase 9.5): needs a new "DoT jumps to a nearby target on expiry" mechanic, on
        top of the status-magnitude gap above._  
* Flamewrath \- fire passive (X second cooldown)  (Unlocked randomly at level 4/5)  
  * Periodically apply to the nearest target. Nearest ablaze enemy consumes blaze effect and create an explosion around himself, dealing X damage to nearby enemies  
    * _Phase 9.5 (implemented): reuses `self_nova` verbatim for "who's nearby" (its own `radius`
      param is the search range) — the ability's `effects` list stays empty; the real mechanic
      (pick the nearest ABLAZE hit, explode within `explosion_radius` of ITS position — not the
      caster's — and consume its blaze) is a targeted execute.rs special-case, since `self_nova` has
      no notion of "who's ablaze" and no `EffectSpec` expresses "explode around a point other than
      the caster."_  
    * (common) Increase cast radius by X%  
      * _Phase 9.5 (implemented): `flamewrath_cast_radius_common`, Stack(3), scales `radius` (the
        search range)._  
    * (common) Increase explosion range by X%  
      * _Phase 9.5 (implemented): `flamewrath_explosion_radius_common`, Stack(3)._  
    * (common) Increase damage by X%  
      * _Phase 9.5 (implemented): `flamewrath_damage_common`, Stack(3)._  
    * (common) Reduce cooldown by X %  
      * _Phase 9.5 (implemented): `flamewrath_cooldown_common`, Stack(3)._  
    * (common, unique) Flamewrath deals 50% reduced damage, but does not consume the blaze stack  
      * _Phase 9.5 (implemented): `flamewrath_no_consume_common` — a real Pre hook (`damage` ×0.5)
        plus a direct ActiveHooks flag read at the blaze-removal special-case that skips the
        `RemoveStatusEvent`._  
    * (rare) Flamewrath affects an additional target  
      * _Deferred (Phase 9.5): needs multi-primary targeting — no existing behavior/`CastOutcome`
        shape carries more than one independent explosion center (the same class of gap Roots'
        "extra projectile" talent hit in Phase 9.4)._  
* Flamestrike \- fire special attack (X second cooldown) (Unlocked at level 1\)  
  * Cast a fiery circle, dealing X damage to enemies within the zone, dealing increased damage per enemy affected by blaze  
    * _Phase 9.5 (implemented): a new `targeted_burst` behavior — an aimed AoE offset `cast_range`
      along the caster's aim (not centred on the caster, unlike `self_nova`/`dropped_zone`), hitting
      everyone within `zone_radius` of that point. "Increased damage per enemy affected by blaze" is
      BASE KIT identity (not a talent) — a def_id-keyed execute.rs top-up: for every blaze-affected
      hit present, EVERY hit takes extra `damage * bonus_per_blazed_percent / 100` — a global
      per-cast scaling the generic effects pipeline (one uniform amount per `EffectSpec`) can't
      express._  
    * (common) Increase cast range by X%  
      * _Phase 9.5 (implemented): `flamestrike_cast_range_common`, Stack(3)._  
    * (common) Increase zone range by X%  
      * _Phase 9.5 (implemented): `flamestrike_zone_range_common`, Stack(3)._  
    * (common) Increase damage by X%  
      * _Phase 9.5 (implemented): `flamestrike_damage_common`, Stack(3)._  
    * (epic, unique) Flamestrike deals no damage, and triggers flamewrath on all enemies hit  
      * _Deferred (Phase 9.5): a chained-explosion mechanic (each hit re-triggering Flamewrath's own
        consume+explode) with no existing primitive._  
    * (epic, unique) Flamestrike deals 80% reduced damage, and deals the remaining blaze damage on enemies instantly, removing the blaze effects  
      * _Deferred (Phase 9.5): needs a "read a status's own remaining DoT magnitude" primitive — the
        identical gap Ferocious Bite's deferred "instant remaining bleed" talent hit in Phase 9.4._  
* Frostbolt \- ice basic attack (X second cooldown) (Unlocked at level 1\)  
  * Shoot a projectile, dealing X damage to the first enemy hit, applying frostbite, if the target is already affected by frostbite, generate a frost charge  
    * _Phase 9.5 (implemented): the frost-charge generation is BASE KIT identity, not a talent — a
      `grants_frost_charge` resolved-param flag baked into a new
      `ProjectilePayload.grants_frost_charge_on_frostbitten` at cast time, checked in
      `projectile_collision` BEFORE this hit's own `ApplyStatus(frostbite)` lands (so it only fires
      against a target frostbitten by a PRIOR cast) — the projectile-impact talent/innate-effect gap
      the Phase 9.4 as-built notes flagged for Mage completion._  
    * (common) Increase damage by X%  
      * _Phase 9.5 (implemented): `frostbolt_damage_common`, Stack(3)._  
    * (common) Increase range by X%  
      * _Phase 9.5 (implemented): `frostbolt_range_common`, Stack(3)._  
    * (rare) Frostbolt pierces through, hitting an additional enemy  
      * _Phase 9.5 (implemented): `frostbolt_pierce_rare`, `Override(1.0)` on `pierce` — mirrors
        `roots_pierce_rare`._  
    * (common) Increase the projectile size by X%  
      * _Phase 9.5 (implemented): `frostbolt_size_common`, Stack(3)._  
    * (epic, unique) if an enemy affected by frostbite gets killed by frostbolt, they explode dealing X damage to nearby enemies  
      * _Deferred (Phase 9.5): needs per-ability kill attribution — `DamageEvent` carries no ability
        provenance, the same gap bone shield / Hammer of Justice's kill-explosion / Ferocious Bite's
        kill-reset talent all hit._  
* Frost Impale \- frost special attack (long channel while moving) (X second cooldown) (Unlocked at level 1\)  
  * Consume all frost charges to launch a massive icicle at a target dealing X damage, increased by Y% per frost charge  
    * _Phase 9.5 (implemented): reuses `channel_while_moving`, extended so `tick_channels` can ALSO
      fire a piercing projectile at completion (not just resolve a self-heal) — consumes every held
      `Charges` (`spend_all`), scales `damage` up by `frost_charge_damage_percent`% per charge
      spent, and fires toward the caster's CURRENT aim (read fresh at completion, like Flash of
      Light's radiate — "channeled while moving" lets the caster keep re-aiming throughout). Ships
      with `pierce: 0` (a single target, matching "a target" in the Mechanics text)._  
    * (common) Frost impale deals X% increase damage  
      * _Phase 9.5 (implemented): `frost_impale_damage_common`, Stack(3)._  
    * (common) Reduce the cooldown of frost impale by X%  
      * _Phase 9.5 (implemented): `frost_impale_cooldown_common`, Stack(3)._  
    * (rare) Frost impale deals X% less damage, reduce the cast time by 50%  
      * _Phase 9.5 (implemented): `frost_impale_glacial_spike_rare` — a real Pre hook (the
        HookRegistry loop, not a flag) since it scales TWO stats (`damage` ×0.7, `cast_time` ×0.5)
        at once, which a single Modifier talent can't express._  
    * (rare, unique) Bonus damage from frost charges increased by 50%, cast time increased by 50%  
      * _Phase 9.5 (implemented): `frost_impale_deep_freeze_rare` — same shape, scaling
        `frost_charge_damage_percent` ×1.5 and `cast_time` ×1.5._  
    * (epic, unique) Frost impale deals X% reduced damage to all enemies hit in its path and applies frostbite  
      * _Deferred (Phase 9.5): needs a per-secondary-hit damage-scaling primitive for a piercing
        projectile — the base declarative effects list applies the SAME amount to every hit a
        piercing shot strikes; no mechanism reduces it per pierced target._  
    * (common) Increase the range by X%  
      * _Phase 9.5 (implemented): `frost_impale_range_common`, Stack(3), scales the icicle's travel
        range (`icicle_lifetime = range / speed`)._  
    * (common) Frost impale deals X% more damage to the target per enemy it passes through         
      * _Deferred (Phase 9.5): needs the same per-secondary-hit scaling primitive as the epic talent
        above, plus a non-zero base `pierce` (base kit ships single-target)._  
    * (rare) Increase Frost Impale projectile size by X%  
      * _Phase 9.5 (implemented): `frost_impale_size_rare`, Stack(2), scales `radius` (the icicle's
        own collision size)._  
* Frostbite \- frost passive (passive, no cooldown) (Unlocked randomly at level 2/3)  
  * Enemies affected by frostbite have movement speed reduced by 20% and take 10% increased damage, hitting a target affected by frostbite with a fire spell removes frostbite  
    * _Phase 4: the frostbite status itself is unchanged. Phase 9.5 (implemented): two of its five
      talents as class-wide passives (`ability_scope: None`, since Frostbite has no standalone
      ability entity — inherent to Frostbolt) — new kill-reactive systems in
      `ability::systems::mage_frost_kill`, mirroring `bone_shield_on_kill`'s shape (read the dying
      enemy's frostbite status + the killer's ActiveHooks in `CombatSet::Death`). The remaining
      three are deferred: two rescale the status's own magnitude (no primitive, the same Blaze/
      Bleed gap); "up to 3 times" needs a talent that rewrites a `StackingRule`, which also has no
      primitive (the same gap Druid's Mega Bleed deferred in Phase 9.4)._  
    * (common) Increase damage increase by X%  
      * _Deferred (Phase 9.5): status-magnitude, see above._  
    * (common) Increase slow by X%  
      * _Deferred (Phase 9.5): status-magnitude, see above._  
    * (epic, unique) Enemies can be affected by frostbite up to 3 times  
      * _Deferred (Phase 9.5): needs a `StackingRule`-rewriting talent primitive._  
    * (rare, unique) Gain a frost charge if an enemy affected by frostbite dies  
      * _Phase 9.5 (implemented): `mage_passive_frost_charge_on_frostbitten_kill_rare`._  
    * (epic) Heal X% of your max health after killing an enemy affected by frostbite  
      * _Phase 9.5 (implemented): `mage_passive_frostbitten_kill_heal_epic`._  
* Frost charge \- frost passive (passive, no cooldown) (Unlocked randomly at level 4/5)  
  * A charge generated by your spells, deal 1% increased damage against frostbitten targets per frost charge  
    * _Deferred in full (Phase 9.5) — base bullet + all four talents: this describes a UNIVERSAL
      per-hit conditional damage multiplier (any Frost-tagged hit against a frostbitten target,
      scaling with the caster's current UNSPENT `Charges` count) applied across every ability
      uniformly — the same SHAPE the crit/attack-speed universal stat sheet (Phase 9.1) covers for
      its own two stats, but no such "resource-count-scaled multiplier conditioned on the target's
      status" mechanism exists. Frost Impale's own bespoke per-charge scaling (a single ability's
      own resolved param, CONSUMING rather than merely reading the charge count) is the one piece of
      "frost charge" mechanical identity that ships this phase — see Frost Impale above. A future
      general resource-conditional-multiplier primitive is the trigger to revisit this section._  
    * (rare) Increase the damage multiplier by X  
      * _Deferred (Phase 9.5): depends on the base multiplier above._  
    * (epic, unique) Frost charges lower the damage against ablaze by X% per charge, gain 3 frost charges for each ablaze enemy hit by a frost spell  
      * _Deferred (Phase 9.5): depends on the base multiplier above._  
    * (rare, unique) Frost charges reduce your movement speed by X% per charge, but all your frost spells deal X% more damage pre charge  
      * _Deferred (Phase 9.5): depends on the base multiplier above._  
    * (epic) Heal 0.X% max health per second for each frost charge  
      * _Deferred (Phase 9.5): needs a per-second heal-over-time-scaled-by-resource primitive, on
        top of the base gap above._  
* Passive cross cutting talents  
  * _Deferred in full (Phase 9.5): every talent below references "frost spells" / "fire spells" as
    a CATEGORY, or toggles a specific other ability ("no longer use Flamewrath") —
    `TalentDef.ability_scope` binds a Modifier to exactly ONE ability (or `None` for global), never a
    group of abilities sharing an element tag; no "spell school" grouping primitive exists to hang a
    category-wide modifier on. A future tagging primitive (e.g. an `AbilityDef.school` field + a
    scope variant matching it) is the trigger to revisit this section._  
  * (rare) Reduce the damage of your frost spells by X%, increase the damage of your fire spells by Y%  
    * (rare) Reduce the damage of your fire spells by X%, increase the damage of your frost spells by Y%  
    * (rare) Frost spells no longer remove baze on enemies, frost spells deal X% reduced damage  
    * (rare) Fire spells no longer remove frostbite on enemies, fire spells deal X% reduced damage  
    * (rare) Targets affected by frostbite and blaze receive X% increased damage  
    * (epic) You can no longer use flamewrath, gain a frost charge for each application of blaze

# Paladin

Melee, healing, area damage zones, strong single target

_Phase 9.3 (implemented): the full kit below is live as its own hero (`paladin.hero.ron`).
`has_stance: false` (no Q), `resource_model: None`, base_stats 160 hp / 33 move speed (a reasonable
default, not yet a dedicated balance pass — the same caveat Phase 9.2 flagged for its own numbers).
Consecrated Ground / Spinning Hammer / Smite are unlocked from one shared `band_2_3_pool` of three
(levels 2/3/4, drawn without replacement) rather than the Death Knight's split 2-3/4-6 bands._

* Hammer of Justice \- primary attack (X second cooldown) (Unlocked at level 1\)  
  * Deal a large amount of damage to a single target, and 50% of the damage to all targets in a cone behind the primary target  
    * _Phase 9.3 (implemented): a new `hammer_cleave` behavior — acquires ONE primary (nearest enemy in range/arc, the `melee_cone` acquisition shape), then hits every other enemy in a cone behind the primary via a new `EffectTarget::SecondaryHits` + `EffectSpec::DamageFraction` (50% of the primary's own, already-talent-scaled damage — a damage talent automatically scales the cleave too)._  
    * (common) Increase the damage by X%  
      * _Phase 9.3 (implemented): `hammer_of_justice_damage_common`, Stack(3)._  
    * (common) Increase the cast range by X  
      * _Phase 9.3 (implemented): `hammer_of_justice_range_common`, Stack(3)._  
    * (rare, unique) Hammer of justice bounces up to 3 nearby targets, dealing 50% damage  
      * _Deferred (Phase 9.3): a chain-bounce targeting shape with no existing behavior to build on. No talent RON references it, so it stays invisible to the offer generator._  
    * (epic, unique) If hammer of justice kills an enemy inside consecrated ground, create an explosion at the impact location dealing X damage to all enemies in a range around the target  
      * _Deferred (Phase 9.3): needs per-kill ability attribution — `DamageEvent` carries no ability provenance, the same gap Phase 9.2's bone shield simplification hit. No talent RON references it._  
    * (rare, unique) If hammer of justice strikes a target affected by holy mark, emit a shockwave from your character, dealing X damage and pushing enemies back  
      * _Phase 9.3 (implemented): `hammer_of_justice_shockwave_rare` — a holy-mark-read + forced-movement (Phase 9.1 knockback) targeted execute.rs special-case, centred on the caster per the text ("from your character")._  
* Flash of light \- special attack (channeled while moving) (X second cooldown) (Unlocked at level 1\)  
  * Cast down a holy ray of light upon yourself, healing you for X% max health  
    * _Phase 9.3 (implemented): a new `channel_while_moving` behavior + `Channeling` component — the heal (and every talent below) resolves once the `cast_time` channel completes, not instantly. "No interrupt" (this doc's own default, phase9-plan.md §4): nothing cancels the channel once started, not even damage or movement._  
    * (common, unique) Overhealed health becomes a shield  
      * _Phase 9.3 (implemented): `flash_of_light_overheal_shield_common` — the pre-heal overheal amount becomes an `Absorb` (Phase 9.1)._  
    * (common) Increase healing by X%  
      * _Phase 9.3 (implemented): `flash_of_light_healing_common`, Stack(3)._  
    * (common) Reduce cooldown by X   
      * _Phase 9.3 (implemented): `flash_of_light_cooldown_common`, Stack(3)._  
    * (rare) Deal X% of amount healed to enemies in a radius around you  
      * _Phase 9.3 (implemented): `flash_of_light_radiate_rare` — read at channel COMPLETION (the caster's current position, since a channel can complete well after it started moving)._  
    * (epic, unique) Casting flash of light inside consecrated ground makes you radiate holy energy, exploding in a small radius around you, dealing X damage to nearby enemies  
      * _Phase 9.3 (implemented): `flash_of_light_consecrated_radiate_epic` — the zone check happens at cast START (`PlayerZonePresence`), baked into the channel; the explosion itself fires at completion._  
    * (rare, unique) Flash of light makes your next hammer of justice deal X% increased damage  
      * _Deferred (Phase 9.3): a one-shot cross-ability buff-consumption shape none of Modifier/Pre-hook/Post-hook cover cleanly (Pre hooks can't consume a marker — no `Commands` access; Post hooks are deliberately read-only, §8.1(3)). No talent RON references it._  
* Consecrated ground \- passive (passive, no cooldown) (Unlocked randomly at level 2/3/4)  
  * Drop zones of consecrated ground under your feet as you move, dealing X damage per second to enemies inside  
    * _Phase 6 (implemented as a demonstrator); Phase 9.3 (implemented): promoted to the real Paladin band ability — the mechanic itself is unchanged._  
    * (rare) Increase the size of the zone by X  
      * _Phase 9.3 (implemented): `consecrated_ground_radius_rare`, Stack(2)._  
    * (common) Increase the damage by X%  
      * _Phase 9.3 (implemented): `consecrated_ground_damage_common`, Stack(3)._  
    * (common) Consecrated ground also slows enemies inside by X%  
      * _Phase 9.3 (implemented): `consecrated_ground_slow_common` — a new `ZoneEffects.slow_status` (a fixed `consecrated_slow` status, `move_speed_mult: 0.8`) applied each zone tick._  
    * (rare) Consecrated ground deals X% increased damage per enemy inside  
      * _Phase 9.3 (implemented): `consecrated_ground_count_scaling_rare` — `zone_tick_effects` scales `damage_per_second` by `CONSECRATED_COUNT_SCALING_FRACTION` (15%) per additional occupant that tick._  
* Spinning hammer \- passive (always active, no cooldown) (Unlocked randomly at level 2/3/4)  
  * Spawn a hammer spinning around your character at all times, dealing X damage, if target is affected by holy mark, deal double damage  
    * _Phase 9.3 (implemented): a new `orbiting` behavior — modeled as a fast (0.25s) AutoCast maintenance cadence sampling the hammer's current position (driven by a new `AbilityContext.elapsed_secs`) rather than a literal continuous-collision sweep, the same discrete-sampling approximation the zone-tick system already uses. The holy-mark double damage is the holy-mark READ path — a targeted execute.rs special-case (a per-target conditional the generic effects pipeline can't express)._  
    * (rare, unique) Spinning hammer also stuns enemies for X seconds  
      * _Phase 9.3 (implemented): `spinning_hammer_stun_rare` — a targeted execute.rs special-case, same shape as Abomination Limb's stun talent (Phase 9.2)._  
    * (epic) Spawn an additional hammer orbiting your character  
      * _Phase 9.3 (implemented): `spinning_hammer_extra_hammer_epic`, `+1 hammer_count` — however many hammers that resolves to are evenly re-spaced automatically._  
    * (common) increase the damage by X%  
      * _Phase 9.3 (implemented): `spinning_hammer_damage_common`, Stack(3)._  
    * (common) Increase the radius by X  
      * _Phase 9.3 (implemented): `spinning_hammer_radius_common` (the orbit radius), Stack(3)._  
* Smite \- passive (X second cooldown) (Unlocked randomly at level 2/3/4)  
  * Smite the closest enemy dealing X damage, applying a holy mark to the target  
    * _Phase 9.3 (implemented): reuses `nearest_melee` (`target_count: 1`) as-is — no new behavior. The holy-mark GRANT path (its `effects` list applies `holy_mark` alongside the damage)._  
    * (common) increase the damage by X%  
      * _Phase 9.3 (implemented): `smite_damage_common`, Stack(3)._  
    * (common) Increase the range by X  
      * _Phase 9.3 (implemented): `smite_range_common`, Stack(3)._  
    * (rare, unique) After smiting an enemy, create a consecrated ground under him, dealing X damage to every enemy inside every second  
      * _Phase 9.3 (implemented): `smite_spawns_consecrated_rare` — a targeted execute.rs special-case (spawns at the smitten TARGET's position), same shape as `bdk_passive_blood_boil_spawns_dnd`._  
    * (epic) Holy mark affects all enemies in a radius around the target  
      * _Phase 9.3 (implemented): `smite_mark_radius_epic`, Exclusive — a targeted execute.rs special-case._  
    * (rare) Smite strikes an additional target  
      * _Phase 9.3 (implemented): `smite_extra_target_rare`, `+1 target_count` — the same primitive Heart Strike's own "+1 target" talent uses (Phase 9.2)._

# General Passive talents

* (rare, unique \[5\]) Gain 10% movement speed up to 50% for x seconds after killing an enemy  
* (common) Gain X% crit strike  
  * _Phase 9.1 (implemented): the underlying stat sheet — every ability resolves a `crit_chance` /
    `crit_mult` pair (neutral defaults) that a general talent can modify, and the damage-application
    path rolls an independent crit per hit from `RunRng`. This specific talent (granting the
    percentage) is still content._
* (common) Gain X% attack speed  
  * _Phase 9.1 (implemented): the underlying stat sheet — every ability resolves an `attack_speed`
    value (neutral default 0) that shortens its cooldown via `cooldown / (1 + attack_speed)`. This
    specific talent is still content._  
* (rare, unique) Deal X% increased damage to enemies within close range, but take Y% increased damage from projectiles  
* (rare, unique) Killing a rare enemy or a boss heals you for X% of your max health and increases your movement speed by Y% for Z seconds  
* (epic, unique) Taking damage emits a shockwave, knocking enemies back and destroying all enemy projectiles (cooldown X seconds)  
* (rare, unique) Gain X% increased experience from all enemies, but bosses have Y% more health and deal Z% more damage  
* (rare, unique) Deal X% increased damage, but your max health is reduced by Y%  
* (epic, unique) If you clear a room without taking any damage, permanently increase your damage by X% and raise your max health by Y%


  

# Game progression and big picture level design

Big picture:

* Character progression:  
  * Select a character  
  * Basic active abilities available from level 1  
  * Reach level 2 by finishing map 1  
  * Gain core offensive abilities at random at level 2 and 3  
  * Gain other core abilities at levels 4-6 at random  
  * Select 1 of 3 available talents (common, rare, epic) after each level up after getting all core abilities  
  * Select 1 of 3 rare or epic available talents after special events  
  * Remove a talent at a merchant  
  * Trade 3 talents for a new random talent of higher quality  
  * Gain experience by killing enemies (more difficult enemy, more XP)  
* Map progression:  
  * Map layout procedurally generated  
  * Different map types  
    * Survive X minutes  
    * Kill all enemies  
    * Kill map boss  
    * Act Boss  
  * Maps have different themes \- each theme has different type of enemies (Each enemy has different ability, bosses have multiple abilities \- all TBD)  
    * Sand dune  
      * Enemy types:  
        * Scorpions, Vultures, Tusken (Sand people), Flame demon, Oil elemental  
      * Bosses:  
        * King Scorpion, Tusken Lord, Flame Cultist, Undead Lord  
    * Forest  
      * Enemy types:  
        * Bear, Wolf, Corrupted Ranger, Hiker,   
      * Bosses:  
        * Mad Lumberjack, Dire wolf, Ent lord, Corrupted Druid   
    * Castle ruins  
      * Enemy types:  
        * Animated Armor, Dancing Sword, Gargoyle, Skeleton, Banshee  
      * Bosses:  
        * The fallen King, Grand Lich, Gargoyle Lord  
    * Frozen wasteland  
      * Enemy types:  
        * Frostbite Zombie, Ice Elemental, White Bear, Snow Troll, Icy Owl, Sabretooth Cat  
      * Bosses:  
        * Yeti Abomination, Frost Giant, Winter Witch  
    * Alpine lakeside  
      * Enemy types:  
        * Stone golems, Lake Siren, Mountain Eagle, Corrupted Fisherman, Mud golem  
      * Bosses:  
        * The Lake Phantom, Fallen Townlord, Corrupted Mountaineer, King Crab  
  * Maps contain swarms of enemies and a map boss  
  * Enemies have their own scaling, independent of the player  
  * Player can choose to speedrun to finish the objective, but may end up being underleveled for the map  
* Act progression  
  * Three acts in a playthrough  
  * Each act has 3 different paths, intertwining and branching off (Slay the Spire style)  
  * Player has to complete 15 encounters in an act, encounters may be:  
    * Map, Boss Room, Act Boss, Throne Room, Merchant  
  * Act 1 has a “tutorial” map as the beginning, where the player reaches level 2 \- then branching off  
  * Other acts start branched off, the player can select his path  
  * The player can see the encounter type, and the map theme   
  * After Act 3 boss is a secret level, enabled by completing special feats of strength  
    * TBD  
  * _Phase 7 (implemented): the act flow is live end-to-end — a seeded 15-column branching graph per
    act (single entry Map / terminal Act Boss / a Boss Room / a guaranteed Throne Room), typed
    encounters (Map with a Survive / Kill-all / Kill-map-boss objective, Boss Room, Act Boss, Throne
    Room curse+kiss, Merchant rest node), a themed depth-scaled enemy spawner (enemies scale with how
    deep into the run the node is), and a keyboard branch picker between encounters. **The 5 theme
    rosters + bosses are placeholders** (they point at the existing grunt/runner/brute/spitter + a
    stand-in "warlord" boss); the designed per-theme enemies/bosses above are a Phase-9 data pass. The
    Throne Room curses are live for enemy-stat effects (e.g. "enemies deal double damage"); the
    player-stat curses (no-regen / slow) get their consumers as those mechanics land. Merchant
    remove/trade **ops**, run save/resume, and the visual act-graph map screen are later phases._
  * _Phase 7.5 (implemented): the **visual act-graph map screen** is live — a Slay-the-Spire column
    view showing each node's encounter type + theme, with the reachable branches numbered (the player
    sees the encounter type and map theme before choosing, as designed). The **Merchant remove/trade
    ops** are live (remove a talent; sacrifice 3 for 1 higher-rarity pick). Run **save/resume** is
    still Phase 8._
  * _Phase 8 (implemented): **run save/resume** is live — a run is snapshotted (RunState + the exact
    RunRng stream position) into `MetaState.in_progress_run` at every node boundary (an encounter
    clearing, or an act advance); **Resume Run** from the main menu tears down and rehydrates that
    snapshot into a live run byte-for-byte, including the RNG stream — a resumed encounter's roster
    is identical to what an uninterrupted run would have rolled at that point._
* User Flow (screens)
  _Phase 8 status: the windowed game now boots to **Log in** → **Main menu** → **Character
  selection** → run; **Game over** (death/victory) with restart, **Pause** (Esc, a build inspector),
  the in-run **HUD**, the visual **map** screen, the **Merchant** shop, the **Throne Room curse
  banner**, **Resume Run**, and the **Scoreboard** (+ score formula) are all live. Hero lock/unlock
  greying is wired end-to-end (character select greys a locked hero and refuses its pick) but every
  defined hero ships unlocked for now — concrete unlock triggers arrive with the Phase-9 roster. Not
  scheduled: a separate **Heroes** gallery (character select covers it) and **Settings** (nothing to
  configure yet). Every screen is keyboard-driven; mouse is a later polish pass._
  * Log in _(live — a local-profile splash; no credentials/multi-profile, see architecture-plan §6 Q3)_
  * Main menu _(live — New Run / Resume Run (greyed with no save) / Scoreboard)_
    * Start new run _(live)_
      * Character selection _(live)_
        * Character hero cards with sample abilities, talents, CTA button _(live — cards show name,
          stance, resource, level-1 abilities)_
        * Unlocked characters colorful, locked characters grayed out _(live — every hero ships
          unlocked for now; the greying + locked-pick refusal is tested against a deliberately-locked
          hero, since none are locked by default yet)_
    * Resume run _(live — only enabled when a save exists; rehydrates the exact saved run)_
    * Heroes _(folded into character select for now)_
    * Scoreboard _(live — run history sorted by score: act/node depth + level + a victory bonus +
      a faster-clear speed bonus)_
    * Settings _(later — nothing to configure yet)_
    * Exit Game _(live)_  
* 

