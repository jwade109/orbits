use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use early_returns::{ok_or_continue, ok_or_return, some_or_return};
use game::starling::parts::{PartCoord, PartLayer};

use crate::game_version_two::CursorWorldPosition;

use super::grid_coord::GridCoord;
use super::*;

#[derive(Debug, Clone, Copy)]
pub struct SelectedPointInfo {
    pub grid: Entity,
    pub part: Entity,
    pub coord: PartCoord,
}

impl SelectedPointInfo {
    pub fn grid_coord(&self) -> GridCoord {
        GridCoord {
            grid: self.grid,
            coord: self.coord,
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct SelectedSpacecraft {
    pub hovered: Option<SelectedPointInfo>,
    pub primary: Option<SelectedPointInfo>,
    pub secondary: Option<SelectedPointInfo>,
}

pub fn update_selected_spacecraft_system(
    mut cursor: ResMut<SelectedSpacecraft>,
    map: Res<GridSpatialLookup>,
    pos: Res<CursorWorldPosition>,
    grids: Query<(&GlobalTransform, &Children), With<SpacecraftGrid>>,
    parts: Query<(Entity, &PartInstance)>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    cursor.hovered = None;

    let pos = some_or_return!(pos.get());
    let grid_ids = some_or_return!(map.lup(pos));

    'outer: for grid_id in grid_ids {
        let (transform, children) = ok_or_return!(grids.get(*grid_id));

        if children.is_empty() {
            warn!("Empty grid!");
            continue;
        }

        let offset = pos - transform.translation().xy();
        let (yaw, _pitch, _roll) = transform.rotation().to_euler(EulerRot::ZYX);
        let rot = Vec2::from_angle(-yaw);
        let offset = rot.rotate(offset);

        let coord = PartCoord::from_meters_floored(offset);

        for id in children {
            let (e, part) = ok_or_continue!(parts.get(*id));
            if part.prototype().layer() != PartLayer::Internal {
                continue;
            }
            let origin = part.origin_meters();
            let dims = part.dims_meters();
            let part_offset = offset - origin;
            if part_offset.x >= 0.0
                && part_offset.y >= 0.0
                && part_offset.x <= dims.x
                && part_offset.y <= dims.y
            {
                let info = SelectedPointInfo {
                    grid: *grid_id,
                    part: e,
                    coord,
                };

                cursor.hovered = Some(info);
                break 'outer;
            }
        }
    }

    if buttons.just_pressed(MouseButton::Left) {
        cursor.primary = cursor.hovered;
    }

    if buttons.just_pressed(MouseButton::Right) {
        cursor.secondary = cursor.hovered;
    }
}

pub fn draw_selected_part_system(
    mut painter: ShapePainter,
    grids: Query<&GlobalTransform, With<SpacecraftGrid>>,
    parts: Query<(&GlobalTransform, &PartInstance)>,
    sel: Res<SelectedSpacecraft>,
    time: Res<Time>,
) {
    let angle = time.elapsed_secs_f64() % (2.0 * std::f64::consts::PI);
    let angle = angle as f32;

    const Z_SELECTED_PART: f32 = 1.0;
    const Z_SELECTED_PART_OUTLINE: f32 = 1.001;

    for (color, e) in [
        (RED.with_alpha(0.8), sel.hovered),
        (ORANGE, sel.primary),
        (TEAL, sel.secondary),
    ] {
        let e = match e {
            Some(e) => e,
            None => continue,
        };

        if let Ok((tf, part)) = parts.get(e.part) {
            let dims = part.prototype().dims_meters();
            let r = dims.length() / 2.0 + 0.5;
            painter.reset();
            painter.set_translation(tf.translation().with_z(Z_SELECTED_PART));
            painter.set_rotation(tf.rotation());
            let dims = dims + Vec2::splat(0.2);

            painter.hollow = false;
            painter.set_color(color.with_alpha(0.4));
            painter.rect(dims);

            // outline
            painter.set_color(color);
            painter.thickness = 0.03;
            painter.hollow = true;
            painter.thickness_type = ThicknessType::World;
            painter.rect(dims);
        }

        if let Ok(tf) = grids.get(e.grid) {
            painter.set_translation(tf.translation().with_z(Z_SELECTED_PART_OUTLINE));
            painter.set_rotation(tf.rotation());
            let offset = e.coord.to_meters();
            painter.translate(offset.extend(0.0));
            let dims = Vec2::splat(PartCoord::CELL_WIDTH);
            painter.translate(dims.extend(0.0) / 2.0);
            painter.set_color(color);
            painter.rect(dims);
        }
    }
}
