use crate::ui::OnClick;
use crate::mouse::{FrameId, MouseButt, MouseState};
use crate::planetary::{CursorMode, GameState};
use crate::scenes::Scene;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug, Clone)]
pub struct OrbitalScene {
    primary: PlanetId,
}

impl OrbitalScene {
    pub fn new(primary: PlanetId) -> Self {
        Self { primary }
    }
}

pub struct OrbitalView<'a> {
    pub info: &'a OrbitalScene,
    pub mouse: &'a MouseState,
    pub ui: &'a Tree<OnClick>,
    pub scene: &'a Scene,
}

impl<'a> OrbitalView<'a> {
    pub fn measuring_tape(&self) -> Option<(Vec2, Vec2, Vec2)> {
        let mouse: &MouseState = self.scene.mouse_if_world(self.mouse)?;
        let a = mouse.world_position(MouseButt::Left, FrameId::Down)?;
        let b = mouse.world_position(MouseButt::Left, FrameId::Current)?;
        let corner = Vec2::new(a.x, b.y);
        Some((a, b, corner))
    }

    pub fn cursor_pv(p1: Vec2, p2: Vec2, state: &GameState) -> Option<PV> {
        if p1.distance(p2) < 20.0 {
            return None;
        }

        let wrt_id = state.scenario.relevant_body(p1, state.sim_time)?;
        let parent = state.scenario.lup_planet(wrt_id, state.sim_time)?;

        let r = p1.distance(parent.pv().pos);
        let v = (parent.body()?.mu() / r).sqrt();

        Some(PV::new(p1, (p2 - p1) * v / r))
    }

    pub fn cursor_orbit(p1: Vec2, p2: Vec2, state: &GameState) -> Option<GlobalOrbit> {
        let pv = Self::cursor_pv(p1, p2, &state)?;
        let parent_id = state.scenario.relevant_body(pv.pos, state.sim_time)?;
        let parent = state.scenario.lup_planet(parent_id, state.sim_time)?;
        let parent_pv = parent.pv();
        let pv = pv - PV::pos(parent_pv.pos);
        let body = parent.body()?;
        Some(GlobalOrbit(
            parent_id,
            SparseOrbit::from_pv(pv, body, state.sim_time)?,
        ))
    }

    pub fn left_cursor_orbit(&self, state: &GameState) -> Option<GlobalOrbit> {
        let mouse = self.scene.mouse_if_world(&self.mouse)?;
        let a = self.mouse.world_position(MouseButt::Left, FrameId::Down)?;
        let b = self
            .mouse
            .world_position(MouseButt::Left, FrameId::Current)?;
        Self::cursor_orbit(a, b, state)
    }

    pub fn right_cursor_orbit(&self, state: &GameState) -> Option<GlobalOrbit> {
        let mouse = self.scene.mouse_if_world(&self.mouse)?;
        let a = mouse.world_position(MouseButt::Right, FrameId::Down)?;
        let b = mouse.world_position(MouseButt::Right, FrameId::Current)?;
        Self::cursor_orbit(a, b, state)
    }

    pub fn selection_region(&self, state: &GameState) -> Option<Region> {
        let mouse: &MouseState = self.scene.mouse_if_world(&self.mouse)?;
        match state.selection_mode {
            CursorMode::Rect => {
                let a = mouse.world_position(MouseButt::Left, FrameId::Down)?;
                let b = mouse.world_position(MouseButt::Left, FrameId::Current)?;
                Some(Region::aabb(a, b))
            }
            CursorMode::Altitude => {
                let a = mouse.world_position(MouseButt::Left, FrameId::Down)?;
                let b = mouse.world_position(MouseButt::Left, FrameId::Current)?;
                Some(Region::altitude(a, b))
            }
            CursorMode::NearOrbit => self
                .left_cursor_orbit(state)
                .map(|GlobalOrbit(_, orbit)| Region::NearOrbit(orbit, 50.0)),
            CursorMode::Measure => None,
        }
    }

    pub fn follow_position(&self, state: &GameState) -> Option<Vec2> {
        let id = state.follow?;
        let lup = match id {
            ObjectId::Orbiter(id) => state.scenario.lup_orbiter(id, state.sim_time)?,
            ObjectId::Planet(id) => state.scenario.lup_planet(id, state.sim_time)?,
        };
        Some(lup.pv().pos)
    }
}
