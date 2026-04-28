use bary_core::prelude::*;
use bary_raylib::render::draw::draw_gridventory;
use bary_raylib::sim::apparent_datetime;
use bary_raylib::sim::*;
use bary_raylib::utils::BasicApp;
use bary_raylib::{constants::TICKS_PER_SECOND, sim::apparent_elapsed_time};
use rand::{RngExt, SeedableRng, rngs::StdRng};
use raylib::prelude::*;
use rayon::prelude::*;
use std::time::Instant;

fn draw_pie_chart(d: &mut RaylibDrawHandle, x: i32, y: i32, radius: f32, values: &[f32]) {
    let mut rng = StdRng::seed_from_u64(6);

    let sum: f32 = values.iter().sum();
    let mut a = 0.0;
    for value in values {
        let portion = *value / sum;
        let da = portion * 360.0;
        let segments = (da / 5.0).round() as i32;
        let r = rng.random_range(100..=255);
        let g = rng.random_range(100..=255);
        let b = rng.random_range(100..=255);
        let color = Color::new(r, g, b, 255);
        let pos = Vector2::new(x as f32, y as f32);
        d.draw_circle_sector(pos, radius, a, a + da, segments, color);
        a += da;
    }
}

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

fn raylib_window(mut grids: Vec<GridVentory>) {
    if grids.is_empty() {
        panic!();
    }

    let mut app = BasicApp::new("Gridventory Demo");

    let mut ticks = 0;

    let mut rate = 8;
    let mut index = 0;
    let mut parallel = true;

    let mut times = Vec::new();

    while !app.handle.window_should_close() {
        app.frame();

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

            draw_gridventory(&mut d, &grids[index]);

            let s = format!(
                "{:?} {}/{} {:?}\n{} {}\n{:00000000000000.0}",
                apparent_datetime(ticks),
                index + 1,
                grids.len(),
                app.this_frame - app.last_frame,
                rate,
                parallel,
                times[index].as_nanos()
            );

            let times: Vec<_> = times.iter().map(|d| d.as_secs_f32()).collect();

            let y = d.get_render_height() - 60;
            draw_pie_chart(&mut d, 60, y, 50.0, &times);

            d.draw_text(&s, 400, 20, 28, Color::GRAY);
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
    let grids = vec![
        GridVentory::random(12),
        GridVentory::random(45),
        GridVentory::random(8),
        GridVentory::random(7),
        GridVentory::random(90),
        GridVentory::random(8),
        GridVentory::random(7),
        GridVentory::random(90),
        GridVentory::random(8),
        GridVentory::random(7),
        GridVentory::random(90),
        GridVentory::random(8),
    ];
    raylib_window(grids);
}
