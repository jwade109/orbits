#![allow(unused)]

use crate::mouse::{FrameId, MouseButt, MouseState};
use crate::planetary::{CursorMode, GameState};
use crate::ui::OnClick;
use bevy::log::*;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug, Clone)]
pub enum SceneType {
    OrbitalView(OrbitalScene),
    DockingView(OrbiterId),
    TelescopeView,
    MainMenu,
}

#[derive(Debug, Clone)]
pub struct OrbitalScene {
    primary: PlanetId,
}

impl OrbitalScene {
    fn new(primary: PlanetId) -> Self {
        Self { primary }
    }
}

#[derive(Debug, Clone)]
pub struct Scene {
    name: String,
    scene_type: SceneType,
    ui: Tree<OnClick>,
}

impl Scene {
    pub fn orbital(name: impl Into<String>, primary: PlanetId) -> Self {
        Scene {
            name: name.into(),
            scene_type: SceneType::OrbitalView(OrbitalScene::new(primary)),
            ui: Tree::new(),
        }
    }

    pub fn telescope() -> Self {
        Scene {
            name: "Telescope".into(),
            scene_type: SceneType::TelescopeView,
            ui: Tree::new(),
        }
    }

    pub fn docking(name: impl Into<String>, primary: OrbiterId) -> Self {
        Scene {
            name: name.into(),
            scene_type: SceneType::DockingView(primary),
            ui: Tree::new(),
        }
    }

    pub fn main_menu() -> Self {
        Scene {
            name: "Main Menu".into(),
            scene_type: SceneType::MainMenu,
            ui: Tree::new(),
        }
    }
}

impl Scene {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn kind(&self) -> &SceneType {
        &self.scene_type
    }

    pub fn current_clicked_gui_element(&self, mouse: &MouseState) -> Option<OnClick> {
        let a = mouse.position(MouseButt::Left, FrameId::Down);
        let b = mouse.position(MouseButt::Right, FrameId::Down);
        let p = a.or(b)?;
        let q = Vec2::new(p.x, mouse.viewport_bounds.span.y - p.y);
        self.ui.at(q).map(|n| n.id()).flatten().cloned()
    }

    pub fn mouse_if_world<'a>(&self, mouse: &'a MouseState) -> Option<&'a MouseState> {
        let id = self.current_clicked_gui_element(mouse)?;
        (id == OnClick::World).then(|| mouse)
    }

    pub fn orbital_view<'a>(&'a self, mouse: &'a MouseState) -> Option<OrbitalView<'a>> {
        match &self.scene_type {
            SceneType::OrbitalView(info) => Some(OrbitalView {
                info,
                mouse,
                ui: &self.ui,
                scene: &self,
            }),
            _ => None,
        }
    }

    pub fn on_mouse_event(&mut self, button: MouseButt, id: FrameId) {
        info!("{} {:?} {:?}", self.name, button, id);
    }

    pub fn ui(&self) -> &Tree<OnClick> {
        &self.ui
    }

    pub fn set_ui(&mut self, ui: Tree<OnClick>) {
        self.ui = ui;
    }

    pub fn on_load(&mut self) {
        info!("On load: {}", &self.name);
    }

    pub fn on_exit(&mut self) {
        info!("On exit: {}", &self.name);
    }
}

pub struct OrbitalView<'a> {
    info: &'a OrbitalScene,
    mouse: &'a MouseState,
    ui: &'a Tree<OnClick>,
    scene: &'a Scene,
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
}
