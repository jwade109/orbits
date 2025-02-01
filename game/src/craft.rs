use bevy::color::palettes::basic::*;
use bevy::prelude::*;

use crate::drawing::*;
use starling::aabb::*;
use starling::pv::PV;

pub struct CraftPlugin;

impl Plugin for CraftPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_system);
        app.add_systems(Update, (update, draw).chain());
    }
}

#[derive(Default, Debug)]
struct Spacecraft {
    pv: PV,
    body: Vec<AABB>,
}

fn spacecraft_mesh() -> Vec<AABB> {
    vec![
        AABB::from_arbitrary((-300.0, -70.0), (200.0, 340.0)).scale(5.0),
        AABB::from_arbitrary((-40.0, -70.0), (210.0, 65.0)).scale(5.0),
    ]
}

impl Spacecraft {
    fn new(pv: impl Into<PV>) -> Self {
        Spacecraft {
            pv: pv.into(),
            body: spacecraft_mesh(),
        }
    }
}

#[derive(Resource)]
struct CraftState {
    crafts: Vec<Spacecraft>,
}

impl Default for CraftState {
    fn default() -> Self {
        CraftState {
            crafts: vec![Spacecraft::new(((10.0, 20.0), (40.0, -2.0)))],
        }
    }
}

fn init_system(mut commands: Commands) {
    commands.insert_resource(CraftState::default());
}

fn update(mut state: ResMut<CraftState>, time: Res<Time>) {
    let dt = time.delta_secs();
    for sp in &mut state.crafts {
        let v = sp.pv.vel;
        sp.pv.pos += v * dt;
    }
}

fn draw(mut gizmos: Gizmos, state: Res<CraftState>) {
    for sp in &state.crafts {
        draw_circle(&mut gizmos, sp.pv.pos, 30.0, WHITE);
        for b in &sp.body {
            draw_aabb(&mut gizmos, *b, WHITE);
        }
    }
}
