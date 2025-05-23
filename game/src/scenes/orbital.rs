use crate::mouse::{FrameId, InputState, MouseButt};
use crate::planetary::GameState;
use crate::scenes::{Render, StaticSpriteDescriptor, TextLabel};
use bevy::color::palettes::css::*;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use enum_iterator::Sequence;
use rfd::FileDialog;
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
    VeryLow,
}

impl ThrottleLevel {
    pub fn to_ratio(&self) -> f32 {
        match self {
            Self::High => 1.0,
            Self::Medium => 0.2,
            Self::Low => 0.01,
            Self::VeryLow => 0.002,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LowPass {
    pub value: f32,
    pub target: f32,
    /// LPF coefficient, must be in (0, 1]
    pub alpha: f32,
}

impl LowPass {
    fn step(&mut self) {
        self.value += (self.target - self.value) * self.alpha
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct OrbitalContext {
    primary: PlanetId,
    pub selected: HashSet<OrbiterId>,
    pub highlighted: HashSet<OrbiterId>,
    center: Vec2,
    target_center: Vec2,
    scale: f32,
    target_scale: f32,
    pub following: Option<ObjectId>,
    pub queued_orbits: Vec<GlobalOrbit>,
    pub cursor_mode: CursorMode,
    pub show_orbits: ShowOrbitsState,
    pub show_animations: bool,
    pub draw_mode: DrawMode,
    pub throttle: ThrottleLevel,

    pub piloting: Option<OrbiterId>,
    pub targeting: Option<OrbiterId>,
    pub rendezvous_scope_radius: LowPass,
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
            highlighted: HashSet::new(),
            center: Vec2::ZERO,
            target_center: Vec2::ZERO,
            scale: 0.02,
            target_scale: 0.025,
            following: None,
            queued_orbits: Vec::new(),
            cursor_mode: CursorMode::Rect,
            show_orbits: ShowOrbitsState::Focus,
            show_animations: true,
            draw_mode: DrawMode::Default,
            throttle: ThrottleLevel::Medium,
            piloting: None,
            targeting: None,
            rendezvous_scope_radius: LowPass {
                value: 50.0,
                target: 50.0,
                alpha: 0.1,
            },
        }
    }

    pub fn go_to(&mut self, p: Vec2) {
        self.target_center = p;
        self.center = p;
    }

    pub fn follow_position(&self, state: &GameState) -> Option<Vec2> {
        let id = self.following?;
        let lup = match id {
            ObjectId::Orbiter(id) => state.scenario.lup_orbiter(id, state.sim_time)?,
            ObjectId::Planet(id) => state.scenario.lup_planet(id, state.sim_time)?,
        };
        Some(lup.pv().pos)
    }

    pub fn toggle_track(&mut self, id: OrbiterId) {
        if self.selected.contains(&id) {
            self.selected.retain(|e| *e != id);
        } else {
            self.selected.insert(id);
        }
    }

    pub fn save_to_file(state: &mut GameState) -> Option<()> {
        let orbiters: Vec<_> = state
            .orbital_context
            .selected
            .iter()
            .filter_map(|id| {
                state
                    .scenario
                    .lup_orbiter(*id, state.sim_time)
                    .map(|lup| lup.orbiter())
                    .flatten()
            })
            .collect();

        let dir = FileDialog::new().set_directory("/").pick_folder()?;

        for orbiter in orbiters {
            let mut file = dir.clone();
            file.push(format!("{}.strl", orbiter.id()));
            info!("Saving {}", file.display());
            starling::file_export::to_strl_file(orbiter, &file).ok()?;
        }

        Some(())
    }

    pub fn highlighted(state: &GameState) -> HashSet<OrbiterId> {
        if let Some(a) = state.selection_region() {
            state
                .scenario
                .orbiter_ids()
                .into_iter()
                .filter_map(|id| {
                    let pv = state.scenario.lup_orbiter(id, state.sim_time)?.pv();
                    a.contains(pv.pos).then(|| id)
                })
                .collect()
        } else {
            HashSet::new()
        }
    }

    pub fn measuring_tape(state: &GameState) -> Option<(Vec2, Vec2, Vec2)> {
        if state.is_currently_left_clicked_on_ui() {
            return None;
        }
        let ctx = &state.orbital_context;
        let a = state.input.position(MouseButt::Left, FrameId::Down)?;
        let b = state.input.position(MouseButt::Left, FrameId::Current)?;
        let a = ctx.c2w(a);
        let b = ctx.c2w(b);
        let corner = Vec2::new(a.x, b.y);
        Some((a, b, corner))
    }

    pub fn protractor(state: &GameState) -> Option<(Vec2, Vec2, Option<Vec2>)> {
        if state.is_currently_left_clicked_on_ui() {
            return None;
        }
        let ctx = &state.orbital_context;
        let c = state.input.position(MouseButt::Left, FrameId::Down)?;
        let l = state.input.position(MouseButt::Left, FrameId::Current)?;

        let c = ctx.c2w(c);

        let (a, b) = if state
            .input
            .position(MouseButt::Right, FrameId::Current)
            .is_some()
        {
            let r = state.input.position(MouseButt::Right, FrameId::Down)?;
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

    pub fn left_cursor_orbit(state: &GameState) -> Option<GlobalOrbit> {
        if state.is_currently_left_clicked_on_ui() {
            return None;
        }
        let ctx = &state.orbital_context;
        let a = state.input.position(MouseButt::Left, FrameId::Down)?;
        let b = state.input.position(MouseButt::Left, FrameId::Current)?;
        let a = ctx.c2w(a);
        let b = ctx.c2w(b);
        Self::cursor_orbit(a, b, state)
    }

    pub fn selection_region(state: &GameState) -> Option<Region> {
        if state.is_currently_left_clicked_on_ui() {
            return None;
        }
        let ctx = &state.orbital_context;
        match state.orbital_context.cursor_mode {
            CursorMode::Rect => {
                let a = state
                    .input
                    .world_position(MouseButt::Left, FrameId::Down, ctx)?;
                let b = state
                    .input
                    .world_position(MouseButt::Left, FrameId::Current, ctx)?;
                Some(Region::aabb(a, b))
            }
            CursorMode::NearOrbit => Self::left_cursor_orbit(state)
                .map(|GlobalOrbit(_, orbit)| Region::NearOrbit(orbit, 500.0)),
            _ => None,
        }
    }
}

pub fn get_orbital_object_mouseover_label(state: &GameState) -> Option<TextLabel> {
    let cursor = match state.input.position(MouseButt::Hover, FrameId::Current) {
        Some(p) => p,
        None => return None,
    };

    let cursor_world = state.orbital_context.c2w(cursor);

    for id in state.scenario.ids() {
        let lup = match state.scenario.lup(id, state.sim_time) {
            Some(lup) => lup,
            None => continue,
        };
        let pw = lup.pv().pos;
        let pc = state.orbital_context.w2c(pw);

        let (passes, label, pos) = if let Some((name, body)) = lup.named_body() {
            // distance based on world space
            let d = pw.distance(cursor_world);
            let p = state.orbital_context.w2c(pw + Vec2::Y * body.radius);
            (d < body.radius, name.to_uppercase(), p + Vec2::Y * 30.0)
        } else {
            let orb_id = id.orbiter().unwrap();
            let is_rpo = state.rpos.contains_key(&orb_id);
            let is_controllable = state
                .orbital_vehicles
                .get(&orb_id)
                .map(|v| v.is_controllable())
                .unwrap_or(false);

            // distance based on pixel space
            let d = pc.distance(cursor);
            (
                d < 25.0,
                if is_rpo {
                    format!("RPO {}", orb_id)
                } else if is_controllable {
                    format!("VEH {}", orb_id)
                } else {
                    format!("AST {}", orb_id)
                },
                pc + Vec2::Y * 40.0,
            )
        };
        if passes {
            return Some(TextLabel::new(label, pos, 1.0));
        }
    }
    None
}

impl Render for OrbitalContext {
    fn text_labels(state: &GameState) -> Option<Vec<TextLabel>> {
        let mut text_labels: Vec<TextLabel> = get_orbital_object_mouseover_label(state)
            .into_iter()
            .collect();

        if state.paused {
            let s = "PAUSED".to_string();
            let c = Vec2::Y * (60.0 - state.input.screen_bounds.span.y * 0.5);
            text_labels.push(TextLabel::new(s, c, 1.0));
        }

        {
            let date = state.sim_time.to_date();
            let s = format!(
                "Y{} W{} D{} {:02}:{:02}:{:02}.{:03}",
                date.year + 1,
                date.week + 1,
                date.day + 1,
                date.hour,
                date.min,
                date.sec,
                date.milli,
            );
            let c = Vec2::Y * (20.0 - state.input.screen_bounds.span.y * 0.5);
            text_labels.push(TextLabel::new(s, c, 1.0));
        }

        if let Some((m1, m2, corner)) = state.measuring_tape() {
            for (a, b) in [(m1, m2), (m1, corner), (m2, corner)] {
                let middle = (a + b) / 2.0;
                let middle = state.orbital_context.w2c(middle);
                let d = format!("{:0.1} km", a.distance(b));
                text_labels.push(TextLabel::new(d, middle, 1.0));
            }
        }

        if let Some((c, a, b)) = state.protractor() {
            for (a, b) in [(c, Some(a)), (c, b)] {
                if let Some(b) = b {
                    let middle = (a + b) / 2.0;
                    let middle = state.orbital_context.w2c(middle);
                    let d = format!("{:0.1} km", a.distance(b));
                    text_labels.push(TextLabel::new(d, middle, 1.0));
                }
            }
            if let Some(b) = b {
                let da = a - c;
                let db = b - c;
                let angle = da.angle_to(db);
                let d = c + rotate(da * 0.75, angle / 2.0);
                let t = format!("{:0.1} deg", angle.to_degrees().abs());
                let d = state.orbital_context.w2c(d);
                text_labels.push(TextLabel::new(t, d, 1.0));
            }
        }

        Some(text_labels)
    }

    fn sprites(state: &GameState) -> Option<Vec<StaticSpriteDescriptor>> {
        const EXPECTED_PLANET_SPRITE_SIZE: u32 = 1000;
        const PLANET_Z_INDEX: f32 = 5.0;

        let ctx = &state.orbital_context;
        Some(
            state
                .scenario
                .planet_ids()
                .into_iter()
                .filter_map(|id| {
                    let lup = state.scenario.lup_planet(id, state.sim_time)?;
                    let pos = lup.pv().pos;
                    let (name, body) = lup.named_body()?;
                    let path = format!("embedded://game/../assets/{}.png", name);
                    Some(StaticSpriteDescriptor::new(
                        ctx.w2c(pos),
                        0.0,
                        path,
                        ctx.scale() * 2.0 * body.radius / EXPECTED_PLANET_SPRITE_SIZE as f32,
                        PLANET_Z_INDEX,
                    ))
                })
                .collect(),
        )
    }

    fn background_color(state: &GameState) -> bevy::color::Srgba {
        match state.orbital_context.draw_mode {
            DrawMode::Default => BLACK,
            DrawMode::Constellations => GRAY.with_luminance(0.1),
            DrawMode::Stability => GRAY.with_luminance(0.13),
            DrawMode::Occlusion => GRAY.with_luminance(0.04),
        }
    }

    fn draw_gizmos(gizmos: &mut Gizmos, state: &GameState) -> Option<()> {
        crate::drawing::draw_orbital_view(gizmos, state);
        Some(())
    }
}

impl Interactive for OrbitalContext {
    fn step(&mut self, input: &InputState, dt: f32) {
        if input.just_pressed(KeyCode::BracketLeft) {
            self.rendezvous_scope_radius.target /= 1.5;
        }
        if input.just_pressed(KeyCode::BracketRight) {
            self.rendezvous_scope_radius.target *= 1.5;
        }

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
        self.rendezvous_scope_radius.step();
    }
}
