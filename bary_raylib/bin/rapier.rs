use bary_core::prelude::*;
use bary_raylib::utils::{glam_to_raylib_swap_x, glam_to_raylib_swap_y};
use rapier2d::prelude::*;
use raylib::prelude::*;

struct RapierPhysics {
    gravity: Vector,
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
}

impl RapierPhysics {
    fn new() -> Self {
        Self {
            gravity: Vector::new(0.0, -9.81),
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
        }
    }

    fn step(&mut self) {
        self.physics_pipeline.step(
            self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            &(),
            &(),
        );
    }

    fn add_floor(&mut self) {
        let rigid_body = RigidBodyBuilder::fixed()
            .translation(Vector::new(0.0, -20.0))
            .build();
        let collider = ColliderBuilder::cuboid(250.0, 20.0).friction(0.7).build();
        let handle = self.rigid_body_set.insert(rigid_body);
        self.collider_set
            .insert_with_parent(collider, handle, &mut self.rigid_body_set);
    }

    fn add_ball(&mut self, radius: f32) -> RigidBodyHandle {
        let p = randvec(5.0, 30.0);

        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(Vector::new(p.x, p.y))
            .linvel(Vector::new(0.0, 0.0))
            .build();

        let collider = ColliderBuilder::cuboid(radius, radius)
            .restitution(0.7)
            .build();
        let ball_body_handle = self.rigid_body_set.insert(rigid_body);
        self.collider_set
            .insert_with_parent(collider, ball_body_handle, &mut self.rigid_body_set);
        ball_body_handle
    }
}

fn init_raylib_window() -> (RaylibHandle, RaylibThread) {
    raylib::init()
        .size(1080, 700)
        .title("Rapier Physics Demo")
        .log_level(TraceLogLevel::LOG_WARNING)
        .msaa_4x()
        .resizable()
        .vsync()
        .build()
}

fn rect_from_center_and_dims(center: Vec2, dims: Vec2) -> Rectangle {
    Rectangle::new(
        center.x - dims.x / 2.0,
        center.y - dims.y / 2.0,
        dims.x,
        dims.y,
    )
}

fn draw_isometry(d: &mut RaylibDrawHandle, iso: Isometry2d) {
    let rec = rect_from_center_and_dims(Vec2::ZERO, Vec2::splat(3.0));

    // phenomenally goddamn confusing
    let o = glam_to_raylib_swap_x(iso.translation);
    let o2 = glam_to_raylib_swap_y(iso.translation);
    let x = glam_to_raylib_swap_y(iso.local_x() * 10.0);
    let y = glam_to_raylib_swap_y(iso.local_y() * 10.0);

    d.draw_rectangle_pro(rec, o, 0.0, Color::WHITE);
    d.draw_line_ex(o2, o2 + x, 1.0, Color::RED);
    d.draw_line_ex(o2, o2 + y, 1.0, Color::GREEN);
}

fn main() {
    let mut physics = RapierPhysics::new();

    let mut balls = Vec::new();
    for _ in 0..100 {
        let radius = rand(1.0, 4.0);
        let ball = physics.add_ball(radius);
        balls.push((ball, radius));
    }

    physics.add_floor();

    let (mut rl, thread) = init_raylib_window();

    let mut camera = Camera2D {
        target: raylib::math::Vector2::zero(),
        offset: raylib::math::Vector2::zero(),
        zoom: 4.0,
        rotation: 0.0,
    };

    while !rl.window_should_close() {
        physics.step();

        let w = rl.get_render_width();
        let h = rl.get_render_height();

        camera.offset = raylib::math::Vector2::new(w as f32, h as f32) / 2.0;

        rl.draw(&thread, |mut d| {
            d.clear_background(Color::BLACK);
            d.draw_fps(12, 12);
            d.draw_mode2D(camera, |mut d, _camera| {
                for (_, body) in physics.rigid_body_set.iter() {
                    let iso = Isometry2d::new(body.translation(), body.rotation().angle());
                    draw_isometry(&mut d, iso);

                    // for c in body.colliders() {
                    //     let Some(c) = physics.collider_set.get(*c) else {
                    //         continue;
                    //     };

                    //     let aabb = c.compute_aabb();
                    //     let dims = aabb.maxs - aabb.mins;
                    //     let rect =
                    //         Rectangle::new(aabb.mins.x, -aabb.mins.y - dims.y, dims.x, dims.y);
                    //     d.draw_rectangle_lines_ex(rect, 0.1, Color::WHITE);

                    //     if let Some(cuboid) = c.shared_shape().as_cuboid() {
                    //         let q = glam_to_raylib(cuboid.half_extents);
                    //         let rect = Rectangle::new(-q.x, -q.y, q.x * 2.0, q.y * 2.0);
                    //         d.draw_rectangle_pro(
                    //             rect,
                    //             glam_to_raylib(p),
                    //             body.rotation().angle(),
                    //             Color::RED.alpha(0.2),
                    //         )
                    //     }
                    // }
                }
            });
        });
    }
}
