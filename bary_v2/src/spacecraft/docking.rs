use bary_core::prelude::GridPlacement;
use bary_core::prelude::*;
use bary_v1::ui::apply_egui_style;
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use early_returns::ok_or_continue;

use crate::{PartInstance, SpacecraftGrid, spacecraft::sysparam_api::Spacecraft};

use super::{PartsResource, SelectedSpacecraft};

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

#[derive(Event, Debug, Clone, Copy)]
pub struct DockingTrigger {
    pub chief: Entity,
    pub deputy: Entity,
    pub offset: PartCoord,
    pub rotation: Rotation,
}

pub fn docking_program_egui(
    mut contexts: EguiContexts,
    mut pgrm: ResMut<DockingProgram>,
    names: Query<&Name>,
    selected: Res<SelectedSpacecraft>,
    mut triggers: EventWriter<DockingTrigger>,
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
            let trigger = DockingTrigger {
                chief: a,
                deputy: b,
                offset: pgrm.offset,
                rotation: pgrm.rotation,
            };
            triggers.write(trigger);
        }

        if ui.button("Reset").clicked() {
            *pgrm = DockingProgram::default();
        }
    });

    None
}

pub fn draw_blueprint(
    gizmos: &mut Gizmos,
    bp: &Blueprint,
    transform: Transform,
    parts: &PartDatabase,
) {
    for (_, part) in bp.parts() {
        if part.layer() == PartLayer::Exterior {
            continue;
        }

        let part_center = part.center_meters();
        let dims = part.dims_meters();
        let Some(proto) = parts.get(&part.name) else {
            continue;
        };

        let color = diagram_color(proto);

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
    parts: Res<PartsResource>,
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
    draw_blueprint(&mut gizmos, &sec_bp, sec_tf, &parts);

    Some(())
}

pub fn process_docking_triggers(
    mut commands: Commands,
    mut reader: EventReader<DockingTrigger>,
    grids: Query<&Children, With<SpacecraftGrid>>,
    mut parts: Query<(&mut Transform, &mut PartInstance)>,
    mut blueprints: Query<&mut Blueprint>,
) {
    for trigger in reader.read() {
        info!("Docking: {:?}", trigger);

        let mut bpa = ok_or_continue!(blueprints.get_mut(trigger.chief));

        let deputy = ok_or_continue!(grids.get(trigger.deputy));

        for d in deputy {
            let (mut transform, mut instance) = ok_or_continue!(parts.get_mut(*d));

            let new_placement = GridPlacement::new(
                instance.placement.bottom_left() + trigger.offset,
                instance.rotation(),
                instance.placement.part_aligned_dims().inner().as_uvec2(),
            );

            instance.placement = new_placement;

            bpa.add_part(instance.name.clone(), instance.placement, instance.layer());

            *transform = isometry_to_transform(instance.placement.center_isometry());

            commands.entity(*d).insert(ChildOf(trigger.chief));
        }
    }
}
