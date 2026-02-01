use bary_core::prelude::GridPlacement;
use bary_core::prelude::*;
use bary_v1::ui::apply_egui_style;
use bary_v1::z_index::ZOrdering;
use bevy::color::palettes::css::*;
use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_vector_shapes::prelude::*;
use early_returns::*;
use egui::Pos2;

use super::grid_coord::GridCoord;

use crate::AddHose;
use crate::AddPipe;
use crate::CellPosition;
use crate::FailedLookup;
use crate::GridPlacementEffect;
use crate::InventoryApi;
use crate::Settings;
use crate::SpawnAnimText;
use crate::add_slot_widget;
use crate::running_status_widget;
use crate::spacecraft::sysparam_api::Spacecraft;
use crate::toggle_on_off_button;
use crate::{CursorWorldPosition, SelectedSpacecraft};

#[derive(Debug, Clone, Copy)]
struct HoseNode {
    pos: Vec2,
    vel: Vec2,
}

#[derive(Debug, Clone, Copy)]
pub struct InventoryJoint {
    part: Entity,
    position: PartCoord,
    container: Entity,
}

/// Describes a link between two inventory containers
#[derive(Debug, Component, Clone, Copy)]
pub struct InventoryLink {
    src: InventoryJoint,
    dst: InventoryJoint,
    is_on: bool,
    status: MachineStatus,
}

#[derive(Component, Debug)]
pub struct HosePhysics {
    src_pos: Vec2,
    dst_pos: Vec2,
    desired_length: f32,
    nodes: Vec<HoseNode>,
    opacity: f32,
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct SelectedHose {
    hovered: Option<Entity>,
    selected: Option<Entity>,
}

impl HosePhysics {
    fn update_src_pos(&mut self, p: Vec2) {
        self.src_pos = p;
    }

    fn rest_segment_length(&self) -> f32 {
        self.desired_length / (self.nodes.len() - 1) as f32
    }

    fn update_dst_pos(&mut self, p: Vec2) {
        self.dst_pos = p;
    }

    fn length(&self) -> f32 {
        let mut sum = 0.0;
        for n in self.nodes.windows(2) {
            let d = n[0].pos.distance(n[1].pos);
            sum += d;
        }
        sum
    }

    fn step_node_velocity(&mut self, dt: f32) {
        for i in [0, self.nodes.len() - 1] {
            let rest_pos = if i == 0 { self.src_pos } else { self.dst_pos };
            let node = &mut self.nodes[i];
            let accel = (rest_pos - node.pos) * 0.2;
            node.vel += accel * dt;
        }

        let seglen = self.rest_segment_length();

        for i in 1..self.nodes.len() - 1 {
            let before = self.nodes[i - 1];
            let after = self.nodes[i + 1];

            let node = &mut self.nodes[i];

            let u = before.pos - node.pos;
            let v = after.pos - node.pos;

            let avg_vel = (before.vel + after.vel) / 2.0;

            let du = u.length();
            let dv = v.length();

            let u = u.normalize_or_zero();
            let v = v.normalize_or_zero();

            let kp = 12.0;
            let kd = 20.0;

            let au = (du - seglen) * kp * u;
            let av = (dv - seglen) * kp * v;

            let delta_vel = avg_vel - node.vel;

            let accel = au + av + kd * delta_vel;

            node.vel += accel * dt;
        }

        for node in &mut self.nodes {
            node.pos += node.vel * dt;
        }

        let distance = self.src_pos.distance(self.dst_pos);
        let _dist_per_segment = distance / (self.nodes.len() - 1) as f32;

        // if self.is_connected() {
        //     if dist_per_segment > self.rest_segment_length() * 2.0 {
        //         info!("Disconnected");
        //         self.dst = None;
        //     }
        // }

        self.nodes.first_mut().expect("Expected nonempty list").pos = self.src_pos;

        self.nodes.last_mut().expect("Expected nonempty list").pos = self.dst_pos;
    }

