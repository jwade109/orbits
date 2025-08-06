use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct ConBot {
    pv: PV,
    angle: f64,
    target_pos: Option<DVec2>,
    target_part: Option<PartId>,
}

const CONBOT_PD_CTRL: PDCtrl = PDCtrl::new(50.0, 40.0);

impl ConBot {
    pub fn new(pv: PV) -> Self {
        Self {
            pv,
            angle: rand(0.0, 2.0 * PI) as f64,
            target_pos: Some(randvec(10.0, 30.0).as_dvec2()),
            target_part: None,
        }
    }

    pub fn pos(&self) -> DVec2 {
        self.pv.pos
    }

    pub fn angle(&self) -> f64 {
        self.angle
    }

    pub fn set_target_pos(&mut self, pos: DVec2) {
        self.target_pos = Some(pos);
    }

    pub fn target_pos(&self) -> Option<DVec2> {
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

        const MAX_ACCEL: f64 = 6.0;
        const MAX_VEL: f64 = 4.0;

        if let Some(target_pos) = self.target_pos {
            let dx = target_pos.x - self.pos().x;
            let dy = target_pos.y - self.pos().y;

            let vx = self.pv.vel.x;
            let vy = self.pv.vel.y;
            let ax = CONBOT_PD_CTRL.apply(dx, vx);
            let ay = CONBOT_PD_CTRL.apply(dy, vy);

            let a = DVec2::new(ax, ay).clamp(-DVec2::splat(MAX_ACCEL), DVec2::splat(MAX_ACCEL));
            self.pv.vel += a * dt.to_secs_f64();
        }

        self.pv.vel = self.pv.vel.clamp_length(0.0, MAX_VEL);

        self.pv.pos += self.pv.vel * dt.to_secs_f64();

        let target_angle = if self.pv.vel.length() < 2.0 {
            self.angle
        } else {
            self.pv.vel.to_angle() as f64
        };

        self.angle += (target_angle - self.angle) * 0.1;
    }
}
