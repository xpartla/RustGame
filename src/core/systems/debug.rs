use bevy::color::palettes::css::{GREEN, RED};
use bevy::prelude::{Gizmos, Query, Vec2};
use crate::core::components::{Health, WorldPosition};

const BAR_WIDTH: f32 = 30.0;
const BAR_Y_OFFSET: f32 = 22.0;

/// Debug health bar drawn above any entity that has `Health`. Only shown once the entity
/// has taken damage, to avoid cluttering full-health entities. Red track + green fill.
pub fn draw_health_bars(
    mut gizmos: Gizmos,
    query: Query<(&WorldPosition, &Health)>,
) {
    for (pos, health) in &query {
        if health.max <= 0.0 || health.current >= health.max {
            continue;
        }

        let frac = (health.current / health.max).clamp(0.0, 1.0);
        let center = pos.0 + Vec2::new(0.0, BAR_Y_OFFSET);
        let left = center - Vec2::new(BAR_WIDTH / 2.0, 0.0);
        let right = center + Vec2::new(BAR_WIDTH / 2.0, 0.0);
        let fill = left + Vec2::new(BAR_WIDTH * frac, 0.0);

        gizmos.line_2d(left, right, RED);
        gizmos.line_2d(left, fill, GREEN);
    }
}
