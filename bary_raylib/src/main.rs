use raylib::prelude::*;

#[derive(Debug)]
struct Spacecraft {
    pos: Vector2,
    vel: Vector2,
    heading: f32,
    texture: Texture2D,
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
        d.draw_texture(
            &self.texture,
            self.pos.x as i32,
            self.pos.y as i32,
            Color::WHITE,
        );
    }
}

#[derive(Debug)]
struct Moon {
    pos: Vector2,
    radius: f32,
}

impl Moon {
    fn draw(&self, d: &mut RaylibDrawHandle, camera: &Camera2D) {
        let p = d.get_world_to_screen2D(self.pos, camera);
        d.draw_circle_lines(p.x as i32, p.y as i32, self.radius, Color::WHITE);
    }
}

#[derive(Debug)]
struct World {
    camera: Camera2D,
    spacecraft: Spacecraft,
    moon: Moon,
}

impl World {
    fn test_scene(texture: Texture2D) -> Self {
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
                texture,
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

fn update_world(world: &mut World) {
    update_spacecraft(&mut world.spacecraft);
}

fn draw_world(world: &World, d: &mut RaylibDrawHandle) {
    world.moon.draw(d, &world.camera);
    world.spacecraft.draw(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_world() {
        let (mut rl, thread) = raylib::init();
        let texture = rl
            .load_texture(&thread, "../assets/parts/cargo/skin.png")
            .unwrap();
        let mut world = World::test_scene(texture);

        for _ in 0..100 {
            update_world(&mut world);
        }

        dbg!(&world);
    }
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1080, 700)
        .title("Hello world!")
        .vsync()
        .msaa_4x()
        .resizable()
        .build();

    rl.maximize_window();

    let texture = rl
        .load_texture(&thread, "assets/parts/cargo/skin.png")
        .unwrap();

    let mut world = World::test_scene(texture);

    while !rl.window_should_close() {
        update_world(&mut world);

        let s = format!(
            "{} FPS\n{:?}\n{:?}\n{:?}\n{:#?}",
            rl.get_fps(),
            rl.is_cursor_on_screen(),
            rl.get_mouse_position(),
            rl.get_mouse_delta(),
            &world.spacecraft
        );

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);

        draw_world(&world, &mut d);

        d.draw_text(&s, 12, 12, 20, Color::WHITE);
    }
}
