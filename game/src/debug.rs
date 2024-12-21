use bevy::color::palettes::css::ORANGE;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use std::time::Duration;

pub struct DebugPlugin {}

#[derive(Resource, Debug, Default)]
struct DebugInfo {
    partial_frames: u32,
    elapsed_time: f32,
    total_frames: u32,
    last_fps: Option<f32>,
}

#[derive(Event)]
pub struct DebugLog {
    pub message: String,
    pub stamp: Duration,
}

#[derive(Event, Debug, Clone)]
pub struct DebugCommand(pub Vec<String>);

impl DebugInfo {
    fn framerate(&self) -> Option<f32> {
        match self.elapsed_time {
            0.0 => None,
            _ => Some(self.partial_frames as f32 / self.elapsed_time),
        }
    }
}

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_debug_readout);
        app.add_systems(
            Update,
            (update_fps_count, redraw_fps, keyboard_input, text_input),
        );
    }
}

#[derive(Component)]
struct DebugReadout {}

#[derive(Component)]
struct DebugKeyInput {}

fn spawn_debug_readout(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        DebugReadout {},
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(5.0),
            top: Val::Px(5.0),
            ..default()
        },
    ));
    commands.spawn((
        Text::new(""),
        DebugKeyInput {},
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(5.0),
            bottom: Val::Px(5.0),
            ..default()
        },
    ));
    commands.insert_resource(DebugInfo::default());
    commands.insert_resource(Events::<DebugLog>::default());
    commands.insert_resource(Events::<DebugCommand>::default());
    commands.insert_resource(CommandsState::default());
}

fn update_fps_count(time: Res<Time>, mut debug: ResMut<DebugInfo>) {
    if debug.elapsed_time >= 0.5 {
        debug.last_fps = debug.framerate();
        debug.partial_frames = 0;
        debug.elapsed_time = 0.0;
    }
    debug.partial_frames += 1;
    debug.total_frames += 1;
    debug.elapsed_time += time.delta().as_secs_f32();
}

fn redraw_fps(
    mut query: Query<&mut Text, With<DebugReadout>>,
    debug: Res<DebugInfo>,
    mut evt: EventReader<DebugLog>,
) {
    let mut logs = String::new();
    for e in evt.read() {
        logs.push_str(&format!("\n{}", e.message));
    }
    for mut t in query.iter_mut() {
        *t = Text::new(format!(
            "{:0.2} fps\n{} frames{}",
            debug.last_fps.unwrap_or(0.0),
            debug.total_frames,
            logs
        ));
    }
}

fn keyboard_input(keys: Res<ButtonInput<KeyCode>>) {
    for _key in keys.get_pressed() {
        // dbg!(key);
    }
}

#[derive(Resource, Default)]
pub struct CommandsState {
    pub text: String,
    pub active: bool,
}

fn text_input(
    mut events: EventReader<KeyboardInput>,
    mut query: Query<&mut Text, With<DebugKeyInput>>,
    mut cstate: ResMut<CommandsState>,
    mut evt: EventWriter<DebugCommand>,
) {
    for ev in events.read() {
        if ev.state == ButtonState::Released {
            continue;
        }
        match &ev.logical_key {
            Key::Enter => {
                if cstate.active {
                    evt.send(DebugCommand(
                        cstate.text[1..].split(" ").map(|s| s.to_string()).collect(),
                    ));
                    cstate.active = false;
                }
                cstate.text.clear();
            }
            Key::Backspace => {
                cstate.text.pop();
            }
            Key::Space => {
                cstate.text.push(' ');
            }
            Key::Character(input) => {
                if input.chars().any(|c| c.is_control()) {
                    continue;
                }
                cstate.text.push_str(&input);
            }
            _ => (),
        }
    }

    cstate.active = Some(':') == cstate.text.chars().nth(0);
    if !cstate.active {
        cstate.text.clear()
    }

    for mut txt in query.iter_mut() {
        txt.clear();
        txt.push_str(&cstate.text);
        txt.push('_');
    }
}

pub fn send_log(evt: &mut EventWriter<DebugLog>, message: &str) {
    let log = DebugLog {
        message: message.into(),
        stamp: Duration::default(),
    };
    evt.send(log);
}
