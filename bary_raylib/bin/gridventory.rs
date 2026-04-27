use bary_core::prelude::*;
use bary_raylib::multiplayer::MessageQueue;
use bary_raylib::utils::BasicApp;
use bary_raylib::{
    constants::TICKS_PER_SECOND,
    multiplayer::new_message_queue,
    sim::{apparent_elapsed_time, consume_rdev_event_into_input_state},
    utils::InputState,
};
use crossbeam_queue::SegQueue;
use rand::{RngExt, SeedableRng, rngs::StdRng};
use raylib::prelude::*;
use rayon::prelude::*;
use std::{
    collections::BTreeSet,
    sync::Arc,
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub struct GridVentory {
    ticks: u64,
    slots: Vec<InvSlot>,
    pipes: Vec<(usize, usize, MachineStatus)>,
    sources: Vec<(usize, Item)>,
    sinks: Vec<usize>,
    dirty_set: BTreeSet<usize>,
    is_settled: bool,
}

impl GridVentory {
    pub fn random(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);

        let n_slots = rng.random_range(200..300);
        let n_pipes = rng.random_range(200..500);

        let slots: Vec<InvSlot> = (0..n_slots)
            .map(|_| {
                let capacity: Volume = Volume::liters(rng.random_range(10..1000));
                let filter = ItemFilter::Any;
                let is_fluid = false;
                let location = (PartCoord::ZERO, PartCoord::ZERO);
                InvSlot::new(capacity, filter, is_fluid, location)
            })
            .collect();

        let pipes = (0..n_pipes)
            .map(|_| {
                let a = rng.random_range(0..slots.len() - 1);
                let b = rng.random_range(0..slots.len() - 1);
                (a, b, MachineStatus::Running)
            })
            .filter(|(a, b, _)| a != b)
            .collect();

        let sources = (0..5)
            .map(|_| {
                let item = Item::random();
                let slot = rng.random_range(0..slots.len() - 1);
                (slot, item)
            })
            .collect();

        let sinks = vec![6];

        Self {
            ticks: 0,
            slots,
            pipes,
            sources,
            sinks,
            dirty_set: BTreeSet::new(),
            is_settled: false,
        }
    }

    pub fn mass(&self) -> Mass {
        self.slots.iter().map(|s| s.mass()).sum()
    }
}

fn update_inventory(grid: &mut GridVentory) {
    grid.dirty_set.clear();

    if grid.ticks.is_multiple_of(5000) {
        grid.is_settled = false;
        for (index, item) in &grid.sources {
            let slot = &mut grid.slots[*index];
            slot.fill_with(*item);
        }
    }

    grid.ticks += 1;

    if grid.is_settled {
        return;
    }

    grid.is_settled = true;

    for index in &grid.sinks {
        let slot = &mut grid.slots[*index];
        slot.empty();
    }

    for (a, b, status) in &mut grid.pipes {
        if a == b {
            continue;
        }

        let [src, dst] = grid.slots.get_disjoint_mut([*a, *b]).unwrap();

        if src.is_empty() || dst.is_full() {
            *status = MachineStatus::Off;
            continue;
        }

        if src.item().is_some() && dst.item().is_some() && src.item() != dst.item() {
            *status = MachineStatus::Off;
            continue;
        }

        grid.is_settled = false;

        let mass = {
            let mul = 150;
            let m = src.mass() / mul as u64;
            if m.is_zero() { Mass::grams(1) } else { m }
        };

        grid.dirty_set.insert(*a);
        grid.dirty_set.insert(*b);

        *status = atomic_transfer(src, dst, mass);
    }
}

