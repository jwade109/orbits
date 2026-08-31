use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::Instant;

use bary_core::prelude::*;
use bary_input::InputState;
use bary_sim::Camera;
use rdev::Key;

use crate::node::*;
use crate::persistence::*;
use crate::railcar::spawn_new_car;
use crate::railcar::{RailCar, get_next_track};
use crate::terrain::*;
use crate::track::*;
use crate::viewport::Viewport;

pub const MOUSEOVER_RADIUS: f64 = 60.0;

pub struct World {
    pub ticks: u64,
    pub current_font_id: usize,
    pub time: f64,
    pub show_detail: bool,

    pub camera: Camera,
    pub target_camera: Camera,

    pub hovered_node: Option<Ent>,
    pub pressed_node: Option<(Ent, Instant)>,
    pub selected_nodes: Vec<Ent>,
    pub hovered_track: Option<Ent>,
    pub selected_track: Option<Ent>,
    pub hovered_chunk: Option<ChunkIndex>,
    pub ruler_start: Option<DVec2>,

    pub spawner: EntitySpawner,
    pub nodes: Components<Node>,
    pub segments: Components<TrackSegment>,
    pub cars: Components<RailCar>,

    pub chunks: Components<TerrainChunk>,
    pub chunk_map: BTreeMap<ChunkIndex, Ent>,

    pub calculated_route: Option<Route>,
}

impl World {
    pub fn new() -> Self {
        Self {
            ticks: 0,
            current_font_id: 0,
            time: 0.0,
            show_detail: false,
            camera: Camera {
                isometry: Isometry2d::ZERO,
                zoom: 0.3,
            },
            target_camera: Camera {
                isometry: Isometry2d::ZERO,
                zoom: 0.28,
            },
            selected_nodes: Vec::new(),
            hovered_node: None,
            pressed_node: None,
            hovered_track: None,
            selected_track: None,
            hovered_chunk: None,
            ruler_start: None,
            spawner: EntitySpawner::default(),
            nodes: Components::default(),
            segments: Components::default(),
            cars: Components::default(),
            chunks: Components::default(),
            chunk_map: BTreeMap::new(),
            calculated_route: None,
        }
    }
}

pub fn update_world(world: &mut World, dt: f64, mouse: DVec2, screen_width: DVec2) {
    world.time += dt;
    world.ticks += 1;

    let mut needs_reparenting = Vec::new();

    for (car_id, car) in world.cars.iter_mut() {
        car.step(dt);

        let Some(track) = world.segments.get(car.segment) else {
            continue;
        };

        if car.pos > track.length {
            let junction = track.get_node_at(car.origin.other());
            needs_reparenting.push((*car_id, car.segment, junction));
        } else if car.pos < 0.0 {
            let junction = track.get_node_at(car.origin);
            needs_reparenting.push((*car_id, car.segment, junction));
        }
    }

    for (car_id, track_id, node_id) in needs_reparenting {
        if let Some((id, term)) = get_next_track(world, node_id, track_id) {
            if let Ok(car) = world.cars.try_get_mut(car_id) {
                car.segment = id;
                car.pos = 0.0;
                car.origin = term;
            }
        } else {
            if let Ok(car) = world.cars.try_get_mut(car_id) {
                car.pos = 0.0;
                car.origin = car.origin.other();
            }
        }
    }

    world.camera.isometry.translation +=
        (world.target_camera.isometry.translation - world.camera.isometry.translation) * 0.2;
    world.camera.isometry.rotation +=
        (world.target_camera.isometry.rotation - world.camera.isometry.rotation) * 0.2;
    world.camera.zoom += (world.target_camera.zoom - world.camera.zoom) * 0.08;

    let view = Viewport::new(world.camera, screen_width);

    {
        let mut best_dist = MOUSEOVER_RADIUS;
        if world.pressed_node.is_none() {
            world.hovered_node = None;
            for (id, node) in world.nodes.iter() {
                let node_screen = view.world_to_screen(node.pos());
                let d = node_screen.distance(mouse);
                if d < best_dist {
                    world.hovered_node = Some(*id);
                    best_dist = d;
                }
            }
        }
    }

    {
        let mut best_dist = MOUSEOVER_RADIUS;
        world.hovered_track = None;
        for (id, track) in world.segments.iter() {
            let p = track.center.translation.as_dvec2();
            let sc = view.world_to_screen(p);
            let d = sc.distance(mouse);
            if d < best_dist {
                best_dist = d;
                world.hovered_track = Some(*id);
            }
        }
    }

    world.hovered_chunk = Some(get_chunk_index(view.screen_to_world(mouse)));
}

