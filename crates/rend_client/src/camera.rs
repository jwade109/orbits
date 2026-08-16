use glm::*;
use rend::translation_matrix;

pub struct Camera {
    heading: f32,
    altitude: f32,
    radius: f32,
    desired_heading: f32,
    desired_altitude: f32,
    desired_radius: f32,
    target: Vec3,
}

#[derive(Default)]
pub enum CamDir {
    #[default]
    Zero,
    Positive,
    Negative,
}

#[derive(Default)]
pub struct CameraControls {
    pub x_axis: CamDir,
    pub y_axis: CamDir,
    pub z_axis: CamDir,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            heading: 0.0,
            altitude: 9.0,
            radius: 28.0,
            desired_heading: 0.2,
            desired_altitude: 9.0,
            desired_radius: 17.0,
            target: Vec3::new(0.0, 5.0, 0.0),
        }
    }

    pub fn eye(&self) -> Vec3 {
        let r = self.radius;
        let x = r * self.heading.cos();
        let z = r * self.heading.sin();

        Vec3::new(x, self.altitude, z)
    }

    pub fn apply_controls(&mut self, ctrls: &CameraControls) {
        match ctrls.x_axis {
            CamDir::Positive => {
                self.desired_heading += 0.01;
            }
            CamDir::Negative => {
                self.desired_heading -= 0.01;
            }
            _ => (),
        }
        match ctrls.y_axis {
            CamDir::Positive => {
                self.desired_altitude += 0.1;
            }
            CamDir::Negative => {
                self.desired_altitude -= 0.1;
            }
            _ => (),
        }
        match ctrls.z_axis {
            CamDir::Positive => {
                self.desired_radius -= 0.1;
            }
            CamDir::Negative => {
                self.desired_radius += 0.1;
            }
            _ => (),
        }

        self.desired_radius = self.desired_radius.clamp(1.0, 250.0);
    }

    pub fn to_projection_matrix(&self, window: &glfw::Window) -> Mat4 {
        let up = normalize(Vec3::new(0.0, 1.0, 0.0));

        let zaxis = normalize(self.eye() - self.target); // forward vector
        let xaxis = normalize(cross(up, zaxis)); // The "right" vector.
        let yaxis = normalize(cross(zaxis, xaxis)); // The "up" vector.

        let orientation = Matrix4::new(
            Vec4::new(xaxis.x, yaxis.x, zaxis.x, 0.0),
            Vec4::new(xaxis.y, yaxis.y, zaxis.y, 0.0),
            Vec4::new(xaxis.z, yaxis.z, zaxis.z, 0.0),
            Vec4::new(0.0, 0.0, 0.0, 1.0),
        );

        let translation = translation_matrix(-self.eye());

        let view = orientation * translation;

        let fov_y: f32 = radians(50.0);
        let (sx, sy) = window.get_size();
        let aspect = sx as f32 / sy as f32;
        let z_near = 0.1;
        let z_far = 1000.0;
        let projection = ext::perspective(fov_y, aspect, z_near, z_far);

        projection * view
    }

    pub fn update(&mut self) {
        self.heading += (self.desired_heading - self.heading) * 0.05;
        self.altitude += (self.desired_altitude - self.altitude) * 0.05;
        self.radius += (self.desired_radius - self.radius) * 0.05;
    }
}
