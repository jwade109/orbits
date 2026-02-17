use bary_core::prelude::*;
use raylib::prelude::*;

#[derive(Debug)]
pub struct Spacecraft {
    pub pos: Vector2,
    pub vel: Vector2,
    pub heading: f32,
}

impl Spacecraft {
    pub fn vertices(&self) -> [Vector2; 3] {
        const SIZE: f32 = 200.0;

        let px = self.heading.cos();
        let py = self.heading.sin();
        let pointing = Vector2::new(px, py);

        let a = self.pos + pointing * SIZE;
        let pointing = pointing.rotated(-2.0 * std::f32::consts::PI / 3.0);
        let b = self.pos + pointing * SIZE / 2.0;
        let pointing = pointing.rotated(-2.0 * std::f32::consts::PI / 3.0);
        let c = self.pos + pointing * SIZE / 2.0;

        [a, b, c]
    }

    pub fn draw(&self, d: &mut RaylibDrawHandle) {
        let [v1, v2, v3] = self.vertices();
        d.draw_triangle_lines(v1, v2, v3, Color::WHITE);
        d.draw_circle(self.pos.x as i32, self.pos.y as i32, 5.0, Color::RED);
    }
}

#[derive(Debug)]
pub struct Moon {
    pub pos: Vector2,
    pub radius: f32,
}

impl Moon {
    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera2D) {
        let p = d.get_world_to_screen2D(self.pos, camera);
        d.draw_circle_lines(p.x as i32, p.y as i32, self.radius, Color::WHITE);
    }
}

#[derive(Debug)]
pub struct World {
    pub camera: Camera2D,
    pub spacecraft: Spacecraft,
    pub moon: Moon,
}

impl World {
    pub fn test_scene() -> Self {
        Self {
            camera: Camera2D {
                offset: Vector2::new(-100.0, 0.0),
                target: Vector2::new(-500.0, -500.0),
                rotation: 0.0,
                zoom: 1.0,
            },
            spacecraft: Spacecraft {
                pos: Vector2::new(600.0, 200.0),
                vel: Vector2::new(6.0, 4.0),
                heading: 0.3,
            },
            moon: Moon {
                pos: Vector2::zero(),
                radius: 500.0,
            },
        }
    }
}

fn update_spacecraft(sc: &mut Spacecraft) {
    let dt = 0.01;
    sc.heading += 0.01;
    sc.pos += sc.vel * dt;
}

pub fn update_world(world: &mut World) {
    update_spacecraft(&mut world.spacecraft);
}

pub fn draw_world(world: &World, d: &mut RaylibDrawHandle) {
    world.moon.draw(d, &world.camera);
    world.spacecraft.draw(d)
}

pub fn draw_blueprint(bp: &Blueprint, offset: Vec2, d: &mut RaylibDrawHandle) {
    for draw_layer in PartLayer::draw_order() {
        let color = match draw_layer {
            PartLayer::Exterior => Color::WHITE,
            PartLayer::Internal => Color::BLUE,
            PartLayer::Plumbing => continue,
            PartLayer::Structural => Color::GRAY,
        };
        for (_, part) in bp.parts() {
            if part.layer() != draw_layer {
                continue;
            }

            let bl = offset + part.placement.bottom_left().to_meters() * 30.0;
            let tr = offset + part.placement.top_right().to_meters() * 30.0;
            let rectangle = Rectangle::new(bl.x, bl.y, tr.x - bl.x, tr.y - bl.y);
            d.draw_rectangle_lines_ex(rectangle, 2.0, color);
        }
    }
}
