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
    camera: LinearCameraController,
    primary: EntityId,
    pub selected: HashSet<EntityId>,
    pub highlighted: HashSet<EntityId>,
    pub following: Option<ObjectId>,
    pub queued_orbits: Vec<GlobalOrbit>,
    pub cursor_mode: CursorMode,
    pub show_orbits: ShowOrbitsState,
    pub show_animations: bool,
    pub draw_mode: DrawMode,
    pub throttle: ThrottleLevel,

    pub piloting: Option<EntityId>,
    pub targeting: Option<EntityId>,
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

impl CameraProjection for OrbitalContext {
    fn origin(&self) -> Vec2 {
        self.camera.origin()
    }

    fn scale(&self) -> f32 {
        self.camera.scale()
    }
}

pub fn relevant_body(planets: &PlanetarySystem, pos: Vec2, stamp: Nanotime) -> Option<EntityId> {
    let results = planets
        .planet_ids()
        .into_iter()
        .filter_map(|id| {
            let (body, pv, _, _) = planets.lookup(id, stamp)?;
            let p = pv.pos_f32();
            let d = pos.distance(p);
            (d <= body.soi).then(|| (d, id))
        })
        .collect::<Vec<_>>();
    results
        .iter()
        .min_by(|(d1, _), (d2, _)| d1.total_cmp(d2))
        .map(|(_, id)| *id)
}

impl OrbitalContext {
    pub fn new(primary: EntityId) -> Self {
        Self {
            camera: LinearCameraController::new(Vec2::ZERO, 0.02, 600.0),
            primary,
            selected: HashSet::new(),
            highlighted: HashSet::new(),
            following: None,
            queued_orbits: Vec::new(),
            cursor_mode: CursorMode::Rect,
            show_orbits: ShowOrbitsState::Focus,
            show_animations: true,
            draw_mode: DrawMode::Default,
            throttle: ThrottleLevel(ThrottleLevel::MAX / 2),
            piloting: None,
            targeting: None,
            rendezvous_scope_radius: LowPass {
                value: 50.0,
                target: 50.0,
                alpha: 0.1,
            },
        }
    }

    #[deprecated]
    pub fn follow_position(&self, state: &GameState) -> Option<Vec2> {
        let id = self.following?;
        let lup = match id {
            ObjectId::Orbiter(id) => state.universe.lup_orbiter(id, state.universe.stamp())?,
            ObjectId::Planet(id) => state.universe.lup_planet(id, state.universe.stamp())?,
        };
        Some(lup.pv().pos_f32())
    }

    pub fn toggle_track(&mut self, id: EntityId) {
        if self.selected.contains(&id) {
            self.selected.retain(|e| *e != id);
        } else {
            self.selected.insert(id);
        }
    }

