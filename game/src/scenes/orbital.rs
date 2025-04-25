use crate::mouse::{FrameId, InputState, MouseButt};
use crate::planetary::{CursorMode, GameState};
use crate::scenes::Scene;
use crate::ui::InteractionEvent;
use bevy::utils::hashbrown::HashSet;
use starling::prelude::*;

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct OrbitalContext {
    primary: PlanetId,
    selected: HashSet<OrbiterId>,
}

impl OrbitalContext {
    pub fn new(primary: PlanetId) -> Self {
        Self {
            primary,
            selected: HashSet::new(),
        }
    }

    pub fn on_interaction(&mut self, _inter: &InteractionEvent) {}
}

#[allow(unused)]
pub struct OrbitalView<'a> {
    pub info: &'a OrbitalContext,
    pub input: &'a InputState,
    pub scene: &'a Scene,
}

impl<'a> OrbitalView<'a> {
    pub fn measuring_tape(&self, state: &GameState) -> Option<(Vec2, Vec2, Vec2)> {
        let vb = state.input.screen_bounds.span;
        let wb = state.orbital_camera.world_bounds(vb);
        let mouse: &InputState = self.scene.mouse_if_world(self.input)?;
        let a = mouse.world_position(MouseButt::Left, FrameId::Down, wb)?;
        let b = mouse.world_position(MouseButt::Left, FrameId::Current, wb)?;
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
        let vb = state.input.screen_bounds.span;
        let wb = state.orbital_camera.world_bounds(vb);
        let _mouse = self.scene.mouse_if_world(&self.input)?;
        let a = self
            .input
            .world_position(MouseButt::Left, FrameId::Down, wb)?;
        let b = self
            .input
            .world_position(MouseButt::Left, FrameId::Current, wb)?;
        Self::cursor_orbit(a, b, state)
    }

    pub fn right_cursor_orbit(&self, state: &GameState) -> Option<GlobalOrbit> {
        let vb = state.input.screen_bounds.span;
        let wb = state.orbital_camera.world_bounds(vb);
        let mouse = self.scene.mouse_if_world(&self.input)?;
        let a = mouse.world_position(MouseButt::Right, FrameId::Down, wb)?;
        let b = mouse.world_position(MouseButt::Right, FrameId::Current, wb)?;
        Self::cursor_orbit(a, b, state)
    }

    pub fn selection_region(&self, state: &GameState) -> Option<Region> {
        let vb = state.input.screen_bounds.span;
        let wb = state.orbital_camera.world_bounds(vb);
        let mouse: &InputState = self.scene.mouse_if_world(&self.input)?;
        match state.selection_mode {
            CursorMode::Rect => {
                let a = mouse.world_position(MouseButt::Left, FrameId::Down, wb)?;
                let b = mouse.world_position(MouseButt::Left, FrameId::Current, wb)?;
                Some(Region::aabb(a, b))
            }
            CursorMode::Altitude => {
                let a = mouse.world_position(MouseButt::Left, FrameId::Down, wb)?;
                let b = mouse.world_position(MouseButt::Left, FrameId::Current, wb)?;
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