fn draw_gridventory(d: &mut RaylibDrawHandle, grid: &GridVentory, elapsed: Duration) {
    d.draw_text(
        &format!(
            "{} {} {} {:?}\n{:0.9} ms",
            d.get_fps(),
            grid.ticks,
            grid.is_settled,
            grid.mass(),
            elapsed.as_nanos() as f64 / 1.0E6,
        ),
        50,
        50,
        24,
        Color::WHITE,
    );

    let mut rng = StdRng::seed_from_u64(10000);

    let n_cols = (grid.slots.len() as f32).sqrt().ceil() as i32 * 3 / 2;

    let screen_width = d.get_screen_width();

    let screen_padding = 120;
    let padding = 3;

    let render_width = screen_width - screen_padding * 2;

    let width = (render_width + padding) / n_cols;

    let x0 = screen_padding;
    let y0 = screen_padding;

    let slot_coord = |i: usize| {
        let col_idx = i as i32 % n_cols;
        let row_idx = i as i32 / n_cols;

        let x = x0 + col_idx * (width + padding);
        let y = y0 + row_idx * (width + padding);

        (x, y)
    };

    let slot_coord_center = |i: usize| {
        let (x, y) = slot_coord(i);
        (x + width / 2, y + width / 2)
    };

    for (i, slot) in grid.slots.iter().enumerate() {
        let (x, y) = slot_coord(i);
        let [r, g, b] = slot.item().map(|i| i.color()).unwrap_or([30, 30, 30]);
        let color = Color::new(r, g, b, 255);
        let pct = slot.fill_percentage();
        let height = (width as f32 * pct) as i32;
        d.draw_rectangle(x, y, width, width, Color::new(20, 20, 20, 255));
        if !slot.is_empty() {
            d.draw_rectangle(x, y, width, height, color);
        }
    }

    for (src, dst, status) in &grid.pipes {
        let a = rng.random_range(-width / 3..=width / 3);
        let b = rng.random_range(-width / 3..=width / 3);
        let j = rng.random_range(-width / 3..=width / 3);
        let k = rng.random_range(-width / 3..=width / 3);

        let (x0, y0) = slot_coord_center(*src);
        let (xf, yf) = slot_coord_center(*dst);

        let x0 = x0 + a;
        let y0 = y0 + b;
        let xf = xf + j;
        let yf = yf + k;

        let color = match status {
            MachineStatus::Off => Color::GRAY.alpha(0.3),
            _ => Color::ORANGE,
        };

        d.draw_circle(x0, y0, 5.0, color);
        d.draw_circle(xf, yf, 3.0, color);

        let x_first = rng.random_bool(0.5);

        let bend = get_bend_location((x0, y0), (xf, yf), x_first);

        let p = Vector2::new(x0 as f32, y0 as f32);
        let q = Vector2::new(xf as f32, yf as f32);
        if let Some(o) = bend.map(|p| p.inner()) {
            let (xb, yb) = (o.x, o.y);
            let o = Vector2::new(xb as f32, yb as f32);
            d.draw_line_ex(p, o, 3.0, color);
            d.draw_line_ex(o, q, 3.0, color);
        } else {
            d.draw_line_ex(p, q, 3.0, color);
        }
    }

    for index in &grid.dirty_set {
        let (x, y) = slot_coord(*index);
        let rec = Rectangle::new(x as f32, y as f32, width as f32, width as f32);
        d.draw_rectangle_lines_ex(rec, 5.0, Color::GREEN);
    }

    for (index, _item) in &grid.sources {
        let (x, y) = slot_coord(*index);
        let rec = Rectangle::new(x as f32, y as f32, width as f32, width as f32);
        d.draw_rectangle_lines_ex(rec, 5.0, Color::GREEN);
    }

    for index in &grid.sinks {
        let (x, y) = slot_coord(*index);
        let rec = Rectangle::new(x as f32, y as f32, width as f32, width as f32);
        d.draw_rectangle_lines_ex(rec, 5.0, Color::RED);
    }
}

fn simulate(mut grid: GridVentory) {
    let total_ticks = TICKS_PER_SECOND * 3600;

    println!("{:?}", grid.mass());
    println!("Simulating...");

    let start = Instant::now();

    for _ in 0..total_ticks {
        update_inventory(&mut grid);

        if grid.is_settled {
            println!("Settled at tick {}", grid.ticks);
            break;
        }
    }

    let elapsed = Instant::now() - start;

    let sim_elapsed = apparent_elapsed_time(grid.ticks);

    let average = elapsed / grid.ticks as u32;

    println!("{:?}", grid.mass());
    println!(
        "Done: took {:?}, sim: {:?}, speed: {:0.1}x, {:?} per tick",
        elapsed,
        sim_elapsed,
        sim_elapsed.as_secs_f64() / elapsed.as_secs_f64(),
        average
    );
}

fn raylib_window(mut grid: GridVentory) {
    let mut app = BasicApp::new("Gridventory Demo");

    while !app.handle.window_should_close() {
        app.update_inputs();

        let start = Instant::now();

        for _ in 0..TICKS_PER_SECOND {
            update_inventory(&mut grid);
        }

        let elapsed = Instant::now() - start;

        if app.input.just_pressed_debounced(rdev::Key::KeyR) {
            grid = GridVentory::random(randint(100, 10000) as u64);
        }

        if app.input.just_pressed_debounced(rdev::Key::Escape) {
            panic!();
        }

        app.handle.draw(&app.thread, |mut d| {
            d.clear_background(Color::BLACK);

            draw_gridventory(&mut d, &grid, elapsed);
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
    let seed = 12;
    let grid = GridVentory::random(seed);
    raylib_window(grid);
}
