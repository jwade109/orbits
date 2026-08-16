use glm::*;
use rend::*;

use crate::camera::{Camera, CameraControls};

pub struct World {
    pub time: f32,
    pub quads: Vec<MeshObject>,
    pub camera: Camera,
}

impl World {
    pub fn new() -> Self {
        World {
            time: 0.0,
            quads: Vec::new(),
            camera: Camera::new(),
        }
    }

    pub fn ground_plane(&mut self, x: i32, z: i32, mesh_id: usize) {
        self.quads.push(MeshObject {
            position: Vec3::new(x as f32, 0.0, z as f32),
            angle: 0.0,
            vel: 0.0,
            should_animate: false,
            mesh_id,
        });
    }

    pub fn update(&mut self, dt: f32, ctrls: &CameraControls) {
        self.time += dt;
        for quad in &mut self.quads {
            quad.angle = quad.angle + quad.vel * dt;
            if quad.angle > 360.0 {
                quad.angle -= 360.0;
            }

            if quad.should_animate {
                let a = quad.position.x * 0.6 + quad.position.z * 0.4 + self.time / 2.0;
                quad.position.y = a.sin() + 1.0;
            }
        }

        self.camera.apply_controls(ctrls);
        self.camera.update();

        // let z = (self.time / 15.0).sin() * 20.0;
        // let y = (self.time / 11.0).sin() * 4.0 + 10.0;
        // let x = (self.time / 15.0).cos() * 20.0;

        // let target = Vec3::new(0.0, 0.0, 0.0);
        // let eye = Vec3::new(x, y, z);

        // self.camera.target = target;
        // self.camera.eye = eye;
    }
}
