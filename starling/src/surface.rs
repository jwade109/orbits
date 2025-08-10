use crate::math::*;
use crate::orbits::Body;
use crate::thrust_particles::*;
use splines::Key;

#[derive(Debug)]
pub struct Surface {
    pub body: Body,
    pub atmo_density: f32,
    pub atmo_color: [f32; 3],
    pub land_color: [f32; 3],
    pub particles: ThrustParticleEffects,
}

impl Surface {
    pub fn random() -> Self {
        let mut keys = Vec::new();
        let mut y = 0.0;

        for x in linspace(-1000.0, 1000.0, 1000) {
            y += rand(-2.0, 2.0);
            keys.push(Key::new(x, y, splines::Interpolation::Linear));
        }

        Surface {
            body: Body::LUNA,
            atmo_density: rand(0.0, 1.0),
            atmo_color: [rand(0.1, 0.2), rand(0.1, 0.2), rand(0.1, 0.2)],
            land_color: [rand(0.1, 0.4), rand(0.1, 0.4), rand(0.1, 0.4)],
            particles: ThrustParticleEffects::new(),
        }
    }

    pub fn on_sim_tick(&mut self) {
        self.particles.step();
    }
}
