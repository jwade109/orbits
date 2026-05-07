use bary_core::prelude::*;
use bary_raylib::render::draw::{draw_gridventory, draw_pie_chart};
use bary_raylib::sim::*;
use bary_raylib::sim::{NewPipeGeometry, apparent_datetime};
use bary_raylib::utils::BasicApp;
use bary_raylib::world_builder::WorldBuilder;
use bary_raylib::{constants::TICKS_PER_SECOND, sim::apparent_elapsed_time};
use raylib::prelude::*;
use rayon::prelude::*;
use std::time::Instant;

fn simulate(mut grid: GridVentory) {
    let total_ticks = TICKS_PER_SECOND * 3600;

    println!("{:?}", grid.mass());
    println!("Simulating...");

    let start = Instant::now();

    let mut ticks = 0;

    for t in 0..total_ticks {
        update_inventory(&mut grid);

        ticks = t;

        if grid.is_settled {
            println!("Settled at tick {}", t);
            break;
        }
    }

    let elapsed = Instant::now() - start;

    let sim_elapsed = apparent_elapsed_time(ticks);

    let average = elapsed / ticks as u32;

    println!("{:?}", grid.mass());
    println!(
        "Done: took {:?}, sim: {:?}, speed: {:0.1}x, {:?} per tick",
        elapsed,
        sim_elapsed,
        sim_elapsed.as_secs_f64() / elapsed.as_secs_f64(),
        average
    );
}

fn example_gridventory() -> GridVentory {
    let mut grid = GridVentory::default();

    let a = grid.add_slot(Volume::liters(1000), (0, 0), (3, 2)).unwrap();

    let mut prev = IVec2::new(1, 1);

    for i in 1..=8 {
        let origin = IVec2::new(i, 3);
        let upper = origin + IVec2::new(1, 2);
        grid.add_slot(Volume::liters(100), origin, upper);
        grid.add_pipe_at(prev, origin, NewPipeGeometry::Straight);
        prev = origin + IVec2::Y;
    }

    grid.add_slot(Volume::liters(600), (4, 0), (5, 2));
    grid.add_pipe_at(prev, (4, 1), NewPipeGeometry::Straight);
    let c = grid.add_slot(Volume::liters(600), (5, 0), (6, 2)).unwrap();
    grid.add_pipe_at(prev, (5, 1), NewPipeGeometry::Straight);

    grid.add_source(a);
    grid.add_sink(c);
    grid
}

fn raylib_window() {
    let world = WorldBuilder::new()
        .assets()
        .blueprint(("pollux", 0))
        .blueprint(("pollux", 2))
        .blueprint(("bellerophon", 2))
        .build();

    let bp = blueprint_by_id(&world.blueprints, &("pollux", 2).into()).unwrap();

    dbg!(bp);

    let mut grids = Vec::new();

    for bp in world.blueprints.values() {
        let grid = GridVentory::from_blueprint(&bp.blueprint, &world.prototypes);
        grids.push(grid);
    }

    let mut app = BasicApp::new("Gridventory Demo");

    let mut ticks = 0;

    let mut rate = 1;
    let mut index = 0;
    let mut parallel = true;

    let mut times = Vec::new();

    while app.frame() {
        let start = Instant::now();

        app.fixed_50_fps(|| {
            let op = |g: &mut GridVentory| {
                let start = Instant::now();
                for _ in 0..rate {
                    update_inventory(g);
                }
                Instant::now() - start
            };

            ticks += rate;

            times = if parallel {
                grids.par_iter_mut().map(op).collect()
            } else {
                grids.iter_mut().map(op).collect()
            };
        });

        let elapsed = Instant::now() - start;

        if app.input.just_pressed_debounced(rdev::Key::KeyR) {
            for g in &mut grids {
                *g = GridVentory::random(randint(100, 10000) as u64);
            }
            grids[0] = example_gridventory();
        }

        if app.input.just_pressed_debounced(rdev::Key::UpArrow) {
            rate *= 2;
        }

        if app.input.just_pressed_debounced(rdev::Key::DownArrow) && rate > 1 {
            rate /= 2;
        }

        if app.input.just_pressed_debounced(rdev::Key::LeftArrow) && index > 0 {
            index -= 1;
        }

        if app.input.just_pressed_debounced(rdev::Key::KeyP) {
            parallel ^= true;
        }

        if app.input.just_pressed_debounced(rdev::Key::RightArrow) && index < grids.len() {
            index += 1;
        }

        if app.input.just_pressed_debounced(rdev::Key::Escape) {
            panic!();
        }

        if grids.len() <= index {
            index = grids.len() - 1;
        }

        app.handle.draw(&app.thread, |mut d| {
            d.clear_background(Color::BLACK);

            let labels = app.input.is_key_pressed(rdev::Key::ShiftLeft);

            draw_gridventory(&mut d, &grids[index], labels);

            let s = [
                format!("sim time: {:?}", apparent_datetime(ticks)),
                format!("{}/{}", index + 1, grids.len()),
                format!("settled: {}", grids[index].is_settled),
                format!("mass: {}", grids[index].mass()),
                format!("frame_delta: {:?}", app.this_frame - app.last_frame),
                format!("rate: {}", rate),
                format!("multithreaded: {}", parallel),
                format!("ms this grid: {:00000000.0}", times[index].as_millis()),
                format!("ms total: {:00000000.0}", elapsed.as_millis()),
            ];

            let s = s.join("\n");

            let times: Vec<_> = times.iter().map(|d| d.as_secs_f32()).collect();

            let y = d.get_render_height() - 60;
            draw_pie_chart(&mut d, 60, y, 50.0, &times, None);

            d.draw_text(&s, 40, 20, 22, Color::WHITE.alpha(0.5));
        });
    }
}

fn simulate_parallel() {
    let seeds = [
        414, 414, 414, 414, 414, 414, 414, 414, 414, 414, 414, 414, 414, 414, 414,
    ];

    let t0 = Instant::now();

    // sequentially
    for seed in seeds {
        let grid = GridVentory::random(seed);
        simulate(grid);
    }

    let t1 = Instant::now();

    seeds.par_iter().for_each(|seed| {
        let grid = GridVentory::random(*seed);
        simulate(grid);
    });

    let t2 = Instant::now();

    println!("t1 - t0: {:?}", t1 - t0);
    println!("t2 - t1: {:?}", t2 - t1);
}

fn main() {
    raylib_window();
}
