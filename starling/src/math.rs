use crate::nanotime::Nanotime;
pub use glam::f32::Vec2;
pub use glam::f32::Vec3;
pub use glam::i32::IVec2;
use names::Generator;
use rand::Rng;

pub const PI: f32 = std::f32::consts::PI;

pub fn rand(min: f32, max: f32) -> f32 {
    rand::thread_rng().gen_range(min..max)
}

pub fn randint(min: i32, max: i32) -> i32 {
    rand::thread_rng().gen_range(min..max)
}

pub fn randvec(min: f32, max: f32) -> Vec2 {
    let rot = Vec2::from_angle(rand(0.0, std::f32::consts::PI * 2.0));
    let mag = rand(min, max);
    rot.rotate(Vec2::new(mag, 0.0))
}

pub fn randvec3(min: f32, max: f32) -> Vec3 {
    let r = rand(min, max);
    let a = rand(0.0, 2.0 * PI);
    let z = rand(-1.0, 1.0);
    let p = Vec3::new(
        (1.0 - z.powi(2)).sqrt() * a.cos(),
        (1.0 - z.powi(2)).sqrt() * a.sin(),
        z,
    );
    r * p
}

pub fn rotate(v: Vec2, angle: f32) -> Vec2 {
    Vec2::from_angle(angle).rotate(v)
}

pub fn cross2d(a: Vec2, b: Vec2) -> f32 {
    a.extend(0.0).cross(b.extend(0.0)).z
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// maps x from the range [a, b] to [p, q]
pub fn linmap(x: f32, a: f32, b: f32, p: f32, q: f32) -> f32 {
    let s = (x - a) / (b - a);
    lerp(p, q, s)
}

/// vector projection, rejection of a onto b
pub fn vproj(a: Vec2, b: Vec2) -> (Vec2, Vec2) {
    let bu = b.normalize_or_zero();
    let proj = a.dot(bu) * bu;
    (proj, a - proj)
}

pub fn vfloor(v: Vec2) -> IVec2 {
    IVec2::new(v.x.floor() as i32, v.y.floor() as i32)
}

pub fn vceil(v: Vec2) -> IVec2 {
    IVec2::new(v.x.ceil() as i32, v.y.ceil() as i32)
}

pub fn vround(v: Vec2) -> IVec2 {
    IVec2::new(v.x.round() as i32, v.y.round() as i32)
}

pub fn apply<T: Copy, R>(x: &Vec<T>, func: impl Fn(T) -> R) -> Vec<R> {
    x.iter().map(|x| func(*x)).collect()
}

pub fn apply_filter<T: Copy, K, R>(
    x: &Vec<T>,
    func: impl Fn(T) -> Option<(K, R)>,
) -> (Vec<K>, Vec<R>) {
    x.iter().filter_map(|x| func(*x)).collect()
}

pub fn linspace(a: f32, b: f32, n: usize) -> Vec<f32> {
    if n < 2 {
        return vec![a];
    }
    if n == 2 {
        return vec![a, b];
    }
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32;
            lerp(a, b, t)
        })
        .collect()
}

pub fn tspace(start: Nanotime, end: Nanotime, nsamples: usize) -> Vec<Nanotime> {
    if start > end {
        return Vec::new();
    }
    (0..nsamples)
        .map(|i| start.lerp(end, i as f32 / (nsamples - 1) as f32))
        .collect()
}

pub fn bhaskara_sin_approx(x: f32) -> f32 {
    let xp = x.abs();
    x.signum() * 16.0 * xp * (PI - xp) / (5.0 * PI.powi(2) - 4.0 * xp * (PI - xp))
}

pub fn is_occluded(light_source: Vec2, test: Vec2, object: Vec2, radius: f32) -> bool {
    let test = test - light_source;
    let object = object - light_source;

    //
    //                      * * *
    //                   *      /  *
    //        @ T       *      / r  *
    //                  *     @     *
    //                  *      O    *
    //                   *         *
    //                      * * *
    //
    //   @ L (0, 0)
    //

    let dobj = object.length();

    if object.distance(test) < radius {
        return true;
    }

    if dobj < radius {
        return true;
    }

    if test.length() < dobj {
        return false;
    }

    let angular_radius = (radius / dobj).asin();
    let angle = test.angle_to(object);
    angle.abs() < angular_radius
}

pub fn get_random_name() -> String {
    let mut generator = Generator::default();
    generator.next().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_float_eq::assert_float_absolute_eq;

    #[test]
    fn linspace_is_cool() {
        let t = linspace(-0.3, 0.6, 12);

        assert!(t.len() == 12);

        assert_eq!(t[0], -0.3);

        assert_float_absolute_eq!(t[1], -0.21818182);
        assert_float_absolute_eq!(t[2], -0.13636364);
        assert_float_absolute_eq!(t[3], -0.054545447);
        assert_float_absolute_eq!(t[4], 0.027272731);
        assert_float_absolute_eq!(t[5], 0.109090924);
        assert_float_absolute_eq!(t[6], 0.19090912);
        assert_float_absolute_eq!(t[7], 0.27272725);
        assert_float_absolute_eq!(t[8], 0.35454547);
        assert_float_absolute_eq!(t[9], 0.43636364);
        assert_float_absolute_eq!(t[10], 0.51818186);

        assert_eq!(t[11], 0.6);
    }
}
