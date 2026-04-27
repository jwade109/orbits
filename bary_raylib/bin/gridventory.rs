use bary_core::prelude::*;
use bary_raylib::{constants::TICKS_PER_SECOND, sim::apparent_elapsed_time};
use rand::{RngExt, SeedableRng, rngs::StdRng};
use raylib::prelude::*;
use std::time::Instant;

pub struct GridVentory {
    ticks: u64,
    slots: Vec<InvSlot>,
    pipes: Vec<(usize, usize)>,
    sources: Vec<(usize, Item)>,
    is_settled: bool,
}

impl GridVentory {
    pub fn mass(&self) -> Mass {
        self.slots.iter().map(|s| s.mass()).sum()
    }
}

fn update_inventory(grid: &mut GridVentory) {
    grid.ticks += 1;

    if grid.is_settled {
        return;
    }

    grid.is_settled = true;

    for (index, item) in &grid.sources {
        let slot = &mut grid.slots[*index];
        slot.fill_with(*item);
    }

    for (a, b) in &grid.pipes {
        if a == b {
            continue;
        }

        let [src, dst] = grid.slots.get_disjoint_mut([*a, *b]).unwrap();

        if src.is_empty() || dst.is_full() {
            continue;
        }

        grid.is_settled = false;

        let mass = {
            let mul = randint(140, 160);
            let m = src.mass() / mul as u64;
            if m.is_zero() { Mass::grams(1) } else { m }
        };

        atomic_transfer(src, dst, mass);
    }
}

fn draw_gridventory(d: &mut RaylibDrawHandle, grid: &GridVentory) {
    d.draw_text(
        &format!("{} {} {}", d.get_fps(), grid.ticks, grid.is_settled),
        50,
        50,
        24,
        Color::WHITE,
    );

    let mut x = 200;
    let y = 200;
    let width = 22;
    let padding = 10;

    for slot in &grid.slots {
        let max_height = 600;

        let [r, g, b] = slot.item().map(|i| i.color()).unwrap_or([30, 30, 30]);
        let color = Color::new(r, g, b, 255);
        let pct = if slot.is_empty() {
            0.01
        } else {
            slot.fill_percentage()
        };
        let height = (max_height as f32 * pct) as i32;
        d.draw_rectangle(x, y, width, max_height, Color::new(20, 20, 20, 255));
        d.draw_rectangle(x, y, width, height, color);
        x += width + padding;
    }

    let mut y = 250;
    for pipe in &grid.pipes {
        let x0 = pipe.0 as i32 * (width + padding) + 200 + width / 2;
        let xf = pipe.1 as i32 * (width + padding) + 200 + width / 2;
        d.draw_line(x0, y, xf, y, Color::WHITE);
        d.draw_circle(x0, y, 3.0, Color::WHITE);
        d.draw_circle(xf, y, 3.0, Color::WHITE);
        y += 4;
    }
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1080, 700)
        .title("Hello world!")
        .msaa_4x()
        .resizable()
        .build();

    rl.set_target_fps(120);
    rl.maximize_window();

    let seed = 414;
    let mut rng = StdRng::seed_from_u64(seed);

    let slots: Vec<InvSlot> = (0..50)
        .map(|_| {
            let capacity = Volume::liters(rng.random_range(10..1000));
            let filter = ItemFilter::Any;
            let is_fluid = false;
            let location = (PartCoord::ZERO, PartCoord::ZERO);
            InvSlot::new(capacity, filter, is_fluid, location)
        })
        .collect();

    let pipes = (0..100)
        .map(|_| {
            let a = rng.random_range(0..slots.len() - 1);
            let b = rng.random_range(0..slots.len() - 1);
            (a, b)
        })
        .collect();

    let sources = vec![(5, Item::Iron), (12, Item::Water), (17, Item::Bread)];

    let mut grid = GridVentory {
        ticks: 0,
        slots,
        pipes,
        sources,
        is_settled: false,
    };

    for (a, b) in &grid.pipes {
        if grid.pipes.contains(&(*b, *a)) {
            println!("Loopback detected");
            break;
        }
    }

    let start = Instant::now();

    let total_ticks = TICKS_PER_SECOND * 3600 * 24 * 10;

    println!("{:?}", grid.mass());
    println!("Simulating...");
    // for _ in 0..total_ticks {
    //     update_inventory(&mut grid);

    //     if grid.is_settled {
    //         println!("Settled at tick {}", grid.ticks);
    //         break;
    //     }
    // }

    for slot in &grid.slots {
        println!("{:?}", slot);
    }

    let elapsed = Instant::now() - start;

    let sim_elapsed = apparent_elapsed_time(total_ticks);

    println!("{:?}", grid.mass());
    println!(
        "Done: took {:?}, sim: {:?}, speed: {:0.1}x",
        elapsed,
        sim_elapsed,
        sim_elapsed.as_secs_f64() / elapsed.as_secs_f64()
    );

    while !rl.window_should_close() {
        for _ in 0..10 {
            update_inventory(&mut grid);
        }

        rl.draw(&thread, |mut d| {
            d.clear_background(Color::BLACK);

            draw_gridventory(&mut d, &grid);
        });
    }
}
