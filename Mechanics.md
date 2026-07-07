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

* Stance Swap \- Q (X second cooldown) (Unlocked at level 1\)  
  * Change from human to animal form and cast Scratch  
  * Change from animal form to human and cast Roots  
* Bleed (passive) (passive, no cooldown) (Unlocked at level 1\)  
  * Enemies affected by bleed suffer X damage every second  
    * (common) Increase bleed damage by X%  
    * (common) Increase bleed duration by X seconds  
    * (common) Bleed ticks X% faster  
    * (rare) Bleed slows enemies by X%  
    * (epic) Hitting an enemy while bleeding increases the bleed duration by X seconds  
    * (rare, unique) Bleeding targets take X% increased damage from Human abilities  
* Scratch (Animal) \- Basic Attack (X second cooldown) (Unlocked at level 1\)  
  * Hit all enemies in a cone in front of you for X damage  
  * Enhanced \- Scratch applies a bleed on all enemies hit  
    * (common) Increase the size by X%  
    * (common) Increase the damage by X%  
    * (common) Scratch applies bleed to the closest X targets  
    * (rare, unique) Scratch deals X% more damage to enemies affected by root  
    * (rare) Increase bleed duration by X seconds  
    * (rare, unique) Scratch deals X% more damage to bleeding targets  
    * (epic, unique) Scratch deals only 50% damage, bleed deals triple damage  
* Ferocious Bite (Animal) \- Special Attack (X second cooldown) (Unlocked at level 1\)  
  * Jump to the closest target near your cursor and deal X damage to the enemy. Always critically strikes if the target is bleeding  
  * Enhanced \- Ferocious bite cleaves in a circle around you, applying X% of the damage as bleed  
    * (common) Increase damage by X%  
    * (common) Increase range by X  
    * (rare, unique) Ferocious bite consumes active bleed on the target, to deal X% increased damage per bleed stack  
    * (epic, unique) If Ferocious bite kills an enemy, it resets the cooldown of your next stance swap, and grants 1 Enhanced charge  
    * (rare, unique) If cast while standing inside a Tree Conduit, the cleave applies bleed X times  
    * (epic, unique) Ferocious bite deals no damage to the primary target, instead it deals all the remaining bleed from the current target instantly to enemies inside the cleave radius  
* Primal Pounce (Animal) \- Passive (X second cooldown) (Unlocked randomly at level 2/3)  
  * Every X seconds automatically leap towards the highest-health enemy within a radius, dealing X damage and applying a bleed  
    * (common, unique \[5\]) If the target was Rooted, gain X% movement speed for Y seconds after leaping  
    * (rare, unique) Create a Bloom flower at the point you jumped from  
    * (epic, unique) Primal pounce deals no direct damage, instead the bleed is applied to all targets in your path  
    * (common) Increase the range by X  
    * (rare, unique) If the target is rooted, deal triple damage  
* Roots (Human) \- Basic Attack (X second cooldown) (Unlocked at level 1\)  
  * Shoot a projectile in front of you that deals X damage  
    * (rare, unique) Projectile can pass through enemies  
    * (common, unique) Enemies hit by roots are stunned for X seconds  
    * (common) Increase damage by X%  
    * (rare, unique \[3\]) Shoot additional projectile towards the nearest enemy  
* Heal (Human) \- Special Attack (channeled while moving) (X second cooldown) (Unlocked at level 1\)  
  * Heals you for X% max health  
    * (rare, unique) You heal for X% more per bleeding enemy within Y range  
    * (rare, unique) Your next attack in animal form is enhanced  
    * (rare, unique) Your heal also heals your Ent  
    * (common, unique\[3\]) Lower cast time by X%  
* Tree conduit (human) \- passive (X second cooldown) (Unlocked randomly at level 4/5)  
  * Spawn a tree for Y seconds, within X range of the tree, your next animal attack is enhanced  
    * _Phase 6 (implemented as a marker demonstrator — no Druid hero yet): drops a "tree_conduit" zone queryable via PlayerZonePresence. The "enhanced next animal attack" consumer is deferred to the Druid content pass._  
    * (common) Increase tree radius by X%  
    * (rare) Reduce spawn range by X%  
    * (rare) Increase duration by X seconds  
    * (epic, unique) All animal attacks are enhanced while in tree range  
* Bloom (Human) \- passive (X second cooldown) (Unlocked randomly at level 4/5)  
  * Periodically spawn a flower that can be picked up when ran over, upon pickup your next animal form attack is enhanced  
    * (rare) After picking up you heal for X% health over Y seconds  
    * (rare, unique) Your next 2 attacks are enhanced  
    * (common) You gain X% movement speed after pickup  
