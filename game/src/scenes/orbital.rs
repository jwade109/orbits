use crate::camera_controller::*;
use crate::canvas::Canvas;
use crate::game::GameState;
use crate::input::{FrameId, InputState, MouseButt};
use crate::onclick::OnClick;
use crate::scenes::{Render, TextLabel};
use crate::sounds::EnvironmentSounds;
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

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct OrbitalContext {
    pub camera: LinearCameraController,
    primary: EntityId,
    pub selected: HashSet<EntityId>,
    pub following: Option<EntityId>,
    pub queued_orbits: Vec<GlobalOrbit>,
    pub cursor_mode: CursorMode,
    pub show_orbits: ShowOrbitsState,
    pub show_animations: bool,
    pub draw_mode: DrawMode,
    pub piloting: Option<EntityId>,
    pub hovered_entity: Option<EntityId>,
}

impl CameraProjection for OrbitalContext {
    fn origin(&self) -> DVec2 {
        self.camera.origin()
    }

    fn scale(&self) -> f64 {
        self.camera.scale()
    }

    fn offset(&self) -> DVec2 {
        self.camera.offset()
    }

    fn parent(&self) -> EntityId {
        self.camera.parent()
    }
}

pub const SPACECRAFT_HOVER_RADIUS: f64 = 30.0;

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
            piloting: None,
            hovered_entity: None,
        }
    }

    pub fn toggle_track(&mut self, id: EntityId) {
        if self.selected.contains(&id) {
            self.selected.retain(|e| *e != id);
        } else {
            self.selected.insert(id);
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
        let parent = state.universe.lup_planet(wrt_id)?;

        let r = p1.distance(parent.pv().pos);
        let v = (parent.body()?.mu() / r).sqrt();

        Some(PV::from_f64(p1, (p2 - p1) * v / r))
    }

    pub fn cursor_orbit(p1: DVec2, p2: DVec2, state: &GameState) -> Option<GlobalOrbit> {
        let pv = Self::cursor_pv(p1, p2, &state)?;
        let parent_id: EntityId =
            nearest_relevant_body(&state.universe.planets, pv.pos, state.universe.stamp())?;
        let parent = state.universe.lup_planet(parent_id)?;
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
            if let Some(pv) = universe.pv(follow) {
                self.camera.follow(follow, pv.pos);
            }
        }

        self.camera.on_game_tick();

        let mut track_list = self.selected.clone();
        track_list.retain(|o| universe.surface_vehicles.contains_key(o));
        self.selected = track_list;
    }

    pub fn on_render_tick(
        &mut self,
        on_ui: bool,
        input: &InputState,
        universe: &mut Universe,
        sounds: &mut EnvironmentSounds,
    ) {
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

        self.hovered_entity = if let Some(p) = input.position(MouseButt::Hover, FrameId::Current) {
            let dist = (SPACECRAFT_HOVER_RADIUS / self.scale()).max(10.0);
            let w = self.c2w(p);
            nearest_orbiter_or_planet(universe, w, dist)
        } else {
            None
        };

        if let Some(_) = input.on_frame(MouseButt::Left, FrameId::Down) {
            if input.is_pressed(KeyCode::ControlLeft) {
                self.following = self.hovered_entity;
                self.camera.clear_offset();
            } else {
                if let Some(h) = self.hovered_entity {
                    self.piloting = Some(h);
                    sounds.play_once("soft-pulse-higher.ogg", 0.3);
                } else {
                    self.piloting = None;
                    sounds.play_once("soft-pulse.ogg", 0.3);
                }
            }
        }

        if let Some(_) = input.on_frame(MouseButt::Right, FrameId::Down) {
            || -> Option<()> {
                let pilot = self.piloting?;
                let sv = universe.surface_vehicles.get_mut(&pilot)?;
                if self.hovered_entity != Some(pilot) {
                    if sv.target() == self.hovered_entity {
                        sv.set_target(None);
                    } else {
                        sv.set_target(self.hovered_entity);
                    }
                }
                Some(())
            }();
        }
    }
}

pub fn get_orbital_labels(state: &GameState) -> Vec<TextLabel> {
    let mut ret = Vec::new();

    let target_id = state
        .orbital_context
        .piloting
        .map(|p| state.universe.surface_vehicles.get(&p).map(|p| p.target()))
        .flatten()
        .flatten();

    for (id, alpha) in [
        (state.orbital_context.piloting, 0.3),
        (state.orbital_context.hovered_entity, 0.9),
        (target_id, 0.3),
    ] {
        let id = match id {
            Some(id) => id,
            None => continue,
        };

        let lup = if let Some(lup) = state.universe.lup_orbiter(id) {
            lup
        } else if let Some(lup) = state.universe.lup_planet(id) {
            lup
        } else {
            continue;
        };

        let pw = lup.pv().pos;
        let pc = state.orbital_context.w2c(pw);

        let label = if let Some((name, body)) = lup.named_body() {
            // distance based on world space
            let p = state.orbital_context.w2c(pw + DVec2::Y * body.radius);
            let text = name.to_uppercase();
            let pos = p + Vec2::Y * 50.0;
            TextLabel::new(text, pos, 1.0).with_color(WHITE.with_alpha(alpha))
        } else {
            let vehicle = state.universe.surface_vehicles.get(&id);
            let code = vehicle
                .map(|ov| ov.vehicle().title())
                .unwrap_or("UFO".to_string());

            let text = format!("{} {}", code, id);
            let pos = pc + Vec2::X * 40.0;

            let mut t = TextLabel::new(text, pos, 0.6).with_anchor_left();
            t.color = WHITE.with_alpha(alpha);
            t
        };
        ret.push(label);
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
    let mut text_labels: Vec<TextLabel> = get_orbital_labels(state);

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
        .map(|id| state.universe.lup_planet(id))
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
                    // .with_child(sidebar)
                    .with_child(world),
            );

        Some(Tree::new().with_layout(root, Vec2::ZERO))
    }
}
