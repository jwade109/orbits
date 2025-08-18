use crate::camera_controller::LinearCameraController;
use crate::canvas::Canvas;
use crate::game::GameState;
use crate::input::{FrameId, InputState, MouseButt};
use crate::onclick::OnClick;
use crate::scenes::{Render, TextLabel};
use crate::ui::*;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use enum_iterator::all;
use enum_iterator::Sequence;
use layout::layout::{Node, Size, Tree};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThrottleLevel(pub u32);

impl ThrottleLevel {
    pub const MAX: u32 = 10;

    pub fn to_ratio(&self) -> f32 {
        self.0 as f32 / Self::MAX as f32
    }

    pub fn increment(&mut self, d: i32) {
        let v = self.0 as i32 + d;
        self.0 = v.clamp(0, Self::MAX as i32) as u32;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LowPass {
    pub value: f64,
    pub target: f64,
    /// LPF coefficient, must be in (0, 1]
    pub alpha: f64,
}

impl LowPass {
    fn step(&mut self) {
        self.value += (self.target - self.value) * self.alpha
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct OrbitalContext {
    camera: LinearCameraController,
    primary: EntityId,
    pub selected: HashSet<EntityId>,
    pub following: Option<ObjectId>,
    pub queued_orbits: Vec<GlobalOrbit>,
    pub cursor_mode: CursorMode,
    pub show_orbits: ShowOrbitsState,
    pub show_animations: bool,
    pub draw_mode: DrawMode,
    pub throttle: ThrottleLevel,

    pub piloting: Option<EntityId>,

    pub mouse_down_world_pos: Option<DVec2>,
    pub selection_bounds: Option<AABB>,
}

pub trait CameraProjection {
    /// World to camera transform
    fn w2c(&self, p: DVec2) -> Vec2 {
        graphics_cast((p - self.origin()) * self.scale())
    }

    fn w2c_aabb(&self, aabb: AABB) -> AABB {
        let a = aabb.lower().as_dvec2();
        let b = aabb.upper().as_dvec2();
        AABB::from_arbitrary(self.w2c(a), self.w2c(b))
    }

    /// Camera to world transform
    fn c2w(&self, p: Vec2) -> DVec2 {
        p.as_dvec2() / self.scale() + self.origin()
    }

    #[allow(unused)]
    fn c2w_aabb(&self, aabb: AABB) -> AABB {
        let a = aabb.lower();
        let b = aabb.upper();
        AABB::from_arbitrary(
            aabb_stopgap_cast(self.c2w(a)),
            aabb_stopgap_cast(self.c2w(b)),
        )
    }

    fn origin(&self) -> DVec2;

    fn scale(&self) -> f64;
}

impl CameraProjection for OrbitalContext {
    fn origin(&self) -> DVec2 {
        self.camera.origin()
    }

    fn scale(&self) -> f64 {
        self.camera.scale()
    }
}

impl OrbitalContext {
    pub fn new(primary: EntityId) -> Self {
        Self {
            camera: LinearCameraController::new(DVec2::ZERO, 0.00002, 600.0),
            primary,
            selected: HashSet::new(),
            following: None,
            queued_orbits: Vec::new(),
            cursor_mode: CursorMode::Rect,
            show_orbits: ShowOrbitsState::Focus,
            show_animations: true,
            draw_mode: DrawMode::Default,
            throttle: ThrottleLevel(ThrottleLevel::MAX / 2),
            piloting: None,
            mouse_down_world_pos: None,
            selection_bounds: None,
        }
    }

    pub fn toggle_track(&mut self, id: EntityId) {
        if self.selected.contains(&id) {
            self.selected.retain(|e| *e != id);
        } else {
            self.selected.insert(id);
        }
    }

    pub fn highlighted(state: &GameState) -> HashSet<EntityId> {
        if let Some(a) = state.orbital_context.selection_bounds {
            orbiters_within_bounds(&state.universe, a).collect()
        } else {
            HashSet::new()
        }
    }

    pub fn measuring_tape(state: &GameState) -> Option<(DVec2, DVec2, DVec2)> {
        if state.is_currently_left_clicked_on_ui() {
            return None;
        }
        let ctx = &state.orbital_context;
        let a = state.input.position(MouseButt::Left, FrameId::Down)?;
        let b = state.input.position(MouseButt::Left, FrameId::Current)?;
        let a = ctx.c2w(a);
        let b = ctx.c2w(b);
        let corner = DVec2::new(a.x, b.y);
        Some((a, b, corner))
    }

    pub fn protractor(state: &GameState) -> Option<(DVec2, DVec2, Option<DVec2>)> {
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

    pub fn cursor_pv(p1: DVec2, p2: DVec2, state: &GameState) -> Option<PV> {
        if p1.distance(p2) < 20.0 {
            return None;
        }

        let wrt_id = nearest_relevant_body(&state.universe.planets, p1, state.universe.stamp())?;
        let parent = state.universe.lup_planet(wrt_id, state.universe.stamp())?;

        let r = p1.distance(parent.pv().pos);
        let v = (parent.body()?.mu() / r).sqrt();

        Some(PV::from_f64(p1, (p2 - p1) * v / r))
    }

    pub fn cursor_orbit(p1: DVec2, p2: DVec2, state: &GameState) -> Option<GlobalOrbit> {
        let pv = Self::cursor_pv(p1, p2, &state)?;
        let parent_id: EntityId =
            nearest_relevant_body(&state.universe.planets, pv.pos, state.universe.stamp())?;
        let parent = state
            .universe
            .lup_planet(parent_id, state.universe.stamp())?;
        let parent_pv = parent.pv();
        let pv = pv - PV::pos(parent_pv.pos);
        let body = parent.body()?;
        Some(GlobalOrbit(
            parent_id,
            SparseOrbit::from_pv(pv, body, state.universe.stamp())?,
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

    pub fn on_game_tick(&mut self, universe: &Universe) {
        if let Some(follow) = self.following {
            if let Some(pv) = universe.pv(follow.as_eid()) {
                self.camera.follow(pv.pos);
            }
        }

        self.camera.on_game_tick();

        let mut track_list = self.selected.clone();
        track_list.retain(|o| universe.surface_vehicles.contains_key(o));
        self.selected = track_list;
    }

    pub fn on_render_tick(&mut self, on_ui: bool, input: &InputState, universe: &mut Universe) {
        self.camera.handle_input(input);

        if input.just_pressed(KeyCode::KeyN) {
            if let Some(id) = self.piloting {
                if let Some(sv) = universe.surface_vehicles.get_mut(&id) {
                    sv.controller.go_to_next_mode();
                }
            }
        }

        if on_ui {
            return;
        }

        if let Some(p) = input.on_frame(MouseButt::Right, FrameId::Down) {
            let w = self.c2w(p);
            if let Some(ObjectId::Orbiter(id)) = nearest_orbiter_or_planet(universe, w) {
                self.piloting = Some(id);
            }
        }

        if let Some(p) = input.double_click() {
            let w = self.c2w(p);
            if let Some(id) = nearest_orbiter_or_planet(universe, w) {
                self.following = Some(id);
            }
        }

        if self.mouse_down_world_pos.is_none() {
            if let Some(p) = input.on_frame(MouseButt::Left, FrameId::Down) {
                self.mouse_down_world_pos = Some(self.c2w(p));
            }
        }

        if input.on_frame(MouseButt::Left, FrameId::Up).is_some() {
            self.mouse_down_world_pos = None;

            if let Some(bounds) = self.selection_bounds {
                self.selected = orbiters_within_bounds(universe, bounds).collect();
            }

            self.selection_bounds = None;
        }

        self.selection_bounds = self
            .mouse_down_world_pos
            .zip(input.position(MouseButt::Left, FrameId::Current))
            .map(|(p, q)| {
                let q = self.c2w(q);
                AABB::from_arbitrary(aabb_stopgap_cast(p), aabb_stopgap_cast(q))
            });

        if input.is_pressed(KeyCode::KeyW)
            || input.is_pressed(KeyCode::KeyA)
            || input.is_pressed(KeyCode::KeyS)
            || input.is_pressed(KeyCode::KeyD)
        {
            self.following = None;
        }
    }
}

pub fn get_orbital_object_mouseover_labels(state: &GameState) -> Vec<TextLabel> {
    let mut ret = Vec::new();

    let cursor = match state.input.position(MouseButt::Hover, FrameId::Current) {
        Some(p) => p,
        None => return Vec::new(),
    };

    let cursor_world = state.orbital_context.c2w(cursor);

    for id in all_orbital_ids(&state.universe) {
        let lup = match id {
            ObjectId::Orbiter(id) => state.universe.lup_orbiter(id, state.universe.stamp()),
            ObjectId::Planet(id) => state.universe.lup_planet(id, state.universe.stamp()),
        };
        let lup = match lup {
            Some(lup) => lup,
            None => continue,
        };
        let pw = lup.pv().pos;
        let pc = state.orbital_context.w2c(pw);

        let (passes, label, pos) = if let Some((name, body)) = lup.named_body() {
            // distance based on world space
            let d = pw.distance(cursor_world);
            let p = state.orbital_context.w2c(pw + DVec2::Y * body.radius);
            (d < body.radius, name.to_uppercase(), p + Vec2::Y * 30.0)
        } else {
            let orb_id = id.as_orbiter().unwrap();
            let vehicle = state.universe.surface_vehicles.get(&orb_id);
            let code = vehicle
                .map(|ov| ov.vehicle().title())
                .unwrap_or("UFO".to_string());

            // distance based on pixel space
            let d = pc.distance(cursor);
            (
                d < 25.0,
                format!("{} {}", code, orb_id),
                pc + Vec2::Y * 40.0,
            )
        };
        if passes {
            ret.push(TextLabel::new(label, pos, 1.0));
            if ret.len() > 6 {
                return ret;
            }
        }
    }
    ret
}

pub fn date_info(state: &GameState) -> String {
    let date = state.universe.stamp().to_date();
    format!(
        "{}({}) {} (x{}/{} {} us)",
        if state.paused { "[PAUSED] " } else { "" },
        if state.using_batch_mode { "B" } else { "S" },
        date,
        state.actual_universe_ticks_per_game_tick,
        state.universe_ticks_per_game_tick.as_ticks(),
        state.exec_time.as_micros()
    )
}

fn text_labels(state: &GameState) -> Vec<TextLabel> {
    let mut text_labels: Vec<TextLabel> = get_orbital_object_mouseover_labels(state);

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
            let d = c + rotate_f64(da * 0.75, angle / 2.0);
            let t = format!("{:0.1} deg", angle.to_degrees().abs());
            let d = state.orbital_context.w2c(d);
            text_labels.push(TextLabel::new(t, d, 1.0));
        }
    }

    text_labels
}

impl Render for OrbitalContext {
    fn background_color(state: &GameState) -> bevy::color::Srgba {
        match state.orbital_context.draw_mode {
            DrawMode::Default => BLACK,
            DrawMode::Constellations => GRAY.with_luminance(0.1),
            DrawMode::Stability => GRAY.with_luminance(0.13),
            DrawMode::Occlusion => GRAY.with_luminance(0.04),
        }
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        crate::drawing::draw_orbital_view(canvas, state);

        for label in text_labels(state) {
            canvas.label(label);
        }

        Some(())
    }

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        let vb = state.input.screen_bounds;
        if vb.span.x == 0.0 || vb.span.y == 0.0 {
            return Some(Tree::new());
        }

        let mut sidebar = Node::column(300).with_color(UI_BACKGROUND_COLOR);

        let body_color_lup: std::collections::HashMap<&'static str, Srgba> =
            std::collections::HashMap::from([("Earth", BLUE), ("Luna", GRAY), ("Asteroid", BROWN)]);

        if let Some(lup) = nearest_relevant_body(
            &state.universe.planets,
            state.orbital_context.origin(),
            state.universe.stamp(),
        )
        .map(|id| state.universe.lup_planet(id, state.universe.stamp()))
        .flatten()
        {
            if let Some((s, _)) = lup.named_body() {
                let color: Srgba = body_color_lup
                    .get(s.as_str())
                    .unwrap_or(&Srgba::from(crate::sprites::hashable_to_color(s)))
                    .with_luminance(0.2)
                    .with_alpha(0.9);
                sidebar.add_child(
                    Node::button(
                        s,
                        OnClick::CurrentBody(lup.id()),
                        Size::Grow,
                        state.settings.ui_button_height,
                    )
                    .with_color(color.to_f32_array()),
                );
            }
        }

        sidebar.add_child(Node::button(
            format!("Visual: {:?}", state.orbital_context.draw_mode),
            OnClick::ToggleDrawMode,
            Size::Grow,
            state.settings.ui_button_height,
        ));

        sidebar.add_child(
            Node::button(
                "Clear Orbits",
                OnClick::ClearOrbits,
                Size::Grow,
                state.settings.ui_button_height,
            )
            .enabled(!state.orbital_context.queued_orbits.is_empty()),
        );

        sidebar.add_child(
            Node::button(
                "Commit Mission",
                OnClick::CommitMission,
                Size::Grow,
                state.settings.ui_button_height,
            )
            .enabled(state.current_orbit().is_some() && !state.orbital_context.selected.is_empty()),
        );

        sidebar.add_child(Node::hline());

        sidebar.add_children(all::<CursorMode>().map(|c| {
            let s = format!("{:?}", c);
            let id = OnClick::CursorMode(c);
            Node::button(s, id, Size::Grow, state.settings.ui_button_height)
                .enabled(c != state.orbital_context.cursor_mode)
        }));

        if !state.universe.constellations.is_empty() {
            sidebar.add_child(Node::hline());
        }

        for gid in state.universe.unique_groups() {
            let color: Srgba = crate::sprites::hashable_to_color(&gid)
                .with_luminance(0.3)
                .into();
            let s = format!("{}", gid);
            let id = OnClick::Group(gid.clone());
            let button = Node::button(s, id, Size::Grow, state.settings.ui_button_height)
                .with_color(color.to_f32_array());
            sidebar.add_child(delete_wrapper(
                OnClick::DisbandGroup(gid.clone()),
                button,
                state.settings.ui_button_height as f32,
            ));
        }

        sidebar.add_child(Node::hline());

        sidebar.add_child(piloting_buttons(state, Size::Grow));

        sidebar.add_child(selected_button(state, Size::Grow));

        if !state.orbital_context.selected.is_empty() {
            orbiter_list(
                state,
                &mut sidebar,
                32,
                state.orbital_context.selected.iter().cloned().collect(),
            );
            sidebar.add_child(Node::button(
                "Create Group",
                OnClick::CreateGroup,
                Size::Grow,
                state.settings.ui_button_height,
            ));
        }

        let mut inner_topbar = Node::fit().with_color(UI_BACKGROUND_COLOR);

        if let Some(id) = state.orbital_context.following {
            let s = format!("Following {}", id);
            let id = OnClick::Nullopt;
            let n = Node::button(s, id, 400, state.settings.ui_button_height).enabled(false);
            inner_topbar.add_child(n);
        }

        for (i, orbit) in state.orbital_context.queued_orbits.iter().enumerate() {
            let orbit_button = {
                let s = format!("{}", orbit);
                let id = OnClick::GlobalOrbit(i);
                Node::button(s, id, 400, state.settings.ui_button_height)
            };

            inner_topbar.add_child(delete_wrapper(
                OnClick::DeleteOrbit(i),
                orbit_button,
                state.settings.ui_button_height,
            ));
        }

        let notif_bar = notification_bar(state, Size::Fixed(900.0));

        let world = Node::grow()
            .down()
            .invisible()
            .tight()
            .with_child(Node::grow().down().invisible().with_child(inner_topbar))
            .with_child(
                Node::grow()
                    .tight()
                    .down()
                    .invisible()
                    .with_child(Node::grow().invisible())
                    .with_child(notif_bar),
            );

        let root = Node::new(vb.span.x, vb.span.y)
            .down()
            .tight()
            .invisible()
            .with_child(top_bar(state))
            .with_child(
                Node::grow()
                    .tight()
                    .invisible()
                    .with_child(sidebar)
                    .with_child(world),
            );

        Some(Tree::new().with_layout(root, Vec2::ZERO))
    }
}
