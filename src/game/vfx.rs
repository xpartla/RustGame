// Cast-VFX presentation (Phase 7.5F) — consumes the `CastVfxEvent` bus and draws the flash.
//
// This is the presentation half of the bus: the logic side (execute_ready_abilities) only *emits*
// events; every VFX entity lives here, so a logic-path spawn never perturbs the golden master.
// Registered only by PresentationPlugin. The Blood Boil nova flash (the §8.5 deferral) is the first
// consumer — a fading, expanding ring drawn with gizmos (no mesh assets, mirroring the debug
// hitbox-gizmo path). `CastVfxKind::Other` casts keep their existing gizmo VFX, so the bus ignores them.

use bevy::color::Color;
use bevy::prelude::*;

use crate::ability::components::{CastVfxEvent, CastVfxKind};

/// A transient expanding ring for a self-nova cast. Presentation-only; despawned when its timer ends.
#[derive(Component)]
pub struct NovaFlash {
    origin: Vec2,
    radius: f32,
    timer: Timer,
    color: Color,
}

/// Spawns a `NovaFlash` for each `Nova` cast-VFX event.
pub fn spawn_cast_vfx(mut commands: Commands, mut events: EventReader<CastVfxEvent>) {
    for ev in events.read() {
        if let CastVfxKind::Nova { radius } = ev.kind {
            commands.spawn(NovaFlash {
                origin: ev.origin,
                radius: radius.max(4.0),
                timer: Timer::from_seconds(0.3, TimerMode::Once),
                color: Color::srgb(0.85, 0.15, 0.15), // blood
            });
        }
    }
}

/// Animates + draws each nova ring (expands toward full radius, fades to transparent) and despawns it
/// when finished.
pub fn draw_cast_vfx(
    mut commands: Commands,
    time: Res<Time>,
    mut gizmos: Gizmos,
    mut flashes: Query<(Entity, &mut NovaFlash)>,
) {
    for (entity, mut flash) in &mut flashes {
        flash.timer.tick(time.delta());
        let f = flash.timer.fraction();
        let radius = flash.radius * (0.3 + 0.7 * f);
        gizmos.circle_2d(flash.origin, radius, flash.color.with_alpha(1.0 - f));
        if flash.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}
