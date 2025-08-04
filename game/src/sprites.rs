use crate::game::GameState;
use crate::scenes::*;
use bevy::prelude::*;

pub fn hashable_to_color(h: &impl std::hash::Hash) -> Hsla {
    use std::hash::Hasher;
    let mut s = std::hash::DefaultHasher::new();
    h.hash(&mut s);
    let h: u64 = s.finish() % 1000;
    let hue = 360.0 * (h as f32 / 1000 as f32);
    Hsla::new(hue, 1.0, 0.5, 1.0)
}

pub fn update_background_color(
    mut camera: Single<&mut Camera, With<crate::game::BackgroundCamera>>,
    state: Res<GameState>,
) {
    let c = GameState::background_color(&state);

    camera.clear_color = ClearColorConfig::Custom(c.with_alpha(0.0).into());
}

#[derive(Component)]
pub struct StaticSprite(usize, String);

pub fn update_static_sprites(
    mut commands: Commands,
    state: Res<GameState>,
    mut query: Query<(Entity, &mut Sprite, &mut Transform, &mut StaticSprite)>,
) {
    let sprites: Vec<StaticSpriteDescriptor> = state.sprites.clone();

    let mut sprite_entities: Vec<_> = query.iter_mut().collect();

    for (i, sprite) in sprites.iter().enumerate() {
        let pos = sprite.position.extend(sprite.z_index);

        let handle = state
            .image_handles
            .get(&sprite.path)
            .or(state.image_handles.get("wmata7000"));

        let (handle, dims) = if let Some((handle, dims)) = handle {
            (handle.clone(), dims.as_vec2())
        } else {
            (Handle::default(), Vec2::splat(100.0))
        };

        let sx = sprite.dims.x / dims.x;
        let sy = sprite.dims.y / dims.y;

        let transform = Transform::from_scale(Vec3::new(sx, sy, 1.0))
            .with_translation(pos)
            .with_rotation(Quat::from_rotation_z(sprite.angle));

        let ent = sprite_entities.iter_mut().find(|(_, _, _, ss)| ss.0 == i);

        let mut new_sprite = Sprite::from_image(handle);
        if let Some(c) = sprite.color {
            new_sprite.color = Color::Srgba(c);
        }

        if let Some((_, ref mut spr, ref mut tf, ref mut desc)) = ent {
            **tf = transform;
            **spr = new_sprite;
            desc.1 = sprite.path.clone();
        } else {
            // println!("[{}] ({}) New sprite {}", state.wall_time, i, path);
            commands.spawn((new_sprite, transform, StaticSprite(i, sprite.path.clone())));
        }
    }

    for (e, _, _, ss) in &query {
        if ss.0 >= sprites.len() {
            // println!("[{}] ({}) Deleting sprite {}", state.wall_time, ss.0, ss.1);
            commands.entity(e).despawn();
        }
    }
}
