use crate::mouse::{FrameId, InputState, MouseButt};
use crate::planetary::GameState;
use crate::scenes::Scene;
use bevy::input::keyboard::KeyCode;
use starling::prelude::*;
use std::collections::HashSet;

pub trait EnumIter
where
    Self: Sized,
{
    fn next(&self) -> Self;

    fn to_next(&mut self) {
        let n = self.next();
        *self = n;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorMode {
    Rect,
    Altitude,
    NearOrbit,
    Measure,
}

impl EnumIter for CursorMode {
    fn next(&self) -> Self {
        match self {
            CursorMode::Rect => CursorMode::Altitude,
            CursorMode::Altitude => CursorMode::NearOrbit,
            CursorMode::NearOrbit => CursorMode::Measure,
            CursorMode::Measure => CursorMode::Rect,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ShowOrbitsState {
    None,
    Focus,
    All,
}

impl EnumIter for ShowOrbitsState {
    fn next(&self) -> Self {
        match self {
            ShowOrbitsState::None => ShowOrbitsState::Focus,
            ShowOrbitsState::Focus => ShowOrbitsState::All,
            ShowOrbitsState::All => ShowOrbitsState::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawMode {
    Default,
    Constellations,
    Stability,
    Occlusion,
}

impl EnumIter for DrawMode {
    fn next(&self) -> Self {
        match self {
            DrawMode::Default => DrawMode::Constellations,
            DrawMode::Constellations => DrawMode::Stability,
            DrawMode::Stability => DrawMode::Occlusion,
            DrawMode::Occlusion => DrawMode::Default,
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct OrbitalContext {
    primary: PlanetId,
    pub selected: HashSet<OrbiterId>,
    pub center: Vec2,
    target_center: Vec2,
    pub scale: f32,
    pub follow: Option<ObjectId>,
    pub queued_orbits: Vec<GlobalOrbit>,
    pub selection_mode: CursorMode,
    pub show_orbits: ShowOrbitsState,
    pub show_animations: bool,
    pub draw_mode: DrawMode,
}

pub trait CameraProjection {
    /// World to camera transform
    fn w2c(&self, p: Vec2) -> Vec2 {
        (p - self.origin()) * self.scale()
    }

    /// Camera to world transform
    fn c2w(&self, p: Vec2) -> Vec2 {
        p / self.scale() + self.origin()
    }

    fn origin(&self) -> Vec2;

    fn scale(&self) -> f32;
}

impl CameraProjection for OrbitalContext {
    fn origin(&self) -> Vec2 {
        self.center
    }

    fn scale(&self) -> f32 {
        self.scale
    }
}

impl OrbitalContext {
    pub fn new(primary: PlanetId) -> Self {
        Self {
            primary,
            selected: HashSet::new(),
            center: Vec2::ZERO,
            target_center: Vec2::ZERO,
            scale: 0.4,
            follow: None,
            queued_orbits: Vec::new(),
            selection_mode: CursorMode::Rect,
            show_orbits: ShowOrbitsState::Focus,
            show_animations: true,
            draw_mode: DrawMode::Default,
        }
    }

    pub fn step(&mut self, input: &InputState) {
        let speed = 16.0;
        if input.is_pressed(KeyCode::Equal) {
            self.scale *= 1.03;
        }
        if input.is_pressed(KeyCode::Minus) {
            self.scale /= 1.03;
        }
        if input.is_pressed(KeyCode::KeyD) {
            self.target_center.x += speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyA) {
            self.target_center.x -= speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyW) {
            self.target_center.y += speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyS) {
            self.target_center.y -= speed / self.scale;
        }
        if input.is_pressed(KeyCode::KeyR) {
            self.target_center = Vec2::ZERO;
            self.scale = 1.0;
        }

        self.center += (self.target_center - self.center) * 0.1;
    }
}

#[allow(unused)]
pub struct OrbitalView<'a> {
    pub info: &'a OrbitalContext,
    pub input: &'a InputState,
    pub scene: &'a Scene,
}

impl<'a> OrbitalView<'a> {
    pub fn measuring_tape(&self, state: &GameState) -> Option<(Vec2, Vec2, Vec2)> {
        let ctx = &state.orbital_context;
        let input: &InputState = self.scene.mouse_if_world(self.input)?;
        let a = input.position(MouseButt::Left, FrameId::Down)?;
        let b = input.position(MouseButt::Left, FrameId::Current)?;
        let a = ctx.c2w(a);
        let b = ctx.c2w(b);
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
        let _mouse = self.scene.mouse_if_world(&self.input)?;
        let ctx = &state.orbital_context;
        let a = self.input.position(MouseButt::Left, FrameId::Down)?;
        let b = self.input.position(MouseButt::Left, FrameId::Current)?;
        let a = ctx.c2w(a);
        let b = ctx.c2w(b);
        Self::cursor_orbit(a, b, state)
    }

    pub fn right_cursor_orbit(&self, state: &GameState) -> Option<GlobalOrbit> {
        let _mouse = self.scene.mouse_if_world(&self.input)?;
        let ctx = &state.orbital_context;
        let a = self.input.position(MouseButt::Right, FrameId::Down)?;
        let b = self.input.position(MouseButt::Right, FrameId::Current)?;
        let a = ctx.c2w(a);
        let b = ctx.c2w(b);
        Self::cursor_orbit(a, b, state)
    }

    pub fn selection_region(&self, state: &GameState) -> Option<Region> {
        let ctx = &state.orbital_context;
        let mouse: &InputState = self.scene.mouse_if_world(&self.input)?;
        match state.orbital_context.selection_mode {
            CursorMode::Rect => {
                let a = mouse.world_position(MouseButt::Left, FrameId::Down, ctx)?;
                let b = mouse.world_position(MouseButt::Left, FrameId::Current, ctx)?;
                Some(Region::aabb(a, b))
            }
            CursorMode::Altitude => {
                let a = mouse.world_position(MouseButt::Left, FrameId::Down, ctx)?;
                let b = mouse.world_position(MouseButt::Left, FrameId::Current, ctx)?;
                Some(Region::altitude(a, b))
            }
            CursorMode::NearOrbit => self
                .left_cursor_orbit(state)
                .map(|GlobalOrbit(_, orbit)| Region::NearOrbit(orbit, 50.0)),
            CursorMode::Measure => None,
        }
    }

    pub fn follow_position(&self, state: &GameState) -> Option<Vec2> {
        let id = state.orbital_context.follow?;
        let lup = match id {
            ObjectId::Orbiter(id) => state.scenario.lup_orbiter(id, state.sim_time)?,
            ObjectId::Planet(id) => state.scenario.lup_planet(id, state.sim_time)?,
        };
        Some(lup.pv().pos)
    }
}
