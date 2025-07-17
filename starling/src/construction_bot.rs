use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ConBot {
    pv: PV,
    angle: f32,
    target_pos: Option<Vec2>,
    target_part: Option<PartId>,
}

const CONBOT_PD_CTRL: PDCtrl = PDCtrl::new(50.0, 40.0);

impl ConBot {
    pub fn new(pv: PV) -> Self {
        Self {
            pv,
            angle: rand(0.0, 2.0 * PI),
            target_pos: Some(randvec(10.0, 30.0)),
            target_part: None,
        }
    }

    pub fn pos(&self) -> Vec2 {
        self.pv.pos_f32()
    }

    pub fn angle(&self) -> f32 {
        self.angle
    }

    pub fn set_target_pos(&mut self, pos: Vec2) {
        self.target_pos = Some(pos);
    }

    pub fn target_pos(&self) -> Option<Vec2> {
        self.target_pos
    }

    pub fn target_part(&self) -> Option<PartId> {
        self.target_part
    }

    pub fn set_target_part(&mut self, id: PartId) {
        self.target_part = Some(id);
    }

    pub fn clear_target_part(&mut self) {
        self.target_part = None;
    }

    pub fn on_sim_tick(&mut self) {
        let dt = PHYSICS_CONSTANT_DELTA_TIME;

        if let Some(target_pos) = self.target_pos {
            let dx = target_pos.x - self.pos().x;
            let dy = target_pos.y - self.pos().y;

            let vx = self.pv.vel_f32().x;
            let vy = self.pv.vel_f32().y;
            let ax = CONBOT_PD_CTRL.apply(dx, vx);
            let ay = CONBOT_PD_CTRL.apply(dy, vy);

            let a = Vec2::new(ax, ay)
                .as_dvec2()
                .clamp(-DVec2::splat(30.0), DVec2::splat(30.0));
            self.pv.vel += a * dt.to_secs_f64();
        }

        self.pv.vel = self.pv.vel.clamp_length(0.0, 50.0);

        self.pv.pos += self.pv.vel * dt.to_secs_f64();

        let target_angle = if self.pv.vel.length() < 2.0 {
            self.angle
        } else {
            self.pv.vel.to_angle() as f32
        };

        self.angle += (target_angle - self.angle) * 0.1;
    }
}
