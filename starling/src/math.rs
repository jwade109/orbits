use crate::nanotime::Nanotime;
use glam::f32::Vec2;
use rand::Rng;

pub const PI: f32 = std::f32::consts::PI;

pub fn rand(min: f32, max: f32) -> f32 {
    rand::thread_rng().gen_range(min..max)
}

pub fn randvec(min: f32, max: f32) -> Vec2 {
    let rot = Vec2::from_angle(rand(0.0, std::f32::consts::PI * 2.0));
    let mag = rand(min, max);
    rot.rotate(Vec2::new(mag, 0.0))
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

// vector projection, rejection of a onto b
pub fn vproj(a: Vec2, b: Vec2) -> (Vec2, Vec2) {
    let bu = b.normalize_or_zero();
    let proj = a.dot(bu) * bu;
    (proj, a - proj)
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
    (0..nsamples)
        .map(|i| start.lerp(end, i as f32 / (nsamples - 1) as f32))
        .collect()
}

pub fn bhaskara_sin_approx(x: f32) -> f32 {
    let xp = x.abs();
    x.signum() * 16.0 * xp * (PI - xp) / (5.0 * PI.powi(2) - 4.0 * xp * (PI - xp))
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
