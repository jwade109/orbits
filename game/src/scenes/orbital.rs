use crate::camera_controller::*;
use crate::canvas::Canvas;
use crate::game::GameState;
use crate::onclick::OnClick;
use crate::prelude::*;
use crate::scenes::*;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use enum_iterator::Sequence;
use layout::layout::Tree;
use starling::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Sequence)]
pub enum CursorMode {
    #[default]
    Rect,
    AddOrbit,
    NearOrbit,
    MeasuringTape,
    Protractor,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct OrbitalContext {
    pub camera: LinearCameraController,
    pub following: Option<EntityId>,
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

    fn distance(&self) -> f64 {
        self.camera.distance()
    }

    fn angle(&self) -> f64 {
        self.camera.angle()
    }
}

pub const SPACECRAFT_HOVER_RADIUS: f64 = 30.0;

impl OrbitalContext {
    pub fn new() -> Self {
        Self {
            camera: LinearCameraController::new(DVec2::ZERO, 100000.0),
            following: None,
            piloting: None,
            hovered_entity: None,
        }
    }

    pub fn on_game_tick(&mut self, universe: &Universe) {
        if let Some(follow) = self.following {
            if let Some(pv) = universe.pv(follow) {
                self.camera.follow(follow, pv.pos);
            }
        }

        self.camera.on_game_tick();
    }
}

pub fn get_orbital_labels(state: &GameState) -> Vec<TextLabel> {
    let mut ret = Vec::new();

    let target_id = state
        .orbital_context
        .piloting
        .map(|p| state.universe.spacecraft.get(&p).map(|p| p.target()))
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

        let pv = match state.universe.pv(id) {
            Some(pv) => pv,
            None => continue,
        };

        let planet = state.universe.get_planet(id);

        let pw = pv.pos;
        let pc = state.orbital_context.w2c(pw);

        let label = if let Some(planet) = planet {
            // distance based on world space
            let p = state
                .orbital_context
                .w2c(pw + DVec2::Y * planet.body.radius);
            let text = planet.name.to_uppercase();
            let pos = p + Vec2::Y * 50.0;
            TextLabel::new(text, pos, 1.0).with_color(WHITE.with_alpha(alpha))
        } else {
            let vehicle = state.universe.spacecraft.get(&id);
            let code = vehicle
                .map(|ov| {
                    let title = ov.vehicle().title_with_id(id);
                    if ov.controller.is_idle() {
                        title
                    } else {
                        format!(
                            "{}\n{}\n{:?}",
                            title,
                            ov.controller.mode().to_status_str(),
                            ov.controller.status()
                        )
                    }
                })
                .unwrap_or("UFO".to_string());

            let r = vehicle
                .map(|v| v.vehicle.bounding_radius() * state.orbital_context.scale() * 1.1)
                .unwrap_or(40.0)
                .max(40.0);

            let pos = pc + Vec2::X * r as f32;

            TextLabel::new(code, pos, 0.6)
                .with_anchor(Anchor::CenterLeft)
                .with_color(WHITE.with_alpha(alpha))
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

impl Render for OrbitalContext {
    fn background_color(state: &GameState) -> bevy::color::Srgba {
        BLACK
    }

    fn draw(canvas: &mut Canvas, state: &GameState) -> Option<()> {
        crate::drawing::draw_orbital_view(canvas, state);

        for label in get_orbital_labels(state) {
            canvas.label(label);
        }

        Some(())
    }

    fn ui(_state: &GameState) -> Option<Tree<OnClick>> {
        return None;

        // let vb = state.input.screen_bounds;
        // if vb.span.x == 0.0 || vb.span.y == 0.0 {
        //     return Some(Tree::new());
        // }

        // let mut sidebar = Node::column(300).with_color(UI_BACKGROUND_COLOR);

        // let body_color_lup: std::collections::HashMap<&'static str, Srgba> =
        //     std::collections::HashMap::from([("Earth", BLUE), ("Luna", GRAY), ("Asteroid", BROWN)]);

        // if let Some(lup) = nearest_relevant_body(
        //     &state.universe.planets,
        //     state.orbital_context.origin(),
        //     state.universe.stamp(),
        // )
        // .map(|id| state.universe.get_planet(id))
        // .flatten()
        // {
        //     if let Some((s, _)) = lup.get_planet() {
        //         let color: Srgba = body_color_lup
        //             .get(s.as_str())
        //             .unwrap_or(&Srgba::from(crate::sprites::hashable_to_color(s)))
        //             .with_luminance(0.2)
        //             .with_alpha(0.9);
        //         sidebar.add_child(
        //             Node::button(
        //                 s,
        //                 OnClick::CurrentBody(lup.id()),
        //                 Size::Grow,
        //                 state.settings.ui_button_height,
        //             )
        //             .with_color(color.to_f32_array()),
        //         );
        //     }
        // }

        // sidebar.add_child(Node::button(
        //     format!("Visual: {:?}", state.orbital_context.draw_mode),
        //     OnClick::ToggleDrawMode,
        //     Size::Grow,
        //     state.settings.ui_button_height,
        // ));

        // sidebar.add_child(
        //     Node::button(
        //         "Commit Mission",
        //         OnClick::CommitMission,
        //         Size::Grow,
        //         state.settings.ui_button_height,
        //     )
        //     .enabled(state.current_orbit().is_some() && !state.orbital_context.selected.is_empty()),
        // );

        // sidebar.add_child(Node::hline());

        // sidebar.add_children(all::<CursorMode>().map(|c| {
        //     let s = format!("{:?}", c);
        //     let id = OnClick::CursorMode(c);
        //     Node::button(s, id, Size::Grow, state.settings.ui_button_height)
        //         .enabled(c != state.orbital_context.cursor_mode)
        // }));

        // if !state.universe.constellations.is_empty() {
        //     sidebar.add_child(Node::hline());
        // }

        // for gid in state.universe.unique_groups() {
        //     let color: Srgba = crate::sprites::hashable_to_color(&gid)
        //         .with_luminance(0.3)
        //         .into();
        //     let s = format!("{}", gid);
        //     let id = OnClick::Group(gid.clone());
        //     let button = Node::button(s, id, Size::Grow, state.settings.ui_button_height)
        //         .with_color(color.to_f32_array());
        //     sidebar.add_child(delete_wrapper(
        //         OnClick::DisbandGroup(gid.clone()),
        //         button,
        //         state.settings.ui_button_height as f32,
        //     ));
        // }

        // sidebar.add_child(Node::hline());

        // sidebar.add_child(piloting_buttons(state, Size::Grow));

        // sidebar.add_child(selected_button(state, Size::Grow));

        // if !state.orbital_context.selected.is_empty() {
        //     orbiter_list(
        //         state,
        //         &mut sidebar,
        //         32,
        //         state.orbital_context.selected.iter().cloned().collect(),
        //     );
        //     sidebar.add_child(Node::button(
        //         "Create Group",
        //         OnClick::CreateGroup,
        //         Size::Grow,
        //         state.settings.ui_button_height,
        //     ));
        // }

        // let mut inner_topbar = Node::fit().with_color(UI_BACKGROUND_COLOR);

        // let notif_bar = notification_bar(state, Size::Fixed(900.0));

        // let world = Node::grow()
        //     .down()
        //     .invisible()
        //     .tight()
        //     .with_child(Node::grow().down().invisible().with_child(inner_topbar))
        //     .with_child(
        //         Node::grow()
        //             .tight()
        //             .down()
        //             .invisible()
        //             .with_child(Node::grow().invisible())
        //             .with_child(notif_bar),
        //     );

        // let root = Node::new(vb.span.x, vb.span.y)
        //     .down()
        //     .tight()
        //     .invisible()
        //     .with_child(top_bar(state))
        //     .with_child(
        //         Node::grow()
        //             .tight()
        //             .invisible()
        //             // .with_child(sidebar)
        //             .with_child(world),
        //     );

        // Some(Tree::new().with_layout(root, Vec2::ZERO))
    }
}
