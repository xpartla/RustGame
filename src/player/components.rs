use bevy::prelude::Component;
use crate::constants::{XP_FIRST_LEVEL, XP_LEVEL_STEP};

#[derive(Component)]
pub struct Player;

/// Player progression. `current` accumulates toward `to_next`; on reaching it the player
/// levels up (overflow carries) and `to_next` is recomputed from the level curve.
#[derive(Component)]
pub struct Experience {
    pub current: u32,
    pub to_next: u32,
    pub level: u32,
}

impl Experience {
    /// Fresh level-1 progression.
    pub fn new() -> Self {
        Self {
            current: 0,
            to_next: Self::to_next_for(1),
            level: 1,
        }
    }

    /// XP required to advance *from* `level` to the next. Simple linear curve.
    pub fn to_next_for(level: u32) -> u32 {
        XP_FIRST_LEVEL + (level - 1) * XP_LEVEL_STEP
    }
}