* Spawn Ent (Human) \- passive (X second cooldown) (multiple ents can live simultaneously) (Unlocked randomly at level 2/3)  
  * Periodically spawn an Ent that runs towards the nearest enemy, forcing the enemy to attach the Ent instead of you  
    * (common) Ent lowers the max health of nearby enemies by X% while alive  
    * (epic, unique\[Fiery Ent / Earth Ent\]) Fiery Ent \- Ent has 50% reduced health and explodes on death, dealing X damage to enemies around him  
      * (common) Increase Fiery Ent damage by X%  
      * (rare, unique) Spawn a mini Fiery Ent for each enemy killed by the explosion, mini Ents are unkillable and live for X seconds, dealing Y damage per second to nearby enemies per Ent \- adding up  
    * (epic, unique\[Earth Ent/ Fiery Ent\]) Earth Ent \- Ent has 200% increased health and casts entangling zone around himself, rooting the enemies in place   
      * (rare, unique) After entangled enemy gets hit by roots, they transform into spiky roots, dealing X damage per second to enemies within Y range, multiplying  
    * (common) Reduce cooldown by X  
    * (rare, unique) Ents can pick up bloom flowers, healing all summoned ents and granting them X% increased movement speed for Y seconds  
* Passive Abilities  
  * (rare, unique) Mega bleed \- enemies can bleed from the same ability up to 3 times, each new application refreshes the old one  
  * (rare, unique) Unstable form \- your first 3 casts in animal form are always enhanced, you have a 10% chance of turning to human form after casting an enhanced ability  
  * (epic, unique) Master of the Forest \- You can no longer turn into animal form, Spawn Ent spawns an additional 2 Ents, Blooming flowers explode after getting picked up by Ents, causing the flower to erupt, dealing X damage in a radius around the flower  
  * (rare, unique) Swapping forms costs X% of your current health, for Y seconds after swapping forms, you deal Y% increased damage and your movement speed is doubled

# Mage

Ice form \- slow, control, damage spikes, combo  
Fire form \- DoTs, spreading damage, big AoE

* Stance swap \- Q (active, no cooldown) (Unlocked at level 1\)  
  * Fire \-\> Ice \- gain ice barrier absorbing the next attack / projectile  
    * _Phase 4 (implemented): Ice Barrier is a **\-40% damage-taken buff for 3s** (a status), a stand-in for the true next-hit absorb — the absorb/shield system is deferred (architecture-plan §8.6)._  
  * Ice \-\> Fire, \- gain boots of fire, increase movement speed by Y% for X sec  
    * _Phase 4 (implemented): Boots of Fire is **\+30% movement speed for 3s** (a status). Values are tunable placeholders._  
* Fireblast \- fire basic attack (X second cooldown) (Unlocked at level 1\)  
  * Shoot a projectile, dealing X damage, setting enemy hit ablaze  
    * (common, unique) Projectile explodes on impact, dealing X damage to nearby enemies  
    * (common) Increase damage by X%  
    * (common) Increase range by X  
* Blaze \- fire passive (passive, no cooldown) (Unlocked randomly at level 2/3)  
  * Enemies affected by blaze receive X damage every Y seconds, hitting an enemy affected by blaze with a frost spell removes blaze  
    * (common) Increase blaze damage by X%  
    * (rare, unique) Blaze deals double damage, enemies affected by blaze have movement speed increased by 50%  
    * (rare, unique) Blaze deals 50% reduced damage, after running out, blaze moves to a nearby target (one blaze can jump up to 3 times)  
* Flamewrath \- fire passive (X second cooldown)  (Unlocked randomly at level 4/5)  
  * Periodically apply to the nearest target. Nearest ablaze enemy consumes blaze effect and create an explosion around himself, dealing X damage to nearby enemies  
    * (common) Increase cast radius by X%  
    * (common) Increase explosion range by X%  
    * (common) Increase damage by X%  
    * (common) Reduce cooldown by X %  
    * (common, unique) Flamewrath deals 50% reduced damage, but does not consume the blaze stack  
    * (rare) Flamewrath affects an additional target  
* Flamestrike \- fire special attack (X second cooldown) (Unlocked at level 1\)  
  * Cast a fiery circle, dealing X damage to enemies within the zone, dealing increased damage per enemy affected by blaze  
    * (common) Increase cast range by X%  
    * (common) Increase zone range by X%  
    * (common) Increase damage by X%  
    * (epic, unique) Flamestrike deals no damage, and triggers flamewrath on all enemies hit  
    * (epic, unique) Flamestrike deals 80% reduced damage, and deals the remaining blaze damage on enemies instantly, removing the blaze effects  
* Frostbolt \- ice basic attack (X second cooldown) (Unlocked at level 1\)  
  * Shoot a projectile, dealing X damage to the first enemy hit, applying frostbite, if the target is already affected by frostbite, generate a frost charge  
    * (common) Increase damage by X%  
    * (common) Increase range by X%  
    * (rare) Frostbolt pierces through, hitting an additional enemy  
    * (common) Increase the projectile size by X%  
    * (epic, unique) if an enemy affected by frostbite gets killed by frostbolt, they explode dealing X damage to nearby enemies  
