use bevy::prelude::*;
use bevy_ecs::{query::QueryEntityError, system::SystemParam};
use bevy_vector_shapes::prelude::*;
use game::{
    starling::{
        parts::PartLayer,
        vehicle::{Blueprint, diagram_color},
    },
    z_index::ZOrdering,
};

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

    pub fn get_part_at(&self, coords: GridCoord) -> Option<Entity> {
        let (_, children, _) = self.grids.get(coords.entity).ok()?;

        if children.is_empty() {
            warn!("Empty grid!");
            return None;
        }

        // TODO this is very bugged.
        for id in children {
            let (part, _, _) = self.parts.get(*id).ok()?;
            if part.prototype().layer() != PartLayer::Internal {
                continue;
            }
            let offset = (coords.coord - part.origin()).inner();
            let dims = part.dims.as_ivec2();
            if offset.x >= 0 && offset.y >= 0 && offset.x <= dims.x && offset.y <= dims.y {
                return Some(*id);
            }
        }

        None
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

pub fn draw_blueprint_of_current_ship_system(
    mut painter: ShapePainter,
    spacecraft: Spacecraft,
    selected: Res<SelectedSpacecraft>,
) -> Option<()> {
    let id = selected.primary?.grid;
    let bp = spacecraft.blueprint(id.entity).ok()?;

    let z = ZOrdering::Debug2.as_f32();

    for (_, part) in bp.parts() {
        if part.layer() == PartLayer::Exterior {
            continue;
        }

        let zoff = part.layer().to_z() as f32 / 100.0;

        let center = part.center_meters();
        let dims = part.dims_meters();
        let color = diagram_color(&part.proto);

        painter.reset();
        painter.set_color(color.with_alpha(0.7));
        painter.set_translation(center.extend(z + zoff));
        painter.rect(dims);
        painter.hollow = true;
        painter.thickness = 0.01;
        painter.set_color(Srgba::gray(0.3));
        painter.rect(dims);
    }

    Some(())
}

pub fn export_blueprint_system(
    spacecraft: Spacecraft,
    selected: Res<SelectedSpacecraft>,
    keys: Res<ButtonInput<KeyCode>>,
    mut writer: EventWriter<SpawnAnimText>,
) -> Option<()> {
    if !keys.just_pressed(KeyCode::KeyO) {
        return Some(());
    }

    let v_a = spacecraft
        .get_vehicle_from_part_id(selected.primary?.part.entity)
        .ok()?;

    let mut bp = spacecraft.blueprint(v_a).ok()?;

    if let Some(sec) = selected.secondary {
        let v_b = spacecraft.get_vehicle_from_part_id(sec.part.entity).ok()?;
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

pub fn swallow_result(In(_): In<Result>) {}
