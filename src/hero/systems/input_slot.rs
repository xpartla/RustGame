// Translates player input + active stance → TriggerAbilityEvent.
//
// This is the indirection layer between raw key/mouse input and ability execution (Phase 4).
// It knows nothing about what the ability does — it only resolves which ability is currently
// mapped to the pressed slot for the player's active stance and emits TriggerAbilityEvent.
//
// Resolution path:
//   1. Read the pressed InputSlot from mouse/keyboard input (LMB → Basic, RMB → Special,
//      Shift/Space → Movement).
//   2. Read the player's ActiveStance.
//   3. Look up HeroDef.stance_slots for the matching (stance, slot) pair → AbilityId.
//   4. Emit TriggerAbilityEvent { ability_id, owner }.
//
// StanceSwap (Q) is handled separately by hero/systems/stance.rs, not here. The Movement slot
// (Shift/Space, Phase 9.1) is wired here, but no shipped hero binds it yet — every `stance_slots`
// entry's `movement` field is still `None` (see assets/abilities/dash.ability.ron, the unbound
// demonstrator), so this stays byte-identical until a class' kit claims the slot.
//
// Replaces the Phase-1 stub player/systems/ability_input.rs, which hardcoded LMB → death_strike.
// Runs before CombatSet::Damage so the event is available to execute_ready_abilities that frame.

use bevy::prelude::*;
use crate::ability::assets::AbilityId;
use crate::ability::components::TriggerAbilityEvent;
use crate::core::components::AbilitiesSuppressed;
use crate::hero::assets::{HeroDef, HeroLibrary};
use crate::hero::components::{ActiveStance, HeroIdentity, InputSlot};

/// Pure resolver: which ability (if any) the given input slot maps to, for `stance`, on `hero_def`.
/// StanceSwap is never resolved here (it fires the swap, not a slot ability). Returns None when
/// the stance is unknown or the slot is unbound. Kept free of ECS types so it is unit-testable.
pub fn resolve_slot(hero_def: &HeroDef, stance: &str, slot: InputSlot) -> Option<AbilityId> {
    let mapping = hero_def.stance_slots.iter().find(|m| m.stance == stance)?;
    match slot {
        InputSlot::Basic => mapping.basic.clone(),
        InputSlot::Special => mapping.special.clone(),
        InputSlot::Movement => mapping.movement.clone(),
        InputSlot::StanceSwap => None,
    }
}

/// Reads mouse + keyboard input and the player's active stance and emits a TriggerAbilityEvent for
/// each pressed, bound slot. No-op until the player's HeroDef asset has loaded.
pub fn resolve_input_to_ability(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    // A suppressed (stunned) player cannot cast — excluded from the query.
    player: Query<(Entity, &HeroIdentity, &ActiveStance), Without<AbilitiesSuppressed>>,
    hero_library: Res<HeroLibrary>,
    hero_defs: Res<Assets<HeroDef>>,
    mut trigger_events: EventWriter<TriggerAbilityEvent>,
) {
    let mut pressed: Vec<InputSlot> = Vec::new();
    if mouse.just_pressed(MouseButton::Left) {
        pressed.push(InputSlot::Basic);
    }
    if mouse.just_pressed(MouseButton::Right) {
        pressed.push(InputSlot::Special);
    }
    // Mechanics' "Shift / Space for movement ability, i.e. dash" (Phase 9.1). Either key triggers
    // the same slot; only one TriggerAbilityEvent per frame even if both are pressed together.
    if keys.just_pressed(KeyCode::ShiftLeft) || keys.just_pressed(KeyCode::Space) {
        pressed.push(InputSlot::Movement);
    }
    if pressed.is_empty() {
        return;
    }

    for (owner, hero_id, stance) in &player {
        let Some(handle) = hero_library.get(&hero_id.0) else { continue };
        let Some(hero_def) = hero_defs.get(handle) else { continue };
        for slot in &pressed {
            if let Some(ability_id) = resolve_slot(hero_def, &stance.0, *slot) {
                trigger_events.write(TriggerAbilityEvent { ability_id, owner });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_hero(rel_path: &str) -> HeroDef {
        let full = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel_path);
        let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("read {full}: {e}"));
        ron::de::from_bytes::<HeroDef>(&bytes)
            .unwrap_or_else(|e| panic!("parse {rel_path}: {e}"))
    }

    #[test]
    fn resolves_default_stance_slots_for_death_knight() {
        let dk = load_hero("assets/heroes/blood_death_knight.hero.ron");
        assert_eq!(resolve_slot(&dk, "default", InputSlot::Basic).as_deref(), Some("death_strike"));
        assert_eq!(resolve_slot(&dk, "default", InputSlot::Special).as_deref(), Some("dnd"));
        assert_eq!(resolve_slot(&dk, "default", InputSlot::Movement), None);
        // StanceSwap is never a slot ability; the stance system handles Q.
        assert_eq!(resolve_slot(&dk, "default", InputSlot::StanceSwap), None);
        // Unknown stance resolves to nothing.
        assert_eq!(resolve_slot(&dk, "fire", InputSlot::Basic), None);
    }

    #[test]
    fn resolves_mage_basic_per_active_stance() {
        let mage = load_hero("assets/heroes/mage.hero.ron");
        // The same Basic slot maps to a different ability depending on the active stance —
        // this is the stance-remaps-LMB mechanic, resolved purely from data.
        assert_eq!(resolve_slot(&mage, "fire", InputSlot::Basic).as_deref(), Some("fireblast"));
        assert_eq!(resolve_slot(&mage, "ice", InputSlot::Basic).as_deref(), Some("frostbolt"));
        // Both Specials are bound now (Phase 9.5: Flamestrike / Frost Impale). Movement is still
        // unbound — no shipped hero claims the Movement slot yet.
        assert_eq!(resolve_slot(&mage, "fire", InputSlot::Special).as_deref(), Some("flamestrike"));
        assert_eq!(resolve_slot(&mage, "ice", InputSlot::Special).as_deref(), Some("frost_impale"));
        assert_eq!(resolve_slot(&mage, "ice", InputSlot::Movement), None);
    }
}