* Frost Impale \- frost special attack (long channel while moving) (X second cooldown) (Unlocked at level 1\)  
  * Consume all frost charges to launch a massive icicle at a target dealing X damage, increased by Y% per frost charge  
    * (common) Frost impale deals X% increase damage  
    * (common) Reduce the cooldown of frost impale by X%  
    * (rare) Frost impale deals X% less damage, reduce the cast time by 50%  
    * (rare, unique) Bonus damage from frost charges increased by 50%, cast time increased by 50%  
    * (epic, unique) Frost impale deals X% reduced damage to all enemies hit in its path and applies frostbite  
    * (common) Increase the range by X%  
    * (common) Frost impale deals X% more damage to the target per enemy it passes through         
    * (rare) Increase Frost Impale projectile size by X%  
* Frostbite \- frost passive (passive, no cooldown) (Unlocked randomly at level 2/3)  
  * Enemies affected by frostbite have movement speed reduced by 20% and take 10% increased damage, hitting a target affected by frostbite with a fire spell removes frostbite  
    * (common) Increase damage increase by X%  
    * (common) Increase slow by X%  
    * (epic, unique) Enemies can be affected by frostbite up to 3 times  
    * (rare, unique) Gain a frost charge if an enemy affected by frostbite dies  
    * (epic) Heal X% of your max health after killing an enemy affected by frostbite  
* Frost charge \- frost passive (passive, no cooldown) (Unlocked randomly at level 4/5)  
  * A charge generated by your spells, deal 1% increased damage against frostbitten targets per frost charge  
    * (rare) Increase the damage multiplier by X  
    * (epic, unique) Frost charges lower the damage against ablaze by X% per charge, gain 3 frost charges for each ablaze enemy hit by a frost spell  
    * (rare, unique) Frost charges reduce your movement speed by X% per charge, but all your frost spells deal X% more damage pre charge  
    * (epic) Heal 0.X% max health per second for each frost charge  
* Passive cross cutting talents  
  * (rare) Reduce the damage of your frost spells by X%, increase the damage of your fire spells by Y%  
    * (rare) Reduce the damage of your fire spells by X%, increase the damage of your frost spells by Y%  
    * (rare) Frost spells no longer remove baze on enemies, frost spells deal X% reduced damage  
    * (rare) Fire spells no longer remove frostbite on enemies, fire spells deal X% reduced damage  
    * (rare) Targets affected by frostbite and blaze receive X% increased damage  
    * (epic) You can no longer use flamewrath, gain a frost charge for each application of blaze

# Paladin

Melee, healing, area damage zones, strong single target

* Hammer of Justice \- primary attack (X second cooldown) (Unlocked at level 1\)  
  * Deal a large amount of damage to a single target, and 50% of the damage to all targets in a cone behind the primary target  
    * (common) Increase the damage by X%  
    * (common) Increase the cast range by X  
    * (rare, unique) Hammer of justice bounces up to 3 nearby targets, dealing 50% damage  
    * (epic, unique) If hammer of justice kills an enemy inside consecrated ground, create an explosion at the impact location dealing X damage to all enemies in a range around the target  
    * (rare, unique) If hammer of justice strikes a target affected by holy mark, emit a shockwave from your character, dealing X damage and pushing enemies back  
* Flash of light \- special attack (channeled while moving) (X second cooldown) (Unlocked at level 1\)  
  * Cast down a holy ray of light upon yourself, healing you for X% max health  
    * (common, unique) Overhealed health becomes a shield  
    * (common) Increase healing by X%  
    * (common) Reduce cooldown by X   
    * (rare) Deal X% of amount healed to enemies in a radius around you  
    * (epic, unique) Casting flash of light inside consecrated ground makes you radiate holy energy, exploding in a small radius around you, dealing X damage to nearby enemies  
    * (rare, unique) Flash of light makes your next hammer of justice deal X% increased damage  
* Consecrated ground \- passive (passive, no cooldown) (Unlocked randomly at level 2/3/4)  
  * Drop zones of consecrated ground under your feet as you move, dealing X damage per second to enemies inside  
    * _Phase 6 (implemented as a demonstrator — no Paladin hero yet): drops a "consecrated_ground" zone dealing a Holy DoT to enemies inside (the generic zone occupant-tick, faction-gated). The slow / per-enemy-scaling talents are deferred to the Paladin content pass._  
    * (rare) Increase the size of the zone by X  
    * (common) Increase the damage by X%  
    * (common) Consecrated ground also slows enemies inside by X%  
    * (rare) Consecrated ground deals X% increased damage per enemy inside  
* Spinning hammer \- passive (always active, no cooldown) (Unlocked randomly at level 2/3/4)  
  * Spawn a hammer spinning around your character at all times, dealing X damage, if target is affected by holy mark, deal double damage  
    * (rare, unique) Spinning hammer also stuns enemies for X seconds  
    * (epic) Spawn an additional hammer orbiting your character  
    * (common) increase the damage by X%  
    * (common) Increase the radius by X  
* Smite \- passive (X second cooldown) (Unlocked randomly at level 2/3/4)  
  * Smite the closest enemy dealing X damage, applying a holy mark to the target  
    * (common) increase the damage by X%  
    * (common) Increase the range by X  
    * (rare, unique) After smiting an enemy, create a consecrated ground under him, dealing X damage to every enemy inside every second  
    * (epic) Holy mark affects all enemies in a radius around the target  
    * (rare) Smite strikes an additional target

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

