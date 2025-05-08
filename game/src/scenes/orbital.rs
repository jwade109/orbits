use crate::mouse::{FrameId, InputState, MouseButt};
use crate::planetary::GameState;
use crate::scenes::Scene;
use bevy::input::keyboard::KeyCode;
use enum_iterator::Sequence;
use starling::prelude::*;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Sequence)]
pub enum CursorMode {
    #[default]
    Rect,
    AddOrbit,
    NearOrbit,
    MeasuringTape,
    Protractor,
}

#[derive(Debug, Clone, Copy, Default, Sequence)]
pub enum ShowOrbitsState {
    #[default]
    None,
    Focus,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Sequence)]
pub enum DrawMode {
    #[default]
    Default,
    Constellations,
    Stability,
    Occlusion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Sequence)]
pub enum ThrottleLevel {
    High,
    #[default]
    Medium,
    Low,
}

impl ThrottleLevel {
    pub fn to_ratio(&self) -> f32 {
        match self {
            Self::High => 1.0,
            Self::Medium => 0.2,
            Self::Low => 0.01,
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct OrbitalContext {
    primary: PlanetId,
    pub selected: HashSet<OrbiterId>,
    center: Vec2,
    target_center: Vec2,
    scale: f32,
    target_scale: f32,
    pub following: Option<ObjectId>,
    piloting: Option<OrbiterId>,
    pub queued_orbits: Vec<GlobalOrbit>,
    pub cursor_mode: CursorMode,
    pub show_orbits: ShowOrbitsState,
    pub show_animations: bool,
    pub draw_mode: DrawMode,
    pub throttle: ThrottleLevel,
}

pub trait CameraProjection {
    /// World to camera transform
    fn w2c(&self, p: Vec2) -> Vec2 {
        (p - self.origin()) * self.scale()
    }

    #[allow(unused)]
    fn w2c_aabb(&self, aabb: AABB) -> AABB {
        let a = aabb.lower();
        let b = aabb.upper();
        AABB::from_arbitrary(self.w2c(a), self.w2c(b))
    }

    /// Camera to world transform
    fn c2w(&self, p: Vec2) -> Vec2 {
        p / self.scale() + self.origin()
    }

    #[allow(unused)]
    fn c2w_aabb(&self, aabb: AABB) -> AABB {
        let a = aabb.lower();
        let b = aabb.upper();
        AABB::from_arbitrary(self.c2w(a), self.c2w(b))
    }

    fn origin(&self) -> Vec2;

    fn scale(&self) -> f32;
}

pub trait Interactive {
    fn step(&mut self, input: &InputState, dt: f32);
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
            scale: 0.02,
            target_scale: 0.025,
            following: None,
            piloting: None,
            queued_orbits: Vec::new(),
            cursor_mode: CursorMode::Rect,
            show_orbits: ShowOrbitsState::Focus,
            show_animations: true,
            draw_mode: DrawMode::Default,
            throttle: ThrottleLevel::Medium,
        }
    }

    pub fn go_to(&mut self, p: Vec2) {
        self.target_center = p;
    }

    pub fn follow_position(&self, state: &GameState) -> Option<Vec2> {
        let id = self.following?;
        let lup = match id {
            ObjectId::Orbiter(id) => state.scenario.lup_orbiter(id, state.sim_time)?,
            ObjectId::Planet(id) => state.scenario.lup_planet(id, state.sim_time)?,
        };
        Some(lup.pv().pos)
    }
}

impl Interactive for OrbitalContext {
    fn step(&mut self, input: &InputState, dt: f32) {
        let speed = 16.0 * dt * 100.0;

        if input.is_pressed(KeyCode::ShiftLeft) {
            if input.is_scroll_down() {
                println!("TODO change sim speed");
            }
            if input.is_scroll_up() {
                println!("TODO change sim speed");
            }
        } else {
            if input.is_scroll_down() {
                self.target_scale /= 1.5;
            }
            if input.is_scroll_up() {
                self.target_scale *= 1.5;
            }
        }

        if input.is_pressed(KeyCode::Equal) {
            self.target_scale *= 1.03;
        }
        if input.is_pressed(KeyCode::Minus) {
            self.target_scale /= 1.03;
        }
        if input.is_pressed(KeyCode::KeyD) {
            self.target_center.x += speed / self.scale;
            self.following = None;
        }
        if input.is_pressed(KeyCode::KeyA) {
            self.target_center.x -= speed / self.scale;
            self.following = None;
        }
        if input.is_pressed(KeyCode::KeyW) {
            self.target_center.y += speed / self.scale;
            self.following = None;
        }
        if input.is_pressed(KeyCode::KeyS) {
            self.target_center.y -= speed / self.scale;
            self.following = None;
        }
        if input.is_pressed(KeyCode::KeyR) {
            self.target_center = Vec2::ZERO;
            self.target_scale = 1.0;
            self.following = None;
        }

        self.scale += (self.target_scale - self.scale) * 0.1;
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
        let input: &InputState = self.scene.mouse_if_not_on_gui(self.input, &state.ui)?;
        let a = input.position(MouseButt::Left, FrameId::Down)?;
        let b = input.position(MouseButt::Left, FrameId::Current)?;
        let a = ctx.c2w(a);
        let b = ctx.c2w(b);
        let corner = Vec2::new(a.x, b.y);
        Some((a, b, corner))
    }

    pub fn protractor(&self, state: &GameState) -> Option<(Vec2, Vec2, Option<Vec2>)> {
        let ctx = &state.orbital_context;
        let input: &InputState = self.scene.mouse_if_not_on_gui(self.input, &state.ui)?;
        let c = input.position(MouseButt::Left, FrameId::Down)?;
        let l = input.position(MouseButt::Left, FrameId::Current)?;

        let c = ctx.c2w(c);

        let (a, b) = if input.position(MouseButt::Right, FrameId::Current).is_some() {
            let r = input.position(MouseButt::Right, FrameId::Down)?;
            (ctx.c2w(r), Some(ctx.c2w(l)))
        } else {
            (ctx.c2w(l), None)
        };

        Some((c, a, b))
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
        let parent_id: PlanetId = state.scenario.relevant_body(pv.pos, state.sim_time)?;
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
        let _mouse = self.scene.mouse_if_not_on_gui(&self.input, &state.ui)?;
        let ctx = &state.orbital_context;
        let a = self.input.position(MouseButt::Left, FrameId::Down)?;
        let b = self.input.position(MouseButt::Left, FrameId::Current)?;
        let a = ctx.c2w(a);
        let b = ctx.c2w(b);
        Self::cursor_orbit(a, b, state)
    }

    pub fn selection_region(&self, state: &GameState) -> Option<Region> {
        let ctx = &state.orbital_context;
        let mouse: &InputState = self.scene.mouse_if_not_on_gui(&self.input, &state.ui)?;
        match state.orbital_context.cursor_mode {
            CursorMode::Rect => {
                let a = mouse.world_position(MouseButt::Left, FrameId::Down, ctx)?;
                let b = mouse.world_position(MouseButt::Left, FrameId::Current, ctx)?;
                Some(Region::aabb(a, b))
            }
            CursorMode::NearOrbit => self
                .left_cursor_orbit(state)
                .map(|GlobalOrbit(_, orbit)| Region::NearOrbit(orbit, 500.0)),
            _ => None,
        }
    }
}
