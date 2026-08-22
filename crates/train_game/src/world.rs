use std::{collections::BTreeSet, time::Instant};

use bary_core::prelude::*;
use bary_input::InputState;
use bary_sim::Camera;
use rdev::Key;

use crate::{
    bezier::BezierCurve,
    track::{
        Node, TrackSegment, despawn_node, pathfind, spawn_new_node, spawn_new_track,
        spawn_three_way_junction,
    },
    viewport::Viewport,
};

pub struct World {
    pub ticks: u64,
    pub current_font_id: usize,
    pub time: f64,
    pub camera: Camera,
    pub target_camera: Camera,
    pub hovered_node: Option<Ent>,
    pub pressed_node: Option<(Ent, Instant)>,
    pub selected_nodes: Vec<Ent>,

    pub spawner: EntitySpawner,
    pub nodes: Components<Node>,
    pub segments: Components<TrackSegment>,
}

pub fn update_world(world: &mut World, dt: f64, mouse: DVec2, screen_width: DVec2) {
    world.time += dt;
    world.ticks += 1;

    world.camera.isometry.translation +=
        (world.target_camera.isometry.translation - world.camera.isometry.translation) * 0.2;
    world.camera.isometry.rotation +=
        (world.target_camera.isometry.rotation - world.camera.isometry.rotation) * 0.2;
    world.camera.zoom += (world.target_camera.zoom - world.camera.zoom) * 0.08;

    let view = Viewport::new(world.camera, screen_width);

    let mut best_dist = 60.0;
    world.hovered_node = None;
    for (id, node) in world.nodes.iter() {
        let node_screen = view.world_to_screen(node.pos);
        let d = node_screen.distance(mouse);
        if d < best_dist {
            world.hovered_node = Some(*id);
            best_dist = d;
        }
    }
}

pub fn make_world() -> World {
    let mut world = World {
        ticks: 0,
        current_font_id: 0,
        time: 0.0,
        camera: Camera {
            isometry: Isometry2d::ZERO,
            zoom: 15.0,
        },
        target_camera: Camera {
            isometry: Isometry2d::ZERO,
            zoom: 30.0,
        },
        selected_nodes: Vec::new(),
        hovered_node: None,
        pressed_node: None,
        spawner: EntitySpawner::default(),
        nodes: Components::default(),
        segments: Components::default(),
    };

    let points = vec![
        DVec2::new(10.0, 34.0),
        DVec2::new(30.0, 24.0),
        DVec2::new(45.0, 80.0),
    ];

    let mut ids = Vec::new();

    for pos in points {
        let id = spawn_new_node(&mut world, pos);
        ids.push(id);
    }

    spawn_new_track(&mut world, ids);

    world
}

pub fn process_input(
    world: &mut World,
    input: &InputState,
    dt: f64,
    mouse: DVec2,
    screen_width: DVec2,
) {
    if input.is_key_pressed(Key::Minus) {
        world.target_camera.zoom /= 1.03;
    }
    if input.is_key_pressed(Key::Equal) {
        world.target_camera.zoom *= 1.03;
    }

    for event in input.events() {
        if let rdev::EventType::Wheel {
            delta_x: _,
            delta_y,
        } = event.event_type
        {
            if delta_y > 0 {
                world.target_camera.zoom *= 1.3;
            } else if delta_y < 0 {
                world.target_camera.zoom /= 1.3;
            }
        }
    }

    world.target_camera.zoom = world.target_camera.zoom.clamp(1.0, 200.0);

    let n = input.is_key_pressed(Key::KeyW);
    let w = input.is_key_pressed(Key::KeyA);
    let s = input.is_key_pressed(Key::KeyS);
    let e = input.is_key_pressed(Key::KeyD);

    let x_pull = -(w as i8) + e as i8;
    let y_pull = -(s as i8) + n as i8;

    let speed = 1100.0;

    let dir = DVec2::new(x_pull as f64, y_pull as f64).normalize_or_zero();

    let vel = (dir.x * world.target_camera.isometry.local_x().as_dvec2()
        + dir.y * world.target_camera.isometry.local_y().as_dvec2())
        * speed;

    let r = input.is_key_pressed(Key::KeyE);
    let l = input.is_key_pressed(Key::KeyQ);

    let angular_vel = (l as u8 as f64 - r as u8 as f64) * 2.0;

    world.target_camera.isometry.translation += (vel * dt).as_vec2() / world.target_camera.zoom;
    world.target_camera.isometry.rotation += (angular_vel * dt) as f32;

    let view = Viewport::new(world.camera, screen_width);
    let mouse_world = view.screen_to_world(mouse);

    let shift = input.is_key_pressed(Key::ShiftLeft);

    if input.just_pressed(Key::KeyC) {
        spawn_new_node(world, mouse_world);
    }

    if input.just_pressed(Key::KeyV) {
        let nodes: Vec<Ent> = world.selected_nodes.clone().into_iter().collect();
        spawn_new_track(world, nodes);
    }

    if world.hovered_node.is_none() || !input.is_key_pressed(rdev::Button::Left) {
        world.pressed_node = None;
    }

    if input.just_pressed_debounced(rdev::Button::Left) {
        if let Some(id) = world.hovered_node {
            world.pressed_node = Some((id, Instant::now()));

            let contains = world.selected_nodes.contains(&id);
            match (shift, contains) {
                (true, true) => {
                    world.selected_nodes.retain(|d| *d != id);
                }
                (true, false) => {
                    world.selected_nodes.push(id);
                }
                (false, false) => {
                    world.selected_nodes = vec![id];
                }
                (false, true) => {
                    world.selected_nodes.clear();
                }
            }
        } else {
            world.selected_nodes.clear();
        }
    }

    if input.just_pressed_debounced(rdev::Button::Right) {
        if let Some(id) = world.hovered_node {
            _ = despawn_node(world, id);
        }
    }

    if let Some(ids) = world.selected_nodes.get(0..2)
        && input.just_pressed_debounced(Key::KeyN)
    {
        pathfind(world, ids[0], ids[1]);
    }

    if let Some(ids) = world.selected_nodes.get(0..3)
        && input.just_pressed_debounced(Key::KeyJ)
    {
        spawn_three_way_junction(world, ids[0], ids[1], ids[2]);
    }
}
