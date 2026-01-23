use crate::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct DockingPort {
    pub attached: PortAttachment,
    pub distance: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum PortAttachment {
    None,
    Seeking(Entity),
    AttachedTo(Entity),
}

impl DockingPort {
    pub fn new(distance: f32) -> Self {
        Self {
            attached: PortAttachment::None,
            distance,
        }
    }

    pub fn target(&self) -> Option<Entity> {
        match self.attached {
            PortAttachment::None => None,
            PortAttachment::Seeking(entity) => Some(entity),
            PortAttachment::AttachedTo(entity) => Some(entity),
        }
    }
}

#[derive(Event, Debug)]
pub struct AttachPorts(Entity);

#[derive(Event, Debug)]
pub struct FuseGrids {
    host_part: Entity,
    target_part: Entity,
}

pub fn inverse_transform(t: Transform) -> Transform {
    let affine = t.compute_affine().inverse();
    let (scale, rot, tr) = affine.to_scale_rotation_translation();
    Transform::from_translation(tr)
        .with_rotation(rot)
        .with_scale(scale)
}

pub fn target_docking_transform(
    ownship: Transform,
    port_a: Transform,
    port_b: Transform,
) -> Transform {
    let inv_port_b = inverse_transform(port_b);
    let rot_180 = Transform::from_rotation(Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI));
    ownship * port_a * rot_180 * inv_port_b
}

pub const DOCKING_PORT_RADIUS: f32 = 6.0;

pub fn check_adjacent_docking_ports(
    mut ports: Query<(Entity, &GlobalTransform, &mut DockingPort, &ChildOf)>,
) {
    #[derive(Clone, Copy)]
    struct PortInfo {
        port: Entity,
        parent: Entity,
        pos: Vec2,
        normal: Dir3,
    }

    let mut port_map: HashMap<IVec2, Vec<PortInfo>> = HashMap::new();

    for (e, transform, mut port, parent) in &mut ports {
        port.attached = PortAttachment::None;

        let pos = transform.translation().xy();
        let normal = transform.right();
        let g = to_grid(pos);

        let info = PortInfo {
            port: e,
            parent: parent.0,
            pos,
            normal,
        };

        port_map
            .entry(g)
            .and_modify(|v| v.push(info))
            .or_insert(vec![info]);
    }

    for (_, entities) in port_map {
        if entities.len() < 2 {
            continue;
        }

        for i in 1..entities.len() {
            for j in 0..i {
                let info_a = &entities[i];
                let info_b = &entities[j];

                // these ports are attached to the same ship!
                if info_a.port == info_b.port {
                    continue;
                }

                let dot_a = info_a
                    .normal
                    .dot((info_b.pos - info_a.pos).extend(0.0).normalize_or_zero())
                    .clamp(0.0, 1.0);

                let dot_b = info_b
                    .normal
                    .dot((info_a.pos - info_b.pos).extend(0.0).normalize_or_zero())
                    .clamp(0.0, 1.0);

                let dot = dot_a * dot_b;

                let distance = info_a.pos.distance(info_b.pos);
                let seeking = distance < DOCKING_PORT_RADIUS && dot > 0.85;

                if seeking {
                    let mut port_a = ok_or_continue!(ports.get_mut(info_a.port));
                    port_a.2.attached = PortAttachment::Seeking(info_b.port);

                    let mut port_b = ok_or_continue!(ports.get_mut(info_b.port));
                    port_b.2.attached = PortAttachment::Seeking(info_a.port);
                }
            }
        }
    }
}

pub fn draw_docking_info(
    mut painter: ShapePainter,
    settings: Res<Settings>,
    ports: Query<(&DockingPort, &GlobalTransform)>,
) {
    if !settings.draw_docking_info {
        return;
    }

    let z = 250.0;

    for (port, tf) in ports {
        let p1 = tf.translation().xy();

        let target_id = some_or_continue!(port.target());
        let (target_port, target_tf) = ok_or_continue!(ports.get(target_id));

        let p2 = target_tf.translation().xy();

        let q1 = p1.extend(z);
        let q2 = p2.extend(z);
        painter.reset();
        painter.set_color(ORANGE);
        painter.hollow = true;
        painter.thickness = 2.0;
        painter.thickness_type = ThicknessType::Pixels;
        painter.line(q1, q2);
        painter.set_translation(q1);
        painter.circle(0.2);
        painter.set_translation(q2);
        painter.circle(0.2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docking_transform() {
        let tf_2d = |x: f32, y: f32, a: f32| {
            Transform::from_xyz(x, y, 0.0).with_rotation(Quat::from_rotation_z(a))
        };

        let ownship = tf_2d(12.0, 13.0, 0.4);
        let port_a = tf_2d(5.0, 1.0, 0.1);
        let port_b = tf_2d(4.0, 0.0, 0.15);

        let target_computed = target_docking_transform(ownship, port_a, port_b);

        let ownship_computed = target_docking_transform(target_computed, port_b, port_a);

        assert_eq!(target_computed.translation.x, 19.97338);
        assert_eq!(target_computed.translation.y, 17.239744);
        assert_eq!(target_computed.translation.z, 0.0);

        assert_eq!(target_computed.rotation.x, 0.0);
        assert_eq!(target_computed.rotation.y, 0.0);
        assert_eq!(target_computed.rotation.z, 0.98472655);
        assert_eq!(target_computed.rotation.w, -0.17410818);

        assert_eq!(ownship_computed.translation.x, 12.000002);
        assert_eq!(ownship_computed.translation.y, 12.999999);
        assert_eq!(ownship_computed.translation.z, 0.0);

        assert_eq!(ownship_computed.rotation.x, 0.0);
        assert_eq!(ownship_computed.rotation.y, 0.0);
        assert_eq!(ownship_computed.rotation.z, -0.19866942);
        assert_eq!(ownship_computed.rotation.w, -0.98006666);
    }
}
