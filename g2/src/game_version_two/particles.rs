use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;

use crate::game_version_two::*;

pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ThrustParticleConfig::default())
            .add_systems(FixedUpdate, thrust_particles)
            .add_systems(PostUpdate, debug_draw_emitters.in_set(Sets::Draw));
        // .add_plugins(ResourceInspectorPlugin::<ThrustParticleConfig>::new());
    }
}

#[derive(Component, Debug)]
#[require(Transform)]
struct ThrustParticle {
    time_remaining: f32,
    velocity: Vec2,
    nominal_position: Vec2,
}

#[derive(Component, Debug, Reflect)]
#[require(Transform)]
pub struct ParticleEmitter {
    pub enabled: bool,
    pub size: Vec3,
}

#[derive(Resource, Reflect, Debug)]
pub struct ThrustParticleConfig {
    color_a: Color,
    color_b: Color,
    mean_velocity: f32,
    velocity_spread: f32,
    spread: f32,
    discrete: bool,
    paused: bool,
    step: bool,
    squares: bool,
    particles_per_tick: usize,
    draw_boxes: bool,
}

impl Default for ThrustParticleConfig {
    fn default() -> Self {
        ThrustParticleConfig {
            color_a: RED.with_alpha(0.8).into(),
            color_b: YELLOW.with_alpha(0.6).into(),
            mean_velocity: 20.0,
            velocity_spread: 3.0,
            spread: 1.0,
            discrete: false,
            paused: false,
            step: false,
            squares: true,
            particles_per_tick: 8,
            draw_boxes: false,
        }
    }
}

fn thrust_particles(
    mut commands: Commands,
    grids: Query<&SpacecraftGrid>,
    emitters: Query<(&GlobalTransform, &ParticleEmitter, &ChildOf)>,
    mut particles: Query<(Entity, &mut ThrustParticle, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut cfg: ResMut<ThrustParticleConfig>,
    time_fixed: Res<Time>,
) {
    if cfg.step {
        cfg.paused = true;
    }

    if cfg.paused && !cfg.step {
        return;
    }

    cfg.step = false;

    let discrete_n = 3;
    let particle_size = 0.07;

    for (tf, emitter, parent) in &emitters {
        if !emitter.enabled {
            continue;
        }

        let grid = ok_or_continue!(grids.get(parent.0));

        for _ in 0..cfg.particles_per_tick {
            let color = cfg.color_a.mix(&cfg.color_b, rand(0.1, 0.9));

            let x = rand(-1.0, 1.0) * emitter.size.x / 2.0;
            let y = rand(-1.0, 1.0) * emitter.size.y / 2.0;

            let pos = Vec2::new(x, y) + tf.translation().xy();

            let angle = rand(-1.0, 1.0) * 0.3;
            let vel = (tf.left() * cfg.mean_velocity).xy() * rand(0.3, 1.0);
            let vel = grid.velocity.as_vec2() + rotate(vel, angle);

            let dpos = if cfg.discrete {
                (pos * discrete_n as f32).round() / discrete_n as f32
            } else {
                pos
            };

            commands.spawn((
                ThrustParticle {
                    time_remaining: rand(0.05, 0.1),
                    velocity: vel,
                    nominal_position: pos,
                },
                Transform::from_translation(dpos.extend(1.0)),
                if cfg.squares {
                    Mesh2d(meshes.add(Rectangle::from_size(Vec2::splat(particle_size))))
                } else {
                    Mesh2d(meshes.add(Circle::new(particle_size)))
                },
                MeshMaterial2d(materials.add(Color::from(color))),
                Pickable::IGNORE,
            ));
        }
    }

    let dt = time_fixed.delta_secs();

    for (e, mut part, mut tf) in &mut particles {
        part.time_remaining -= dt;
        // part.velocity.y += 6.0 * dt;
        part.nominal_position = part.nominal_position + part.velocity * dt;
        tf.translation = if cfg.discrete {
            (part.nominal_position * discrete_n as f32).round() / discrete_n as f32
        } else {
            part.nominal_position
        }
        .extend(1.0);
        if part.time_remaining < 0.0 {
            commands.entity(e).despawn();
        }
    }
}

fn debug_draw_emitters(
    cfg: Res<ThrustParticleConfig>,
    mut gizmos: Gizmos,
    emitters: Query<(&GlobalTransform, &ParticleEmitter)>,
) {
    if !cfg.draw_boxes {
        return;
    }

    for (tf, emitter) in &emitters {
        let color = if emitter.enabled {
            RED
        } else {
            BLUE.with_alpha(0.2)
        };
        gizmos.primitive_3d(
            &Cuboid::from_size(emitter.size),
            Isometry3d::from_translation(tf.translation()),
            color,
        );
    }
}
