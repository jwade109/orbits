use bevy::prelude::*;

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
}

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
        app.add_systems(Update, (update_fps_count, redraw_fps, keyboard_input));
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
        TextColor(
            Srgba {
                alpha: 0.1,
                ..bevy::color::palettes::basic::WHITE
            }
            .into(),
        ),
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

pub fn send_log(evt: &mut EventWriter<DebugLog>, message: &str) {
    let log = DebugLog {
        message: message.into(),
    };
    evt.send(log);
}