pub fn make_world() -> World {
    let mut world: World = World::new();

    if load_world(&mut world, "train_world").is_none() {
        println!("Failed to load world");
    }

    let mut new_chunks = BTreeSet::new();

    for chunk in world.chunks.values() {
        let idx = chunk.index();
        for x in -2..=2 {
            for y in -2..=2 {
                let idx = idx.as_ivec2() + IVec2::new(x, y);
                let idx = ChunkIndex::new(idx);
                if !world.chunk_map.contains_key(&idx) {
                    new_chunks.insert(idx);
                }
            }
        }
    }

    for index in new_chunks {
        ensure_chunk_exists(&mut world, index);
    }

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

    if input.just_pressed_debounced(Key::ControlLeft) {
        world.show_detail ^= true;
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

    world.target_camera.zoom = world.target_camera.zoom.clamp(0.01, 200.0);

    let n = input.is_key_pressed(Key::KeyW) && !input.is_key_pressed(Key::ControlLeft);
    let w = input.is_key_pressed(Key::KeyA) && !input.is_key_pressed(Key::ControlLeft);
    let s = input.is_key_pressed(Key::KeyS) && !input.is_key_pressed(Key::ControlLeft);
    let e = input.is_key_pressed(Key::KeyD) && !input.is_key_pressed(Key::ControlLeft);

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

    if input.just_pressed_debounced(Key::KeyC) {
        spawn_new_node(world, mouse_world);
    }

    if input.just_pressed_debounced(Key::KeyV) {
        let nodes: Vec<Ent> = world.selected_nodes.clone().into_iter().collect();
        if spawn_new_track(world, nodes).is_none() {
            println!("Failed to spawn new track");
        }
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
        if let Some(track_id) = world.hovered_track {
            _ = despawn_track(world, track_id);
        } else if let Some(node_id) = world.hovered_node {
            _ = despawn_node(world, node_id);
        }
    }

    if input.just_pressed_debounced(Key::KeyN) {
        if let Some(ids) = world.selected_nodes.get(0..2) {
            world.calculated_route = pathfind(world, ids[0], ids[1]);
        } else {
            world.calculated_route = None;
        }
    }

    if let Some(ids) = world.selected_nodes.get(0..3)
        && input.just_pressed_debounced(Key::Num3)
    {
        spawn_three_way_junction(world, ids[0], ids[1], ids[2]);
    }

    if let Some(ids) = world.selected_nodes.get(0..4)
        && input.just_pressed_debounced(Key::Num4)
    {
        spawn_four_way_junction(world, ids[0], ids[1], ids[2], ids[3]);
    }

    if input.just_pressed_debounced(Key::Num5) {
        spawn_very_large_track(world, &world.selected_nodes.clone());
    }

    if input.just_pressed_debounced(Key::KeyJ) {
        for id in world.selected_nodes.clone() {
            update_switch_node(world, id);
        }
    }

    if input.is_key_pressed(Key::KeyG) {
        if let Some(track_id) = world.hovered_track {
            spawn_new_car(world, track_id);
        }
    }

    let mouse_world = view.screen_to_world(mouse);

    if input.is_key_pressed(Key::KeyB) {
        let index = get_chunk_index(mouse_world);
        let mut new_chunks = BTreeSet::new();
        for chunk in world.chunks.values() {
            let idx = chunk.index();
            for x in -6..=6 {
                for y in -6..=6 {
                    let idx = index.as_ivec2() + IVec2::new(x, y);
                    let idx = ChunkIndex::new(idx);
                    if !world.chunk_map.contains_key(&idx) {
                        new_chunks.insert(idx);
                    }
                }
            }
        }
        for index in new_chunks {
            ensure_chunk_exists(world, index);
        }
    }

    if input.just_pressed_debounced(Key::ShiftLeft) {
        world.ruler_start = Some(mouse_world);
    }

    if !input.is_key_pressed(Key::ShiftLeft) {
        world.ruler_start = None;
    }

    let now = Instant::now();
    if let Some((node_id, time)) = world.pressed_node {
        let delta = now - time;
        if delta.as_secs_f64() > 0.6 {
            move_node(world, node_id, mouse_world);
        }
    }

    if input.is_key_pressed(Key::ControlLeft) && input.just_pressed_debounced(Key::KeyS) {
        if save_world(world, "train_world").is_none() {
            println!("Failed to save!");
        }
    }

    if input.is_key_pressed(Key::ControlLeft) && input.just_pressed_debounced(Key::KeyL) {
        if load_world(world, "train_world").is_none() {
            println!("Failed to load");
        }
    }
}
