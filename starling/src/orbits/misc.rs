use crate::core::*;
use crate::orbits::sparse_orbit::SparseOrbit;
use crate::orbits::universal::universal_lagrange;
use crate::pv::*;

pub fn export_orbit_data(
    orbit: &SparseOrbit,
    filename: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let orbit_data_path = std::path::Path::new("orbit_data/");
    std::fs::create_dir_all(&orbit_data_path)?;

    let a = orbit.period().unwrap_or(Nanotime::secs(30)).to_secs();

    let ftime = linspace(-a, a, 100000);

    let nt = apply(&ftime, |x| Nanotime::secs_f32(x));

    let data = apply(&nt, |x| {
        universal_lagrange(orbit.initial, x, orbit.body.mu())
    });

    let x = apply(&data, |x| x.1.map(|d| d.pv.pos.x).unwrap_or(f32::NAN));
    let y = apply(&data, |x| x.1.map(|d| d.pv.pos.y).unwrap_or(f32::NAN));
    let vx = apply(&data, |x| x.1.map(|d| d.pv.vel.x).unwrap_or(f32::NAN));
    let vy = apply(&data, |x| x.1.map(|d| d.pv.vel.y).unwrap_or(f32::NAN));
    let r = apply(&data, |x| {
        x.1.map(|d| d.pv.pos.length()).unwrap_or(f32::NAN)
    });
    let z = apply(&data, |x| x.1.map(|d| d.z).unwrap_or(f32::NAN));
    let f = apply(&data, |x| x.1.map(|d| d.lc.f).unwrap_or(f32::NAN));
    let g = apply(&data, |x| x.1.map(|d| d.lc.g).unwrap_or(f32::NAN));
    let fdot = apply(&data, |x| x.1.map(|d| d.lc.fdot).unwrap_or(f32::NAN));
    let gdot = apply(&data, |x| x.1.map(|d| d.lc.gdot).unwrap_or(f32::NAN));

    write_csv(
        &orbit_data_path.join(filename),
        &[
            ("t", &ftime),
            ("x", &x),
            ("y", &y),
            ("vx", &vx),
            ("vy", &vy),
            ("r", &r),
            ("z", &z),
            ("f", &f),
            ("g", &g),
            ("fdot", &fdot),
            ("gdot", &gdot),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orbits::sparse_orbit::Body;
    use crate::orbits::universal::*;
    use approx::assert_relative_eq;
    use glam::f32::Vec2;

    #[test]
    fn universal_lagrange_example() {
        let vec_r_0 = Vec2::new(7000.0, -12124.0);
        let vec_v_0 = Vec2::new(2.6679, 4.6210);
        let mu = 3.986004418E5;

        let tof = Nanotime::secs(3600);

        let (_, res) = super::universal_lagrange((vec_r_0, vec_v_0), tof, mu);

        assert_eq!(
            res.unwrap().pv,
            PV::new((-3297.7869, 7413.3867), (-8.297602, -0.9640651))
        );
    }

    #[test]
    fn stumpff() {
        assert_relative_eq!(stumpff_2(-20.0), 2.1388736);
        assert_relative_eq!(stumpff_2(-5.0), 0.74633473);
        assert_relative_eq!(stumpff_2(-1.0), 0.5430807);
        assert_relative_eq!(stumpff_2(-1E-6), 0.50000006);
        assert_relative_eq!(stumpff_2(-1E-12), 0.5);
        assert_relative_eq!(stumpff_2(0.0), 0.5);
        assert_relative_eq!(stumpff_2(1E-12), 0.5);
        assert_relative_eq!(stumpff_2(1E-6), 0.49999997);
        assert_relative_eq!(stumpff_2(1.0), 0.45969772);
        assert_relative_eq!(stumpff_2(5.0), 0.32345456);
        assert_relative_eq!(stumpff_2(20.0), 0.061897416);

        assert_relative_eq!(stumpff_3(-20.0), 0.43931928);
        assert_relative_eq!(stumpff_3(-1E-12), 0.16666667);
        assert_relative_eq!(stumpff_3(0.0), 0.16666667);
        assert_relative_eq!(stumpff_3(1E-12), 0.16666667);
        assert_relative_eq!(stumpff_3(20.0), 0.060859215);
    }

    #[test]
    fn bad_orbit() {
        let pv = PV::new((825.33563, 564.6425), (200.0, 230.0));
        let body = Body {
            radius: 63.0,
            mass: 1000.0,
            soi: 15000.0,
        };
        let orbit = SparseOrbit::from_pv(pv, body, Nanotime(0));
        dbg!(orbit);
    }

    #[test]
    fn orbit_construction() {
        const TEST_POSITION: Vec2 = Vec2::new(500.0, 300.0);
        const TEST_VELOCITY: Vec2 = Vec2::new(-200.0, 0.0);

        let body = Body {
            radius: 100.0,
            mass: 1000.0,
            soi: 10000.0,
        };

        let o1 = SparseOrbit::from_pv((TEST_POSITION, TEST_VELOCITY), body, Nanotime(0)).unwrap();
        let o2 = SparseOrbit::from_pv((TEST_POSITION, -TEST_VELOCITY), body, Nanotime(0)).unwrap();

        let true_h = TEST_POSITION.extend(0.0).cross(TEST_VELOCITY.extend(0.0)).z;

        assert_relative_eq!(o1.angular_momentum(), true_h);
        assert!(o1.angular_momentum() > 0.0);
        assert!(!o1.retrograde);

        assert_relative_eq!(o2.angular_momentum(), true_h);
        assert!(o1.angular_momentum() > 0.0);
        assert!(o2.retrograde);

        assert_eq!(o1.period().unwrap(), o2.period().unwrap());

        // TODO make this better
        for i in [0] {
            let t = o1.period().unwrap() * i;
            println!("{t:?} {} {}", o1.pv_at_time(t), o2.pv_at_time(t));
            assert_relative_eq!(
                o1.pv_at_time(t).pos.x,
                o2.pv_at_time(t).pos.x,
                epsilon = 0.5
            );
            assert_relative_eq!(
                o1.pv_at_time(t).pos.y,
                o2.pv_at_time(t).pos.y,
                epsilon = 0.5
            );
        }
    }
}
