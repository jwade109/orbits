use crate::mouse::{FrameId, InputState, MouseButt};
use crate::onclick::OnClick;
use crate::planetary::GameState;
use crate::scenes::{Render, StaticSpriteDescriptor, TextLabel};
use crate::ui::*;
use bevy::color::palettes::css::*;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use enum_iterator::all;
use enum_iterator::Sequence;
use layout::layout::{Node, Size, Tree};
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
        Some(lup.pv().pos_f32())
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

        let r = p1.distance(parent.pv().pos_f32());
        let v = (parent.body()?.mu() / r).sqrt();

        Some(PV::from_f64(p1, (p2 - p1) * v / r))
    }

    pub fn cursor_orbit(p1: Vec2, p2: Vec2, state: &GameState) -> Option<GlobalOrbit> {
        let pv = Self::cursor_pv(p1, p2, &state)?;
        let parent_id: PlanetId = state.scenario.relevant_body(pv.pos_f32(), state.sim_time)?;
        let parent = state.scenario.lup_planet(parent_id, state.sim_time)?;
        let parent_pv = parent.pv();
        let pv = pv - PV::pos(parent_pv.pos_f32());
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

pub fn get_orbital_object_mouseover_labels(state: &GameState) -> Vec<TextLabel> {
    let mut ret = Vec::new();

    let cursor = match state.input.position(MouseButt::Hover, FrameId::Current) {
        Some(p) => p,
        None => return Vec::new(),
    };

    let cursor_world = state.orbital_context.c2w(cursor);

    for id in state.scenario.ids() {
        let lup = match state.scenario.lup(id, state.sim_time) {
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
            let vehicle = state.vehicles.get(&orb_id);
            let rpo = state.rpos.contains_key(&orb_id);
            let ufo = if rpo { "RPO" } else { "UFO" }.to_string();
            let code = vehicle.map(|v| v.name()).unwrap_or(&ufo);

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

fn get_thruster_indicators(state: &GameState) -> Option<Vec<TextLabel>> {
    let piloting = state.piloting()?;
    let vehicle = state.vehicles.get(&piloting)?;
    let origin = Vec2::new(state.input.screen_bounds.span.x * 0.5 - 100.0, 0.0);

    Some(
        vehicle
            .thrusters()
            .enumerate()
            .map(|(i, t)| {
                let text = format!("{} / {}", i, t.proto.model.clone());
                let pos = origin + Vec2::Y * 26.0 * i as f32;
                let color = if t.is_thrusting() {
                    RED.with_alpha(0.8)
                } else {
                    WHITE.with_alpha(0.6)
                };
                TextLabel::new(text, pos, 0.7).with_color(color)
            })
            .collect(),
    )
}

impl Render for OrbitalContext {
    fn text_labels(state: &GameState) -> Option<Vec<TextLabel>> {
        let mut text_labels: Vec<TextLabel> = get_orbital_object_mouseover_labels(state);

        text_labels.extend(get_thruster_indicators(state).unwrap_or(vec![]));

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

        const SPACECRAFT_DEFAULT_SCALE: f32 = 0.025;
        const SPACECRAFT_MAGNIFIED_SCALE: f32 = 0.06;
        const SPACECRAFT_DIMINISHED_SCALE: f32 = 0.02;
        const SPACECRAFT_Z_INDEX: f32 = 6.0;

        let ctx = &state.orbital_context;
        let mut planetary_sprites: Vec<_> = state
            .scenario
            .planet_ids()
            .into_iter()
            .filter_map(|id| {
                let lup = state.scenario.lup_planet(id, state.sim_time)?;
                let pos = lup.pv().pos_f32();
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
            .collect();

        let bodies: Vec<_> = state
            .scenario
            .planets()
            .bodies(state.sim_time, None)
            .collect();
        let light_source = state.light_source();
        let orbiter_sprites: Vec<_> = state
            .scenario
            .orbiter_ids()
            .filter_map(|id| {
                let lup = state.scenario.lup_orbiter(id, state.sim_time)?;
                let pos = lup.pv().pos_f32();
                let is_lit = bodies
                    .iter()
                    .all(|(pv, body)| !is_occluded(light_source, pos, pv.pos_f32(), body.radius));

                let path = "embedded://game/../assets/spacecraft.png".to_string();
                let scale = if state.orbital_context.selected.contains(&id) {
                    SPACECRAFT_MAGNIFIED_SCALE
                } else if state.orbital_context.selected.is_empty() {
                    SPACECRAFT_DEFAULT_SCALE
                } else {
                    SPACECRAFT_DIMINISHED_SCALE
                };

                let color = if is_lit { WHITE } else { GRAY };

                Some(
                    StaticSpriteDescriptor::new(ctx.w2c(pos), 0.0, path, scale, SPACECRAFT_Z_INDEX)
                        .with_color(color),
                )
            })
            .collect();

        planetary_sprites.extend(orbiter_sprites);
        Some(planetary_sprites)
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

    fn ui(state: &GameState) -> Option<Tree<OnClick>> {
        let vb = state.input.screen_bounds;
        if vb.span.x == 0.0 || vb.span.y == 0.0 {
            return Some(Tree::new());
        }

        let topbar = top_bar(state);

        let mut sidebar = Node::column(300).with_color(UI_BACKGROUND_COLOR);

        let body_color_lup: std::collections::HashMap<&'static str, Srgba> =
            std::collections::HashMap::from([("Earth", BLUE), ("Luna", GRAY), ("Asteroid", BROWN)]);

        if let Some(lup) = state
            .scenario
            .relevant_body(state.orbital_context.origin(), state.sim_time)
            .map(|id| state.scenario.lup_planet(id, state.sim_time))
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
                        OnClick::CurrentBody(lup.id().planet().unwrap()),
                        Size::Grow,
                        BUTTON_HEIGHT,
                    )
                    .with_color(color.to_f32_array()),
                );
            }
        }

        sidebar.add_child(Node::button(
            format!("Visual: {:?}", state.orbital_context.draw_mode),
            OnClick::ToggleDrawMode,
            Size::Grow,
            BUTTON_HEIGHT,
        ));

        sidebar.add_child(
            Node::button(
                "Clear Orbits",
                OnClick::ClearOrbits,
                Size::Grow,
                BUTTON_HEIGHT,
            )
            .enabled(!state.orbital_context.queued_orbits.is_empty()),
        );

        sidebar.add_child(
            Node::button(
                "Commit Mission",
                OnClick::CommitMission,
                Size::Grow,
                BUTTON_HEIGHT,
            )
            .enabled(state.current_orbit().is_some() && !state.orbital_context.selected.is_empty()),
        );

        sidebar.add_child(Node::hline());

        sidebar.add_children(all::<CursorMode>().map(|c| {
            let s = format!("{:?}", c);
            let id = OnClick::CursorMode(c);
            Node::button(s, id, Size::Grow, BUTTON_HEIGHT)
                .enabled(c != state.orbital_context.cursor_mode)
        }));

        if !state.constellations.is_empty() {
            sidebar.add_child(Node::hline());
        }

        for gid in state.unique_groups() {
            let color: Srgba = crate::sprites::hashable_to_color(gid)
                .with_luminance(0.3)
                .into();
            let s = format!("{}", gid);
            let id = OnClick::Group(gid.clone());
            let button =
                Node::button(s, id, Size::Grow, BUTTON_HEIGHT).with_color(color.to_f32_array());
            sidebar.add_child(delete_wrapper(
                OnClick::DisbandGroup(gid.clone()),
                button,
                BUTTON_HEIGHT as f32,
            ));
        }

        sidebar.add_child(Node::hline());

        if append_piloting_buttons(state, &mut sidebar) {
            sidebar.add_child(Node::hline());
        }

        sidebar.add_child({
            let s = format!("{} selected", state.orbital_context.selected.len());
            let b =
                Node::button(s, OnClick::SelectedCount, Size::Grow, BUTTON_HEIGHT).enabled(false);
            if state.orbital_context.selected.is_empty() {
                b
            } else {
                delete_wrapper(OnClick::ClearTracks, b, BUTTON_HEIGHT as f32)
            }
        });

        let orbiter_list = |root: &mut Node<OnClick>, max_cells: usize, mut ids: Vec<OrbiterId>| {
            ids.sort();

            let rows = (ids.len().min(max_cells) as f32 / 4.0).ceil() as u32;
            let grid = Node::grid(Size::Grow, rows * BUTTON_HEIGHT as u32, rows, 4, 4.0, |i| {
                if i as usize > max_cells {
                    return None;
                }
                let id = ids.get(i as usize)?;
                let s = format!("{id}");
                Some(
                    Node::grow()
                        .with_on_click(OnClick::Orbiter(*id))
                        .with_text(s)
                        .enabled(
                            Some(*id)
                                != state
                                    .orbital_context
                                    .following
                                    .map(|f| f.orbiter())
                                    .flatten(),
                        ),
                )
            });
            root.add_child(grid);

            if ids.len() > max_cells {
                let n = ids.len() - max_cells;
                let s = format!("...And {} more", n);
                root.add_child(
                    Node::new(Size::Grow, BUTTON_HEIGHT)
                        .with_text(s)
                        .enabled(false),
                );
            }
        };

        if !state.orbital_context.selected.is_empty() {
            orbiter_list(
                &mut sidebar,
                32,
                state.orbital_context.selected.iter().cloned().collect(),
            );
            sidebar.add_child(Node::button(
                "Create Group",
                OnClick::CreateGroup,
                Size::Grow,
                BUTTON_HEIGHT,
            ));
        }

        if !state.controllers.is_empty() {
            sidebar.add_child(Node::hline());
            let s = format!("{} autopiloting", state.controllers.len());
            let id = OnClick::AutopilotingCount;
            sidebar.add_child(Node::button(s, id, Size::Grow, BUTTON_HEIGHT).enabled(false));

            let ids = state.controllers.iter().map(|c| c.target()).collect();
            orbiter_list(&mut sidebar, 16, ids);
        }

        let mut inner_topbar = sim_time_toolbar(state);

        if let Some(id) = state.orbital_context.following {
            let s = format!("Following {}", id);
            let id = OnClick::Nullopt;
            let n = Node::button(s, id, 300, BUTTON_HEIGHT).enabled(false);
            inner_topbar.add_child(n);
        }

        for (i, orbit) in state.orbital_context.queued_orbits.iter().enumerate() {
            let orbit_button = {
                let s = format!("{}", orbit);
                let id = OnClick::GlobalOrbit(i);
                Node::button(s, id, 400, BUTTON_HEIGHT)
            };

            inner_topbar.add_child(delete_wrapper(
                OnClick::DeleteOrbit(i),
                orbit_button,
                BUTTON_HEIGHT as f32,
            ));
        }

        let notif_bar = notification_bar(state, Size::Fixed(900.0));

        let throttle_controls = throttle_controls(state);
        let thruster_controls = thruster_control_dialogue(state).unwrap_or(Node::new(0, 0));

        let world = Node::grow()
            .down()
            .invisible()
            .with_child(
                Node::grow()
                    .down()
                    .invisible()
                    .with_child(inner_topbar)
                    .with_child(throttle_controls)
                    .with_child(thruster_controls),
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
            .with_child(topbar)
            .with_child(
                Node::grow()
                    .tight()
                    .invisible()
                    .with_child(sidebar)
                    .with_child(world),
            );

        let tree = Tree::new().with_layout(root, Vec2::ZERO);

        // if let Some(layout) = current_inventory_layout(state) {
        //     tree.add_layout(layout, Vec2::splat(400.0));
        // }

        Some(tree)
    }
}

pub fn delete_wrapper(ondelete: OnClick, button: Node<OnClick>, box_size: f32) -> Node<OnClick> {
    let x_button = {
        let s = "X";
        Node::button(s, ondelete, box_size, box_size).with_color(DELETE_SOMETHING_COLOR)
    };

    let (w, _) = button.desired_dims();

    let width = match w {
        Size::Fit => Size::Fit,
        Size::Fixed(n) => Size::Fixed(n + box_size),
        Size::Grow => Size::Grow,
    };

    Node::new(width, box_size)
        .tight()
        .invisible()
        .with_child(x_button)
        .with_child(button)
}

pub fn thruster_control_dialogue(state: &GameState) -> Option<Node<OnClick>> {
    let id = state.piloting()?;
    let vehicle = state.vehicles.get(&id)?;

    let mut wrapper = Node::new(320, Size::Fit)
        .down()
        .with_color(UI_BACKGROUND_COLOR);

    let active_color: [f32; 4] = [0.3, 0.2, 0.2, 1.0];
    // let forced_color: [f32; 4] = [0.9, 0.2, 0.2, 1.0];

    for (i, thruster) in vehicle.thrusters().enumerate() {
        let torque = cross2d(thruster.pos, thruster.pointing());

        let dir = if torque > 1.0 {
            " [LEFT]"
        } else if torque < -1.0 {
            " [RIGHT]"
        } else {
            ""
        };

        let s = format!("#{} / {}{}", i + 1, thruster.proto.model, dir);
        let onclick = OnClick::ToggleThruster(i);
        let mut child = Node::button(s, onclick, Size::Grow, BUTTON_HEIGHT);

        if thruster.is_thrusting() {
            child.set_color(active_color);
        }
        wrapper.add_child(child);
    }

    Some(wrapper)
}

pub fn append_piloting_buttons(state: &GameState, sidebar: &mut Node<OnClick>) -> bool {
    // piloting and secondary spacecrafts

    let x = if let Some(p) = state.orbital_context.piloting {
        sidebar.add_child({
            let s = format!("Piloting {:?}", p);
            let b = Node::button(s, OnClick::Orbiter(p), Size::Grow, BUTTON_HEIGHT);
            delete_wrapper(OnClick::ClearPilot, b, BUTTON_HEIGHT as f32)
        });
        true
    } else if let Some(ObjectId::Orbiter(p)) = state.orbital_context.following {
        sidebar.add_child({
            let s = format!("Pilot {:?}", p);
            Node::button(s, OnClick::SetPilot(p), Size::Grow, BUTTON_HEIGHT)
        });
        true
    } else {
        false
    };

    let y = if let Some(p) = state.orbital_context.targeting {
        sidebar.add_child({
            let s = format!("Targeting {:?}", p);
            let b = Node::button(s, OnClick::Orbiter(p), Size::Grow, BUTTON_HEIGHT);
            delete_wrapper(OnClick::ClearTarget, b, BUTTON_HEIGHT as f32)
        });
        true
    } else if let Some(ObjectId::Orbiter(p)) = state.orbital_context.following {
        sidebar.add_child({
            let s = format!("Target {:?}", p);
            Node::button(s, OnClick::SetTarget(p), Size::Grow, BUTTON_HEIGHT)
        });
        true
    } else {
        false
    };

    x || y
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
