use bevy::prelude::*;
use bevy_ecs::system::SystemParam;
use game::starling::vehicle::Blueprint;

use crate::game_version_two::SpawnAnimText;

use super::*;

#[derive(SystemParam)]
pub struct Spacecraft<'w, 's> {
    grids: Query<'w, 's, (&'static SpacecraftGrid, &'static Children)>,
    parts: Query<'w, 's, (&'static PartInstance, &'static ChildOf)>,
}

impl<'w, 's> Spacecraft<'w, 's> {
    fn grid_exists(&self, e: Entity) -> bool {
        self.grids.contains(e)
    }

    fn get_vehicle_from_part_id(&self, part: Entity) -> Option<Entity> {
        let (_, child_of) = self.parts.get(part).ok()?;
        Some(child_of.0)
    }

    fn blueprint(&self, e: Entity) -> Option<Blueprint> {
        let mut blueprint = Blueprint::new();
        let (_, children) = self.grids.get(e).ok()?;
        for c in children {
            let (part, _) = self.parts.get(*c).ok()?;
            blueprint.add_part(part.proto.clone(), part.pos, part.rot);
        }
        Some(blueprint)
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

    let v_a = spacecraft.get_vehicle_from_part_id(selected.selected?)?;

    let mut bp = spacecraft.blueprint(v_a)?;

    if let Some(sec) = selected.secondary {
        let v_b = spacecraft.get_vehicle_from_part_id(sec)?;
        let mut bp2: Blueprint = spacecraft.blueprint(v_b)?;
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