    pub fn center(&self) -> Vec2 {
        let center_idx = self.nodes.len() / 2;
        self.nodes[center_idx].pos
    }
}

pub fn on_add_hose(
    event: On<AddHose>,
    transforms: TransformHelper,
    inventory: InventoryApi,
    mut commands: Commands,
) -> Option<()> {
    info!("Add hose: {:?}", event);

    let tf_a = transforms.compute_global_transform(event.start.0).ok()?;
    let tf_b = transforms.compute_global_transform(event.end.0).ok()?;

    let pa = tf_a.translation().xy();
    let pb = tf_b.translation().xy();

    let mut nodes = Vec::new();

    let desired_length = pa.distance(pb) * 0.9;

    let n_segments = ((desired_length / 0.2).round() as u32).max(6);

    for i in 0..n_segments + 1 {
        let s = i as f32 / n_segments as f32;
        let p = pa.lerp(pb, s);
        let node = HoseNode {
            pos: p,
            vel: randvec(0.01, 0.05),
        };
        nodes.push(node);
    }

    let src_container = inventory
        .find_container_at(event.start.0, event.start.1)
        .ok();
    let dst_container = inventory.find_container_at(event.end.0, event.end.1).ok();

    let hose = HosePhysics {
        src_pos: pa,
        dst_pos: pb,
        desired_length,
        nodes,
        opacity: 1.0,
    };

    let id = commands.spawn(hose).id();

    info!("Spawned hose: {}", id);

    if let Some((src, dst)) = src_container.zip(dst_container) {
        let src = InventoryJoint {
            part: event.start.0,
            position: event.start.1,
            container: src,
        };
        let dst = InventoryJoint {
            part: event.end.0,
            position: event.end.1,
            container: dst,
        };
        let link = InventoryLink {
            src,
            dst,
            is_on: true,
            status: MachineStatus::Off,
        };
        info!("Established link: {:?}", link);
        commands.entity(id).insert(link);
    }

    Some(())
}

pub fn on_add_pipe(
    event: On<AddPipe>,
    inventory: InventoryApi,
    spacecraft: Spacecraft,
    mut commands: Commands,
    mut alerts: MessageWriter<SpawnAnimText>,
) -> Result<(), FailedLookup> {
    info!("Add pipe: {:?}", event);

    let grid_a = spacecraft.get_vehicle_from_part_id(event.start.0)?;
    let grid_b = spacecraft.get_vehicle_from_part_id(event.end.0)?;

    if grid_a != grid_b {
        alerts.write(SpawnAnimText::new("Can't add pipe across grids"));
        return Ok(());
    }

    let src_slot = inventory.find_container_at(event.start.0, event.start.1)?;
    let dst_slot = inventory.find_container_at(event.end.0, event.end.1)?;

    let src = InventoryJoint {
        part: event.start.0,
        position: event.start.1,
        container: src_slot,
    };
    let dst = InventoryJoint {
        part: event.end.0,
        position: event.end.1,
        container: dst_slot,
    };
    let link = InventoryLink {
        src,
        dst,
        is_on: true,
        status: MachineStatus::Off,
    };

    info!("Established link: {:?}", link);
    commands.spawn(link);

    Ok(())
}

pub fn update_hose_physics_system(
    mut commands: Commands,
    mut hoses: Query<(Entity, &mut HosePhysics, &mut InventoryLink)>,
    parts: Query<&PartInstance>,
    transforms: TransformHelper,
) {
    let dt = 1.0 / 60.0;
    for (e, mut hose, link) in &mut hoses {
        if let Ok(tf) = transforms.compute_global_transform(link.src.part) {
            if let Ok(part) = parts.get(link.src.part) {
                let part_center = tf.compute_transform();
                let tf =
                    compute_part_cell_transform(part_center, part.placement, link.src.position);
                hose.update_src_pos(tf.translation.xy());
            }
        }

        if let Ok(tf) = transforms.compute_global_transform(link.dst.part) {
            if let Ok(part) = parts.get(link.dst.part) {
                let part_center = tf.compute_transform();
                let tf =
                    compute_part_cell_transform(part_center, part.placement, link.dst.position);
                hose.update_dst_pos(tf.translation.xy());
            }
        }

        hose.step_node_velocity(dt);

        if hose.opacity < 0.0 {
            commands.entity(e).despawn();
        }
    }
}

pub fn process_inventory_links_system(
    links: Query<&mut InventoryLink>,
    mut slots: Query<&mut InvSlot>,
) {
    for mut link in links {
        if !link.is_on {
            link.status = MachineStatus::Off;
            continue;
        }

        link.status = MachineStatus::Disconnected;

        let [mut src, mut dst] =
            ok_or_continue!(slots.get_many_mut([link.src.container, link.dst.container]));

        link.status = atomic_transfer(&mut src, &mut dst, Mass::grams(300));
    }
}

pub fn draw_hoses_system(
    mut painter: ShapePainter,
    hoses: Query<(Entity, &HosePhysics)>,
    selected: Res<SelectedHose>,
) {
    painter.reset();
    painter.thickness_type = ThicknessType::World;

    let z = ZOrdering::Debug.as_f32();

    for (e, hose) in hoses {
        for n in hose.nodes.windows(2) {
            let p = n[0].pos;
            let q = n[1].pos;
            painter.thickness = 0.09;
            painter.set_color(GRAY_800.with_alpha(hose.opacity));
            painter.set_translation(Vec3::Z * z);
            painter.line(p.extend(0.0), q.extend(0.0));
            painter.thickness = 0.06;

            let color = if selected.selected == Some(e) {
                ORANGE_400
            } else {
                GRAY_900
            };

            painter.set_color(color.with_alpha(hose.opacity));
            painter.set_translation(Vec3::Z * (z + 0.03));
            painter.line(p.extend(0.0), q.extend(0.0));
        }
    }
}

pub fn update_selected_hose_system(
    mut info: ResMut<SelectedHose>,
    mouse: Res<CursorWorldPosition>,
    hoses: Query<(Entity, &HosePhysics)>,
    buttons: Res<ButtonInput<MouseButton>>,
) {
    info.hovered = None;

    let Some(p) = mouse.get() else {
        return;
    };

    let mut best_hovered = None;
    let mut min_distance = None;

    for (e, hose) in &hoses {
        let center = hose.center();
        let d = center.distance(p);
        if d < HOSE_SELECTION_AREA_WORLD_RADIUS {
            if let Some(md) = min_distance {
                if d < md {
                    min_distance = Some(d);
                    best_hovered = Some(e);
                }
            } else {
                min_distance = Some(d);
                best_hovered = Some(e);
            }
        }
    }

    info.hovered = best_hovered;

    if buttons.just_pressed(MouseButton::Left) {
        info.selected = info.hovered;
    }
}

const HOSE_SELECTION_AREA_WORLD_RADIUS: f32 = 0.5;

pub fn draw_hose_selection_area_system(
    mut painter: ShapePainter,
    hoses: Query<(Entity, &HosePhysics)>,
    selected: Res<SelectedHose>,
) {
    let z = ZOrdering::Debug.as_f32();

    for (e, hose) in hoses {
        let (color, hollow) = if selected.selected == Some(e) {
            (ORANGE.with_alpha(0.6), false)
        } else if selected.hovered == Some(e) {
            (TEAL.with_alpha(0.3), false)
        } else {
            (GRAY, true)
        };

        painter.reset();
        painter.hollow = hollow;
        painter.thickness = 0.05;
        painter.set_translation(hose.center().extend(z));
        painter.set_color(color);
        painter.circle(HOSE_SELECTION_AREA_WORLD_RADIUS);
    }
}

pub fn item_dropdown(ui: &mut egui::Ui, selected: &mut Item, title: &str, filter: &ItemFilter) {
    egui::ComboBox::from_label(title)
        .selected_text(format!("{:?}", selected))
        .show_ui(ui, |ui| {
            for item in Item::all_that_passes(filter) {
                let text = format!("{:?}", item);
                ui.selectable_value(selected, item, text);
            }
        });
}

/// Computes the transform (isometry - 2D position and yaw) of
/// the center of the given cell defined by the provided PartCoord.
fn compute_part_cell_transform(
    part_center: Transform,
    placement: GridPlacement,
    coord: PartCoord,
) -> Transform {
    let half_dims = placement.part_aligned_dims().to_meters() / 2.0;
    let offset = coord.to_meters() - half_dims + PartCoord::ONE.to_meters() / 2.0;
    let new_translation =
        part_center.translation + part_center.right() * offset.x + part_center.up() * offset.y;
    part_center.with_translation(new_translation)
}

pub fn debug_draw_inventory_links(
    mut gizmos: Gizmos,
    links: Query<&InventoryLink>,
    spacecraft: Spacecraft,
    settings: Res<Settings>,
) {
    if !settings.draw_inventory_cons {
        return;
    }

    for link in links {
        let cell_a = ok_or_continue!(spacecraft.cell_global_transform(
            link.src.part,
            link.src.position,
            CellPosition::Center
        ));
        let cell_b = ok_or_continue!(spacecraft.cell_global_transform(
            link.dst.part,
            link.dst.position,
            CellPosition::Center
        ));

        let color = match link.status {
            MachineStatus::Disconnected => PURPLE,
            MachineStatus::Off => GRAY,
            MachineStatus::NoRecipe => GRAY,
            MachineStatus::Running => ORANGE,
            MachineStatus::NoRoom => RED,
            MachineStatus::Starved => YELLOW,
            MachineStatus::BadFilter => PINK,
        };

        gizmos.line_2d(cell_a.translation.xy(), cell_b.translation.xy(), color);

        gizmos.rect_2d(
            transform_to_isometry(cell_a),
            Vec2::splat(PartCoord::CELL_WIDTH * 0.6),
            RED,
        );
        gizmos.rect_2d(
            transform_to_isometry(cell_b),
            Vec2::splat(PartCoord::CELL_WIDTH * 0.6),
            GREEN,
        );
    }
}

pub fn hose_info_window_egui_system(
    mut contexts: EguiContexts,
    selected: ResMut<SelectedHose>,
    mut links: Query<(Entity, &mut InventoryLink, &HosePhysics)>,
    inventory: InventoryApi,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    let Some(id) = selected.selected else {
        return Ok(());
    };

    let (_, mut link, hose) = links.get_mut(id)?;

    let pos = camera
        .0
        .world_to_viewport(camera.1, hose.center().extend(0.0))
        .unwrap();

    egui::Window::new("Hose Info")
        .fixed_pos(Pos2::new(pos.x, pos.y))
        .show(ctx, |ui| {
            apply_egui_style(ui);
            ui.heading(format!("Hose {}", id));

            ui.separator();

            toggle_on_off_button(ui, &mut link.is_on);
            running_status_widget(ui, link.status);

            ui.separator();

            ui.heading("Hose Physics");
            ui.label(format!("Desired Length: {:0.1}", hose.desired_length));
            ui.label(format!("Actual Length: {:0.1}", hose.length()));
            ui.label(format!("Nodes: {}", hose.nodes.len()));

            ui.separator();

            ui.label(format!("From: {:?}", link.src));
            ui.label(format!("To: {:?}", link.dst));

            ui.heading("Containers");
            if let Ok(container) = inventory.get_container(link.src.container) {
                ui.separator();
                add_slot_widget(ui, container.1);
            }
            if let Ok(container) = inventory.get_container(link.dst.container) {
                ui.separator();
                add_slot_widget(ui, container.1);
            }

            if ui.button("Delete").clicked() {
                commands.entity(id).despawn();
            }
        });

    Ok(())
}