    pub fn highlighted(state: &GameState) -> HashSet<EntityId> {
        if let Some(a) = state.selection_region() {
            state
                .universe
                .orbiter_ids()
                .into_iter()
                .filter_map(|id| {
                    let pv = state.universe.lup_orbiter(id, state.universe.stamp())?.pv();
                    a.contains(pv.pos_f32()).then(|| id)
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

    pub fn landing_site_position(state: &GameState, pid: EntityId, angle: f32) -> Option<Vec2> {
        let lup = state.universe.lup_planet(pid, state.universe.stamp())?;
        let body = lup.body()?;
        let center = lup.pv().pos_f32();
        Some(center + rotate(Vec2::X * body.radius, angle))
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

        let wrt_id = relevant_body(&state.universe.planets, p1, state.universe.stamp())?;
        let parent = state.universe.lup_planet(wrt_id, state.universe.stamp())?;

        let r = p1.distance(parent.pv().pos_f32());
        let v = (parent.body()?.mu() / r).sqrt();

        Some(PV::from_f64(p1, (p2 - p1) * v / r))
    }

    pub fn cursor_orbit(p1: Vec2, p2: Vec2, state: &GameState) -> Option<GlobalOrbit> {
        let pv = Self::cursor_pv(p1, p2, &state)?;
        let parent_id: EntityId = relevant_body(
            &state.universe.planets,
            pv.pos_f32(),
            state.universe.stamp(),
        )?;
        let parent = state
            .universe
            .lup_planet(parent_id, state.universe.stamp())?;
        let parent_pv = parent.pv();
        let pv = pv - PV::pos(parent_pv.pos_f32());
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

    pub fn on_game_tick(&mut self) {
        self.camera.on_game_tick();
        self.rendezvous_scope_radius.step();
    }

    pub fn handle_input(&mut self, input: &InputState) {
        self.camera.handle_input(input);
    }
}

pub fn all_orbital_ids(state: &GameState) -> impl Iterator<Item = ObjectId> + use<'_> {
    state
        .universe
        .orbiter_ids()
        .map(|id| ObjectId::Orbiter(id))
        .chain(
            state
                .universe
                .planets
                .planet_ids()
                .into_iter()
                .map(|id| ObjectId::Planet(id)),
        )
}

pub const LANDING_SITE_MOUSEOVER_DISTANCE: f32 = 50.0;

pub fn get_landing_site_labels(state: &GameState) -> Vec<TextLabel> {
    let ctx = &state.orbital_context;

    let cursor = match state.input.position(MouseButt::Hover, FrameId::Current) {
        Some(p) => p,
        None => return Vec::new(),
    };

    let mut ret = Vec::new();
    for (pid, sites) in &state.universe.landing_sites {
        for (angle, name, _) in sites {
            let pos = OrbitalContext::landing_site_position(state, *pid, *angle);
            if let Some(pos) = pos {
                let pos = ctx.w2c(pos);
                let offset = rotate(Vec2::X, *angle) * LANDING_SITE_MOUSEOVER_DISTANCE;
                if pos.distance(cursor) < LANDING_SITE_MOUSEOVER_DISTANCE {
                    let label = TextLabel::new(name.clone(), pos + offset, 0.7);
                    ret.push(label);
                }
            }
        }
    }
    ret
}

pub fn get_orbital_object_mouseover_labels(state: &GameState) -> Vec<TextLabel> {
    let mut ret = Vec::new();

    let cursor = match state.input.position(MouseButt::Hover, FrameId::Current) {
        Some(p) => p,
        None => return Vec::new(),
    };

    let cursor_world = state.orbital_context.c2w(cursor);

    for id in all_orbital_ids(state) {
        let lup = match id {
            ObjectId::Orbiter(id) => state.universe.lup_orbiter(id, state.universe.stamp()),
            ObjectId::Planet(id) => state.universe.lup_planet(id, state.universe.stamp()),
        };
        let lup = match lup {
            Some(lup) => lup,
            None => continue,
        };
        let pw = lup.pv().pos_f32();
        let pc = state.orbital_context.w2c(pw);

        let (passes, label, pos) = if let Some((name, body)) = lup.named_body() {
            // distance based on world space
            let d = pw.distance(cursor_world);
            let p = state.orbital_context.w2c(pw + Vec2::Y * body.radius);
            (d < body.radius, name.to_uppercase(), p + Vec2::Y * 30.0)
        } else {
            let orb_id = id.orbiter().unwrap();
            let vehicle = state.universe.orbital_vehicles.get(&orb_id);
            let code = vehicle.map(|(_, _, v)| v.name()).unwrap_or(&"UFO");

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

#[allow(unused)]
fn get_indicators(state: &GameState) -> Option<Vec<TextLabel>> {
    let piloting = state.piloting()?;
    let (_, _, vehicle) = state.universe.orbital_vehicles.get(&piloting)?;
    let origin = Vec2::new(state.input.screen_bounds.span.x * 0.5 - 100.0, 0.0);

    Some(
        vehicle
            .thrusters()
            .enumerate()
            .map(|(i, (t, d))| {
                let text = format!("{} / {} {:0.1}", i, t.model_name(), d.throttle() * 100.0);
                let pos = origin + Vec2::Y * 26.0 * i as f32;
                let color = if d.is_thrusting(t) {
                    RED.with_alpha(0.8)
                } else {
                    WHITE.with_alpha(0.6)
                };
                TextLabel::new(text, pos, 0.7).with_color(color)
            })
            .collect(),
    )
}

pub fn date_info(state: &GameState) -> String {
    let date = state.universe.stamp().to_date();
    format!(
        "{} (x{}/{} {} us)",
        date,
        state.actual_universe_ticks_per_game_tick,
        state.universe_ticks_per_game_tick,
        state.exec_time.as_micros()
    )
}

fn text_labels(state: &GameState) -> Vec<TextLabel> {
    let mut text_labels: Vec<TextLabel> = get_orbital_object_mouseover_labels(state);

    text_labels.extend(get_indicators(state).unwrap_or(vec![]));
    text_labels.extend(get_landing_site_labels(state));

    (|| {
        let id = state.piloting()?;
        let pv = state.universe.lup_orbiter(id, state.universe.stamp())?.pv();
        let text = format!("{:0.1} m/s", pv.vel.length() * 1000.0);
        let c = Vec2::Y * (100.0 - state.input.screen_bounds.span.y * 0.5);
        text_labels.push(TextLabel::new(text, c, 1.0));
        Some(())
    })();

    if state.paused {
        let s = "PAUSED".to_string();
        let c = Vec2::Y * (60.0 - state.input.screen_bounds.span.y * 0.5);
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

        let buttons: String = state
            .input
            .pressed()
            .into_iter()
            .map(|k| format!("{:?}\n", k))
            .collect();

        canvas.text(buttons, -Vec2::X * 200.0, 0.8);

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

        if let Some(lup) = relevant_body(
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

        if !state.controllers.is_empty() {
            sidebar.add_child(Node::hline());
            let s = format!("{} autopiloting", state.controllers.len());
            let id = OnClick::AutopilotingCount;
            sidebar.add_child(
                Node::button(s, id, Size::Grow, state.settings.ui_button_height).enabled(false),
            );

            let ids = state.controllers.iter().map(|c| c.target()).collect();
            orbiter_list(state, &mut sidebar, 16, ids);
        }

        let mut inner_topbar = sim_time_toolbar(state);

        if let Some(id) = state.orbital_context.following {
            let s = format!("Following {}", id);
            let id = OnClick::Nullopt;
            let n = Node::button(s, id, 300, state.settings.ui_button_height).enabled(false);
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

        let throttle_controls = throttle_controls(state);
        let favorites = favorites_menu(state);

        let world = Node::grow()
            .down()
            .invisible()
            .tight()
            .with_child(
                Node::grow()
                    .down()
                    .invisible()
                    .with_child(inner_topbar)
                    .with_child(favorites)
                    .with_child(throttle_controls),
            )
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
