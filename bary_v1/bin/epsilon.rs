use float_next_after::*;

const DISTANCE_TO_MOON_METERS: u32 = 384_400_000;
const ASTRONOMICAL_UNIT: u64 = 149_597_870_700;

fn check_precision_f32(x: f32) {
    let e = x.next_after(x + 1000000.0);
    println!(
        "Precision at {:9e} km is {:11.3e} mm {:20.1}% to Luna {:8.1} AU",
        x / 1000.0,
        (e - x) * 1000.0,
        100.0 * x as f64 / DISTANCE_TO_MOON_METERS as f64,
        100.0 * x as f64 / ASTRONOMICAL_UNIT as f64,
    );
}

fn check_precision_f64(x: f64) {
    let e = x.next_after(x + 1000000.0);
    println!(
        "Precision at {:9e} km is {:11.3e} mm {:20.1}% to Luna {:8.1} AU",
        x / 1000.0,
        (e - x) * 1000.0,
        100.0 * x as f64 / DISTANCE_TO_MOON_METERS as f64,
        100.0 * x as f64 / ASTRONOMICAL_UNIT as f64,
    );
}

fn main() {
    let nums = [
        1.0f64,
        2.0,
        3.0,
        10.0,
        11.0,
        12.0,
        100.0,
        1_000.0,
        10_000.0,
        100_000.0,
        10_000_000.0,
        100_000_000.0,
        1_000_000_000.0,
        10_000_000_000.0,
        100_000_000_000.0,
        1_000_000_000_000.0,
        10_000_000_000_000.0,
    ];

    println!("f32 ===");

    for m in nums {
        check_precision_f32(m as f32)
    }

    println!("f64 ===");

    for m in nums {
        check_precision_f64(m)
    }
}
