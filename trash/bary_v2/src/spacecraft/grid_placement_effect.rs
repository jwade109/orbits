use bary_core::prelude::{GridPlacement, transform_to_isometry};
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use early_returns::ok_or_continue;
use std::time::Duration;

use crate::Spacecraft;

#[derive(Component)]
pub struct GridPlacementEffect {
    pub grid: Entity,
    pub placement: GridPlacement,
    pub age: Duration,
}

impl GridPlacementEffect {
    pub fn new(grid: Entity, placement: GridPlacement) -> Self {
        Self {
            grid,
            placement,
            age: Duration::ZERO,
        }
    }

    fn thickness(&self) -> f32 {
        (0.2 - self.age.as_secs_f32() * 0.7).max(0.0)
    }

    fn additional_size(&self) -> Vec2 {
        Vec2::splat(self.age.as_secs_f32())
    }
}

pub fn update_grid_placement_effects(
    effects: Query<&mut GridPlacementEffect>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta();
    for mut effect in effects {
        effect.age += dt;
    }
}

pub fn despawn_grid_placement_effects(
    mut commands: Commands,
    effects: Query<(Entity, &GridPlacementEffect)>,
) {
    for (e, effect) in effects {
        if effect.thickness() <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}

pub fn draw_grid_placement_effects(
    mut painter: ShapePainter,
    effects: Query<&GridPlacementEffect>,
    spacecraft: Spacecraft,
) {
    painter.reset();
    painter.set_color(ORANGE);
    for effect in effects {
        let t = effect.thickness();
        if t <= 0.0 {
            continue;
        }

        let (tf, size) =
            ok_or_continue!(spacecraft.placement_transform(effect.grid, effect.placement));
        painter.transform = tf;
        painter.transform.translation.z = 20.0;
        painter.hollow = true;
        painter.thickness = t;
        painter.rect(size + effect.additional_size());
    }
}
