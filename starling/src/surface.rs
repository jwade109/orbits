use crate::math::*;
use crate::terrain::*;
use splines::{Key, Spline};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Surface {
    pub gravity: f32,
    pub wind: f32,
    pub radius: f32,
    pub atmo_color: [f32; 3],
    pub land_color: [f32; 3],
    pub elevation: Spline<f32, f32>,
    pub terrain: HashMap<IVec2, TerrainChunk>,
}

impl Surface {
    pub fn random() -> Self {
        let mut keys = Vec::new();
        let mut y = 0.0;

        for x in linspace(-1000.0, 1000.0, 1000) {
            y += rand(-2.0, 2.0);
            keys.push(Key::new(x, y, splines::Interpolation::Linear));
        }

        let elevation = Spline::from_vec(keys);
        let mut terrain = HashMap::new();
        for x in -15..=15 {
            let x_elev = x as f32 * CHUNK_WIDTH_METERS;

            if let Some(y_elev) = elevation.clamped_sample(x_elev) {
                let chunk_pos = world_pos_to_chunk(Vec2::new(x_elev, y_elev));

                for yoff in -1..=1 {
                    let chunk_pos = chunk_pos + IVec2::Y * yoff;
                    let chunk = TerrainChunk::with_elevation(&elevation, chunk_pos);
                    terrain.insert(chunk_pos, chunk);
                }
            }
        }

        Surface {
            gravity: 2.0,
            wind: rand(-2.0, 2.0),
            radius: 2000.0,
            atmo_color: [rand(0.1, 0.4), rand(0.1, 0.4), rand(0.1, 0.4)],
            land_color: [rand(0.1, 0.4), rand(0.1, 0.4), rand(0.1, 0.4)],
            elevation,
            terrain,
        }
    }

    pub fn on_sim_tick(&mut self) {
        // self.wind = 0.0;
        // self.gravity = 0.0;
    }

    fn gravity_vector(&self) -> Vec2 {
        Vec2::new(0.0, -self.gravity)
    }

    fn wind_vector(&self) -> Vec2 {
        Vec2::new(self.wind, 0.0)
    }

    pub fn external_acceleration(&self) -> Vec2 {
        self.gravity_vector() + self.wind_vector()
    }

    pub fn increase_gravity(&mut self) {
        self.gravity += 0.1;
    }

    pub fn decrease_gravity(&mut self) {
        self.gravity -= 0.1;
    }

    pub fn increase_wind(&mut self) {
        self.wind += 0.1;
    }

    pub fn decrease_wind(&mut self) {
        self.wind -= 0.1;
    }

    pub fn elevation(&self, x: f32) -> f32 {
        self.elevation.clamped_sample(x).unwrap_or(0.0)
    }
}
