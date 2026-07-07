// Talent state attached to the player entity.
//
// Two components live on the player:
//   AcquiredTalents — the full list of talents the player has taken this run.
//   ActiveHooks     — the set of HookIds currently active (one entry per Behavior talent acquired).
//
// When a talent with TalentEffect::Behavior is acquired, talent/systems/apply.rs pushes
// its HookId into ActiveHooks. When the talent is removed (merchant), it's popped out.
// This means the behavior system can check `active_hooks.contains("my_hook")` in O(n)
// where n is typically small (< 20 hook talents at max progression).
//
// Interactions:
//   - talent/systems/apply.rs mutates both components.
//   - ability/systems/execute.rs reads ActiveHooks to gate pre/post hook execution.
//   - ability/systems/resolve_params.rs reads AcquiredTalents to build the modifier stack.
//   - progression/systems/offer.rs reads AcquiredTalents for uniqueness checks.
//   - talent/systems/merchant.rs mutates AcquiredTalents and ActiveHooks on merchant ops.

use bevy::prelude::*;
use crate::talent::assets::TalentId;
use crate::ability::assets::HookId;

/// The player's full acquired talent list for this run.
/// `(TalentId, u8)` — the u8 is the copy count for Stack(N) talents (usually 1).
#[derive(Component, Debug, Default, Clone)]
pub struct AcquiredTalents {
    pub entries: Vec<(TalentId, u8)>,
}

impl AcquiredTalents {
    pub fn count_of(&self, id: &str) -> u8 {
        self.entries.iter()
            .find(|(t, _)| t == id)
            .map(|(_, c)| *c)
            .unwrap_or(0)
    }

    pub fn has(&self, id: &str) -> bool {
        self.count_of(id) > 0
    }

    /// Adds one copy of the talent (increments count for Stack talents).
    pub fn add(&mut self, id: TalentId) {
        if let Some(entry) = self.entries.iter_mut().find(|(t, _)| *t == id) {
            entry.1 += 1;
        } else {
            self.entries.push((id, 1));
        }
    }

    /// Removes one copy. Returns false if not present.
    pub fn remove_one(&mut self, id: &str) -> bool {
        if let Some(pos) = self.entries.iter().position(|(t, _)| t == id) {
            if self.entries[pos].1 > 1 {
                self.entries[pos].1 -= 1;
            } else {
                self.entries.remove(pos);
            }
            true
        } else {
            false
        }
    }
}

/// Set of currently active behavior hook IDs.
/// Maintained in sync with AcquiredTalents by talent/systems/apply.rs.
#[derive(Component, Debug, Default, Clone)]
pub struct ActiveHooks {
    pub hooks: Vec<HookId>,
}

impl ActiveHooks {
    pub fn contains(&self, id: &str) -> bool {
        self.hooks.iter().any(|h| h == id)
    }

    pub fn add(&mut self, id: HookId) {
        if !self.contains(&id) {
            self.hooks.push(id);
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.hooks.retain(|h| h != id);
    }
}

/// Kills accumulated toward Bone Shield's next grant (Phase 9.2, Death Strike's epic talent).
/// Wraps past the threshold (a multi-kill frame can grant more than once). Inserted unconditionally
/// alongside `AcquiredTalents`/`ActiveHooks` at player spawn (whether or not the talent is ever
/// taken) so `ability::systems::bone_shield::bone_shield_on_kill` never needs to reason about a
/// lazily-inserted component racing a kill.
#[derive(Component, Debug, Default, Clone)]
pub struct BoneShieldProgress(pub u32);
