use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use early_returns::ok_or_continue;
use game::{starling::math::randvec, z_index::ZOrdering};

use crate::game_version_two::{PartInstance, SelectedSpacecraft};

#[derive(Debug, Clone, Copy)]
struct HoseNode {
    pos: Vec2,
    vel: Vec2,
}

#[derive(Component, Debug)]
pub struct Hose {
    src: Option<Entity>,
    dst: Option<Entity>,
    src_pos: Vec2,
    dst_pos: Vec2,
    desired_length: f32,
    nodes: Vec<HoseNode>,
    opacity: f32,
}

impl Hose {
    fn update_src_pos(&mut self, p: Vec2) {
        self.src_pos = p;
    }

    fn rest_segment_length(&self) -> f32 {
        self.desired_length / (self.nodes.len() - 1) as f32
    }

    fn update_dst_pos(&mut self, p: Vec2) {
        self.dst_pos = p;
    }

    fn is_connected(&self) -> bool {
        self.src.is_some() && self.dst.is_some()
    }

    fn step_node_velocity(&mut self, dt: f32) {
        for i in [0, self.nodes.len() - 1] {
            let rest_pos = if i == 0 {
                if self.src.is_some() {
                    self.src_pos
                } else {
                    continue;
                }
            } else {
                if self.dst.is_some() {
                    self.dst_pos
                } else {
                    continue;
                }
            };
            let mut node = &mut self.nodes[i];
            let accel = (rest_pos - node.pos) * 0.2;
            node.vel += accel * dt;
        }

        let seglen = self.rest_segment_length();

        for i in 1..self.nodes.len() - 1 {
            let before = self.nodes[i - 1];
            let after = self.nodes[i + 1];

            let node = &mut self.nodes[i];

            let u = before.pos - node.pos;
            let v = after.pos - node.pos;

            let avg_vel = (before.vel + after.vel) / 2.0;

            let du = u.length();
            let dv = v.length();

            let u = u.normalize_or_zero();
            let v = v.normalize_or_zero();

            let kp = 6.0;
            let kd = 3.0;

            let au = (du - seglen) * kp * u;
            let av = (dv - seglen) * kp * v;

            let delta_vel = avg_vel - node.vel;

            let accel = au + av + kd * delta_vel;

            node.vel += accel * dt;
        }

        for node in &mut self.nodes {
            node.pos += node.vel * dt;
        }

        let distance = self.src_pos.distance(self.dst_pos);
        let dist_per_segment = distance / (self.nodes.len() - 1) as f32;

        if self.is_connected() {
            if dist_per_segment > self.rest_segment_length() * 2.0 {
                info!("Disconnected");
                self.dst = None;
            }
        }

        if self.src.is_some() {
            self.nodes.first_mut().expect("Expected nonempty list").pos = self.src_pos;
        }
        if self.dst.is_some() {
            self.nodes.last_mut().expect("Expected nonempty list").pos = self.dst_pos;
        }
    }
}

pub fn spawn_hose_on_keypress_system(
    keys: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedSpacecraft>,
    parts: Query<&GlobalTransform, With<PartInstance>>,
    mut commands: Commands,
) -> Option<()> {
    if !keys.just_pressed(KeyCode::KeyH) {
        return Some(());
    }

    let a = selected.primary?;
    let b = selected.secondary?;

    let tf_a = parts.get(a.part).ok()?;
    let tf_b = parts.get(b.part).ok()?;

    let n_segments = 5;

    let pa = tf_a.translation().xy();
    let pb = tf_b.translation().xy();

    let mut nodes = Vec::new();

    let desired_length = pa.distance(pb) * 0.9;

    let n_segments = ((desired_length / 0.2).round() as u32).max(6);

    for i in 0..n_segments + 1 {
        let s = i as f32 / n_segments as f32;
        let p = pa.lerp(pb, s);
        let node = HoseNode {
            pos: p,
            vel: randvec(0.01, 0.05),
        };
        nodes.push(node);
    }

    let hose = Hose {
        src: Some(a.part),
        dst: Some(b.part),
        src_pos: pa,
        dst_pos: pb,
        desired_length,
        nodes,
        opacity: 1.0,
    };

    info!("Spawned hose: {:?}", &hose);

    commands.spawn(hose);

    Some(())
}

pub fn update_hose_physics_system(
    mut commands: Commands,
    mut hoses: Query<(Entity, &mut Hose)>,
    transforms: Query<&GlobalTransform>,
    time: Res<Time<Fixed>>,
) {
    let dt = time.delta_secs();
    for (e, mut hose) in &mut hoses {
        if let Some(src) = hose.src {
            if let Ok(tf) = transforms.get(src) {
                hose.update_src_pos(tf.translation().xy());
            }
        }
        if let Some(dst) = hose.dst {
            if let Ok(tf) = transforms.get(dst) {
                hose.update_dst_pos(tf.translation().xy());
            }
        }

        hose.step_node_velocity(dt);

        if !hose.is_connected() {
            hose.opacity -= dt;
        }

        if hose.opacity < 0.0 {
            commands.entity(e).despawn();
        }
    }
}

pub fn draw_hoses_system(mut painter: ShapePainter, hoses: Query<&Hose>) {
    painter.reset();
    painter.thickness_type = ThicknessType::World;

    let z = ZOrdering::Debug.as_f32();

    for hose in hoses {
        for n in hose.nodes.windows(2) {
            let p = n[0].pos;
            let q = n[1].pos;
            painter.thickness = 0.09;
            painter.set_color(GRAY_800.with_alpha(hose.opacity));
            painter.set_translation(Vec3::Z * z);
            painter.line(p.extend(0.0), q.extend(0.0));
            painter.thickness = 0.06;
            painter.set_color(GRAY_900.with_alpha(hose.opacity));
            painter.set_translation(Vec3::Z * (z + 0.03));
            painter.line(p.extend(0.0), q.extend(0.0));
        }
    }
}
