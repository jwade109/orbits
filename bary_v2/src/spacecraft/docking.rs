use bary_core::prelude::GridPlacement;
use bary_core::prelude::*;
use bary_v1::ui::apply_egui_style;
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use early_returns::{ok_or_continue, ok_or_return};

use crate::{DockingTrigger, PartInstance, SpacecraftGrid, spacecraft::sysparam_api::Spacecraft};

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

impl DockingProgram {
    pub fn isometry(&self) -> Isometry2d {
        Isometry2d::new(
            self.offset.to_meters(),
            (self.rotation.to_angle() as f32).into(),
        )
    }
}

impl DockingTrigger {
    pub fn isometry(&self) -> Isometry2d {
        Isometry2d::new(
            self.offset.to_meters(),
            (self.rotation.to_angle() as f32).into(),
        )
    }
}

pub fn docking_program_egui(
    mut commands: Commands,
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
            let trigger = DockingTrigger {
                chief: a,
                deputy: b,
                offset: pgrm.offset,
                rotation: pgrm.rotation,
            };
            commands.trigger(trigger);
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
    isometry: Isometry2d,
    parts: &PartDatabase,
) {
    for (_, part) in bp.parts() {
        if part.layer() == PartLayer::Exterior {
            continue;
        }
        let part_isometry = part.placement.center_isometry();
        let dims = part.placement.part_aligned_dims().to_meters();
        let color = if let Some(proto) = parts.get(&part.name) {
            diagram_color(proto)
        } else {
            continue;
        };
        let total_isometry = isometry * part_isometry;
        gizmos.rect_2d(total_isometry, dims, color);
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
    let pri_isometry = transform_to_isometry(pri_tf);
    let docking_isometry = pgrm.isometry();
    let sec_isometry = pri_isometry * docking_isometry;
    draw_blueprint(&mut gizmos, &sec_bp, sec_isometry, &parts);

    Some(())
}

pub fn process_docking_triggers(
    trigger: On<DockingTrigger>,
    mut commands: Commands,
    grids: Query<&Children, With<SpacecraftGrid>>,
    mut parts: Query<(&mut Transform, &mut PartInstance)>,
    mut blueprints: Query<&mut Blueprint>,
) {
    info!("Docking: {:?}, {:?}", trigger, trigger.isometry());

    let mut bpa = ok_or_return!(blueprints.get_mut(trigger.chief));

    let deputy = ok_or_return!(grids.get(trigger.deputy));

    for d in deputy {
        let (mut transform, mut instance) = ok_or_continue!(parts.get_mut(*d));

        let mut new_placement = instance.placement;
        new_placement.rotate(trigger.rotation);
        new_placement.shift(trigger.offset);
        instance.placement = new_placement;

        bpa.add_part(instance.name.clone(), instance.placement, instance.layer());

        *transform = isometry_to_transform(instance.placement.center_isometry());

        commands.entity(*d).insert(ChildOf(trigger.chief));
    }
}
