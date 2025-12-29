use crate::game_version_two::*;

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
    host_grid: Entity,
    target_grid: Entity,
}

pub fn send_attach_events(
    keys: Res<ButtonInput<KeyCode>>,
    mut attach: EventWriter<AttachPorts>,
    cursor: Res<SelectedSpacecraft>,
) {
    if !keys.pressed(KeyCode::KeyJ) {
        return;
    }

    let id = some_or_return!(cursor.selected);
    let event = AttachPorts(id);
    attach.write(event);
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

pub fn on_attach_event(
    mut fuse: EventWriter<FuseGrids>,
    mut attach: EventReader<AttachPorts>,
    ports: Query<(&DockingPort, &Transform, &ChildOf), Without<SpacecraftGrid>>,
    mut grids: Query<(&mut Transform, &mut SpacecraftGrid)>,
) {
    for msg in attach.read() {
        let (port, port_a, parent) = ok_or_continue!(ports.get(msg.0));
        let target = some_or_continue!(port.target());
        let (other_port, port_b, other_parent) = ok_or_continue!(ports.get(target));

        if parent.0 == other_parent.0 {
            // warn!("Parents of attached ports are the same: {}", parent.0);
            continue;
        }

        info!("Joining grids: {}, {}", parent.0, other_parent.0);

        let (ownship_root, grid) = ok_or_continue!(grids.get(parent.0));

        let ownship_root = ownship_root.clone();
        let velocity = grid.velocity;
        let angular_velocity = grid.angular_velocity;

        let additional_velocity = port_a
            .translation
            .cross((Vec3::Z * angular_velocity as f32))
            .xy()
            .as_dvec2();

        let mut port_a = *port_a;
        let mut port_b = *port_b;

        port_a.translation += port_a.local_x() * port.distance;
        port_b.translation += port_b.local_x() * other_port.distance;

        // let port_a = Transform::from_translation(port_a.translation + port_a.forward() * 2.0);
        // let port_b = Transform::from_translation(port_b.translation + port_b.forward() * 2.0);

        let (mut target_root, mut grid) = ok_or_continue!(grids.get_mut(other_parent.0));

        *target_root = target_docking_transform(ownship_root, port_a, port_b);

        grid.velocity = velocity + additional_velocity;
        grid.angular_velocity = angular_velocity;

        // fuse.write(FuseGrids {
        //     host_grid: parent.0,
        //     target_grid: other_parent.0,
        // });
    }
}

pub fn on_fuse_grids_event(
    mut commands: Commands,
    mut events: EventReader<FuseGrids>,
    grids: Query<&Children, With<SpacecraftGrid>>,
    mut parts: Query<&mut Transform, With<PartInstance>>,
) {
    for event in events.read() {
        info!("Fusing grids {} and {}", event.host_grid, event.target_grid);
        let target_grid = ok_or_continue!(grids.get(event.target_grid));
        for t in target_grid {
            commands.entity(*t).set_parent_in_place(event.host_grid);
        }
    }
}

pub const DOCKING_PORT_RADIUS: f32 = 6.0;

pub fn check_adjacent_docking_ports(
    mut commands: Commands,
    mut painter: ShapePainter,
    mut ports: Query<(Entity, &GlobalTransform, &mut DockingPort, &ChildOf)>,
) {
    #[derive(Clone, Copy)]
    struct PortInfo {
        port: Entity,
        parent: Entity,
        pos: Vec2,
        normal: Dir3,
    };

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
