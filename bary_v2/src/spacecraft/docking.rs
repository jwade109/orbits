use bary_core::prelude::*;
use bary_v1::ui::apply_egui_style;
use bevy::prelude::*;
use bevy_egui::EguiContexts;

use crate::spacecraft::sysparam_api::Spacecraft;

use super::SelectedSpacecraft;

#[derive(Debug, Resource)]
pub struct DockingProgram {
    pub offset: PartCoord,
    pub rotation: Rotation,
}

impl Default for DockingProgram {
    fn default() -> Self {
        Self {
            offset: PartCoord::default(),
            rotation: Rotation::East,
        }
    }
}

pub fn docking_program_egui(
    mut contexts: EguiContexts,
    mut pgrm: ResMut<DockingProgram>,
    names: Query<&Name>,
    selected: Res<SelectedSpacecraft>,
) -> Option<()> {
    let a = selected.primary?.grid.entity;
    let b = selected.secondary?.grid.entity;

    if a == b {
        return None;
    }

    let pri_name = names.get(a).ok()?;
    let sec_name = names.get(b).ok()?;

    let ctx = contexts.ctx_mut().ok()?;

    use egui::*;

    Window::new("Docking Program").show(ctx, |ui| {
        apply_egui_style(ui);

        ui.heading(format!("Chief: {}", pri_name));
        ui.heading(format!("Deputy: {}", sec_name));

        ui.separator();

        ui.label(format!("Offset: {:?}", pgrm.offset));

        let mut off = pgrm.offset.inner();

        ui.add(Slider::new(&mut off.x, -10..=10).clamping(SliderClamping::Never));
        ui.add(Slider::new(&mut off.y, -10..=10).clamping(SliderClamping::Never));

        pgrm.offset = off.into();

        ui.label(format!("Rotation: {:?}", pgrm.rotation));

        if ui.button("Rotate").clicked() {
            pgrm.rotation = pgrm.rotation.next();
        }

        ui.separator();

        if ui.button("Dock").clicked() {
            info!("Docking: {:?}", pgrm);
        }

        if ui.button("Reset").clicked() {
            *pgrm = DockingProgram::default();
        }
    });

    None
}

pub fn draw_blueprint(gizmos: &mut Gizmos, bp: &Blueprint, transform: Transform) {
    for (_, part) in bp.parts() {
        if part.layer() == PartLayer::Exterior {
            continue;
        }

        let part_center = part.center_meters();
        let dims = part.dims_meters();
        let color = diagram_color(&part.proto);

        let (yaw, _pitch, _roll) = transform.rotation.to_euler(EulerRot::ZYX);

        let transform_center =
            transform.translation.xy() + Vec2::from_angle(yaw).rotate(part_center);

        let iso = Isometry2d::new(transform_center, yaw.into());

        gizmos.rect_2d(iso, dims, color);
    }
}

pub fn draw_blueprint_of_docking_program(
    mut gizmos: Gizmos,
    spacecraft: Spacecraft,
    selected: Res<SelectedSpacecraft>,
    pgrm: ResMut<DockingProgram>,
) -> Option<()> {
    let a = selected.primary?.grid.entity;
    let b = selected.secondary?.grid.entity;

    if a == b {
        return None;
    }

    let sec_bp = spacecraft.blueprint(b).ok()?;

    let pri_tf = spacecraft.grid_transform(a).ok()?;

    let mut sec_tf = pri_tf;

    let offset_meters = pgrm.offset.to_meters();

    sec_tf.translation += sec_tf.local_x() * offset_meters.x;
    sec_tf.translation += sec_tf.local_y() * offset_meters.y;

    sec_tf.rotate_local_z(pgrm.rotation.to_angle() as f32);

    gizmos.axes_2d(pri_tf, 2.0);

    gizmos.axes_2d(sec_tf, 2.0);
    draw_blueprint(&mut gizmos, &sec_bp, sec_tf);

    Some(())
}
