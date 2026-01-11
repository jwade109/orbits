use bevy::prelude::*;
use bevy_ecs::{query::QueryEntityError, system::SystemParam};
use game::starling::vehicle::Blueprint;

use crate::game_version_two::{SpawnAnimText, target_docking_transform};

use super::*;

#[derive(SystemParam)]
pub struct Spacecraft<'w, 's> {
    grids: Query<
        'w,
        's,
        (
            &'static mut SpacecraftGrid,
            &'static Children,
            &'static mut Transform,
        ),
    >,
    parts: Query<
        'w,
        's,
        (
            &'static PartInstance,
            &'static ChildOf,
            &'static mut Transform,
        ),
        Without<SpacecraftGrid>,
    >,
}

impl<'w, 's> Spacecraft<'w, 's> {
    pub fn grid_exists(&self, e: Entity) -> bool {
        self.grids.contains(e)
    }

    pub fn get_vehicle_from_part_id(&self, part: Entity) -> Result<Entity, QueryEntityError> {
        let (_, child_of, _) = self.parts.get(part)?;
        Ok(child_of.0)
    }

    pub fn blueprint(&self, e: Entity) -> Result<Blueprint, QueryEntityError> {
        let mut blueprint = Blueprint::new();
        let (_, children, _) = self.grids.get(e)?;
        for c in children {
            let (part, _, _) = self.parts.get(*c)?;
            blueprint.add_part(part.proto.clone(), part.pos, part.rot);
        }
        Ok(blueprint)
    }

    pub fn merge_grids(
        &mut self,
        host_part: Entity,
        target_part: Entity,
    ) -> Result<(), QueryEntityError> {
        info!("Merge grids with parts {}, {}", host_part, target_part);
        let host_vehicle = self.get_vehicle_from_part_id(host_part)?;
        let target_vehicle = self.get_vehicle_from_part_id(target_part)?;
        if host_vehicle == target_vehicle {
            warn!("Tried to merge two parts of same grid: {}", host_vehicle);
            return Ok(());
        }

        let (port, parent, port_a) = self.parts.get(host_part)?;
        let (other_port, other_parent, port_b) = self.parts.get(target_part)?;

        let (grid, children, ownship_root) = self.grids.get(parent.0)?;

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

        let off_a = port.prototype().dims_meters().x;
        let off_b = other_port.prototype().dims_meters().x;

        port_a.translation += port_a.right() * off_a / 2.0;
        port_b.translation += port_b.right() * off_b / 2.0;

        let (mut grid, _, mut target_root) = self.grids.get_mut(other_parent.0)?;

        *target_root = target_docking_transform(ownship_root, port_a, port_b);

        grid.velocity = velocity + additional_velocity;
        grid.angular_velocity = angular_velocity;

        Ok(())
    }
}

pub fn draw_blueprint_system(
    spacecraft: Spacecraft,
    selected: Res<SelectedSpacecraft>,
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: EventWriter<SpawnAnimText>,
) -> Option<()> {
    if !keys.just_pressed(KeyCode::KeyO) {
        return Some(());
    }

    let v_a = spacecraft
        .get_vehicle_from_part_id(selected.selected?)
        .ok()?;

    let mut bp = spacecraft.blueprint(v_a).ok()?;

    if let Some(sec) = selected.secondary {
        let v_b = spacecraft.get_vehicle_from_part_id(sec).ok()?;
        let mut bp2: Blueprint = spacecraft.blueprint(v_b).ok()?;
        bp2.rotate_ccw();
        bp2.shift(bp.dims().as_ivec2().into());
        bp.merge(&bp2);
    }

    let Some(img) = game::starling::vehicle::generate_image(&bp) else {
        return Some(());
    };

    _ = img.save("/tmp/out.png");

    writer.write(SpawnAnimText::new("Wrote vehicle blueprint to file"));

    Some(())
}

pub fn swallow_optional(In(_): In<Option<()>>) {}
