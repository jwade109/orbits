use avian2d::prelude::*;
use bevy::color::palettes::css::*;
use bevy::core_pipeline::bloom::Bloom;
use bevy::input::mouse::MouseWheel;
use bevy::pbr::PointLightShadowMap;
use bevy::prelude::*;
use bevy::sprite::Wireframe2dPlugin;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use bevy_inspector_egui::quick::*;
use bevy_light_2d::prelude::*;
use bevy_vector_shapes::prelude::*;
use game::args::ProgramContext;
use game::new::animated_text::*;
use game::new::particles::*;
use game::new::spacecraft::*;
use starling::prelude::rand;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    unapproved_path_mode: bevy::asset::UnapprovedPathMode::Allow,
                    ..default()
                }),
        )
        .insert_gizmo_config(
            PhysicsGizmos {
                aabb_color: Some(Color::WHITE),
                ..default()
            },
            GizmoConfig::default(),
        )
        // 3rd-party plugins
        .add_plugins(MeshPickingPlugin)
        .add_plugins(Wireframe2dPlugin::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        .add_systems(EguiPrimaryContextPass, egui_ui)
        .add_plugins(Shape2dPlugin::default())
        .add_plugins(Light2dPlugin)
        // plugins I've implemented
        .add_plugins(ParticlePlugin)
        .add_plugins(AnimatedTextPlugin)
        .add_plugins(SpacecraftPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, control_camera)
        .run();
}

fn control_camera(
    mut camera: Single<&mut Transform, With<Camera>>,
    key: Res<ButtonInput<KeyCode>>,
    mut scroll: EventReader<MouseWheel>,
) {
    let speed = 9.0 * camera.scale.max_element();

    if key.pressed(KeyCode::KeyW) {
        camera.translation.y += speed;
    }
    if key.pressed(KeyCode::KeyS) {
        camera.translation.y -= speed;
    }
    if key.pressed(KeyCode::KeyA) {
        camera.translation.x -= speed;
    }
    if key.pressed(KeyCode::KeyD) {
        camera.translation.x += speed;
    }

    use bevy::input::mouse::MouseScrollUnit;

    for ev in scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                // println!("Scroll (line units): vertical: {}, horizontal: {}", ev.y, ev.x);
            }
            MouseScrollUnit::Pixel => {
                // println!("Scroll (pixel units): vertical: {}, horizontal: {}", ev.y, ev.x);
            }
        }

        if ev.y > 0.0 {
            camera.scale /= 1.15;
        } else {
            camera.scale *= 1.15;
        }
    }
}

fn setup(mut commands: Commands, mut ambient_light: ResMut<AmbientLight>) -> Result {
    ambient_light.color = BLACK.into();
    ambient_light.brightness = 0.0;

    commands.insert_resource(ProgramContext::default());

    commands.insert_resource(ClearColor(BLACK.into()));

    commands.insert_resource(Gravity(Vec2::ZERO));

    commands.spawn((
        Camera2d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Light2d::default(),
        Transform::from_xyz(0.0, 20.0, 0.0).with_scale(Vec3::splat(0.04)),
        Bloom {
            intensity: 0.2,
            ..Bloom::OLD_SCHOOL
        },
    ));

    commands.spawn((
        PointLight2d {
            intensity: 3.0,
            radius: 500.0,
            cast_shadows: true,
            color: WHITE.into(),
            ..default()
        },
        Transform::from_xyz(0.0, 20.0, 0.0),
    ));

    for name in [
        "pollux",
        "remora",
        "bellerophon",
        "lander",
        "remora",
        "icecream",
    ] {
        let x = rand(-200.0, 200.0);
        let y = rand(100.0, 300.0);
        commands.send_event(SpacecraftEvent::Spawn {
            name: name.to_string(),
            pos: Vec2::new(x, y),
        });
    }

    Ok(())
}

struct DebugPanelState {
    message_color: [f32; 3],
    message_text: String,
    sc_name: String,
    sc_pos: Vec2,
    sc_diagnostic_view: SpacecraftView,
}

impl Default for DebugPanelState {
    fn default() -> Self {
        Self {
            message_color: [0.2, 0.2, 1.0],
            message_text: "This is some example text!\nIt can contain newlines.".to_string(),
            sc_name: "pollux".to_string(),
            sc_pos: Vec2::Y * 50.0,
            sc_diagnostic_view: SpacecraftView::Real,
        }
    }
}

fn egui_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut state: Local<DebugPanelState>,
) -> Result {
    egui::Window::new("Debug Panel").show(contexts.ctx_mut()?, |ui| {
        ui.label("Text Notification");

        ui.text_edit_multiline(&mut state.message_text);
        ui.color_edit_button_rgb(&mut state.message_color);

        if ui.button("Spawn Text").clicked() {
            commands.send_event(SpawnAnimText {
                text: state.message_text.clone(),
                color: Srgba::from_f32_array([
                    state.message_color[0],
                    state.message_color[1],
                    state.message_color[2],
                    1.0,
                ]),
                pos: None,
            });
        }

        ui.separator();

        ui.label("Spawn Spacecraft");

        ui.horizontal(|ui| {
            ui.label("Model: ");
            ui.text_edit_singleline(&mut state.sc_name);
            ui.label("X: ");
            ui.add(egui::DragValue::new(&mut state.sc_pos.x));
            ui.label("Y: ");
            ui.add(egui::DragValue::new(&mut state.sc_pos.y));
        });

        if ui.button("Spawn Spacecraft").clicked() {
            commands.send_event(SpacecraftEvent::Spawn {
                name: state.sc_name.clone(),
                pos: state.sc_pos,
            });
        }

        ui.separator();

        let before = state.sc_diagnostic_view;
        ui.label("Spacecraft View");
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Select one!")
                .selected_text(format!("{:?}", state.sc_diagnostic_view))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut state.sc_diagnostic_view,
                        SpacecraftView::Real,
                        "Real",
                    );
                    ui.selectable_value(
                        &mut state.sc_diagnostic_view,
                        SpacecraftView::Damage,
                        "Damage",
                    );
                    ui.selectable_value(
                        &mut state.sc_diagnostic_view,
                        SpacecraftView::Type,
                        "Type",
                    );
                });
        });

        if before != state.sc_diagnostic_view {
            commands.send_event(SpacecraftEvent::ToggleView(state.sc_diagnostic_view));
        }
    });
    Ok(())
}
