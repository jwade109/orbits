use bary_core::prelude::*;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::*;
use early_returns::{ok_or_continue, ok_or_return, some_or_return};

use crate::CursorWorldPosition;

use super::grid_coord::GridCoord;
use super::*;

#[derive(Debug, Clone, Copy)]
pub struct SelectedPointInfo {
    pub grid: GridCoord,
    pub part: GridCoord,
}

#[derive(Resource, Debug, Default)]
pub struct SelectedSpacecraft {
    pub hovered: Option<SelectedPointInfo>,
    pub primary: Option<SelectedPointInfo>,
    pub secondary: Option<SelectedPointInfo>,
}

pub fn update_selected_spacecraft_system(
    // mut gizmos: Gizmos,
    mut cursor: ResMut<SelectedSpacecraft>,
    map: Res<GridSpatialLookup>,
    pos: Res<CursorWorldPosition>,
    grids: Query<(&GlobalTransform, &Children), With<SpacecraftGrid>>,
    parts: Query<(Entity, &PartInstance)>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    cursor.hovered = None;

    let mouse_pos = some_or_return!(pos.get());
    let grid_ids = some_or_return!(map.lup(mouse_pos));

    'outer: for grid_id in grid_ids {
        let (transform, children) = ok_or_return!(grids.get(*grid_id));

        if children.is_empty() {
            warn!("Empty grid!");
            continue;
        }

        let grid_origin = transform.translation().xy();

        let offset = mouse_pos - grid_origin;
        let (yaw, _pitch, _roll) = transform.rotation().to_euler(EulerRot::ZYX);
        let rot = Vec2::from_angle(-yaw);
        let offset = rot.rotate(offset);

        let go_grid = PartCoord::from_meters_floored(offset);

        for id in children {
            let (e, part) = ok_or_continue!(parts.get(*id));
            if part.layer() != PartLayer::Internal {
                continue;
            }

            let gp_grid = part.origin();
            let dims_meters = part.dims_meters();

            let rot = Quat::from_rotation_z(yaw);
            let part_origin = Transform::from_isometry(Isometry3d::new(
                (grid_origin + rotate(gp_grid.to_meters(), yaw)).extend(0.0),
                rot,
            ));

            let part_local = in_frame(part_origin, mouse_pos);

            // gizmos.axes(part_origin, dims_meters.min_element());

            if part_local.x >= 0.0
                && part_local.y >= 0.0
                && part_local.x < dims_meters.x
                && part_local.y < dims_meters.y
            {
                // gizmos.arrow_2d(grid_origin, mouse_pos, GREEN);
                // gizmos.arrow_2d(part_origin.translation.xy(), mouse_pos, GREEN);

                let grid = GridCoord {
                    entity: *grid_id,
                    coord: go_grid,
                };

                let part = GridCoord {
                    entity: e,
                    coord: IVec2::ZERO.into(),
                };

                let info = SelectedPointInfo { grid, part };

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
) {
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

        if let Ok((tf, part)) = parts.get(e.part.entity) {
            let dims = part.placement.part_aligned_dims().to_meters();

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

        if let Ok(tf) = grids.get(e.grid.entity) {
            painter.set_translation(tf.translation().with_z(Z_SELECTED_PART_OUTLINE));
            painter.set_rotation(tf.rotation());
            let offset = e.grid.coord.to_meters();
            painter.translate(offset.extend(0.0));
            let dims = Vec2::splat(PartCoord::CELL_WIDTH);
            painter.translate(dims.extend(0.0) / 2.0);
            painter.set_color(color);
            painter.rect(dims);
        }
    }
}
