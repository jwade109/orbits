use crate::drawing::*;
use crate::planetary::GameState;
use bevy::color::palettes::css::*;
use bevy::prelude::*;
use starling::prelude::*;

pub struct GridPlugin {}

impl GridPlugin {
    pub fn new() -> Self {
        GridPlugin {}
    }
}

#[derive(Component, Debug, Clone, Copy)]
#[require(Transform)]
struct Grid {
    x: i32,
    y: i32,
    z: f32,
    last_drawn: Nanotime,
    redraw: bool,
}

impl Grid {
    fn new(x: i32, y: i32) -> Self {
        Grid {
            x,
            y,
            z: -20.0,
            last_drawn: Nanotime::zero(),
            redraw: true,
        }
    }

    fn aabb(&self) -> AABB {
        let span = Vec2::new(SIDELENGTH, SIDELENGTH);
        let center = Vec2::new(self.x as f32, self.y as f32) * SIDELENGTH;
        AABB::new(center, span)
    }

    fn age(&self, now: Nanotime) -> Nanotime {
        now - self.last_drawn
    }
}

const TEXTURE_WIDTH: u32 = 5;
const WIDTH: i32 = 25;
const SIDELENGTH: f32 = 1000.0;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_grids);
        app.add_systems(Update, (draw_grids, redraw_grid_sprites));
        app.add_systems(FixedUpdate, update_grids);
    }
}

fn empty_image(w: u32, h: u32, color: Srgba) -> Image {
    use bevy::render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    };
    Image::new_fill(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &(color.to_u8_array()),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

fn spawn_grids(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let img = empty_image(TEXTURE_WIDTH, TEXTURE_WIDTH, BLACK);
    let handle = images.add(img);
    for x in -WIDTH..=WIDTH {
        for y in -WIDTH..=WIDTH {
            let g = Grid::new(x, y);
            let sprite = Sprite::from_image(handle.clone());
            let t = Transform::from_translation(g.aabb().center.extend(g.z))
                .with_scale(Vec3::splat(SIDELENGTH / (TEXTURE_WIDTH as f32)));
            commands.spawn((g, t, sprite));
        }
    }
}

fn draw_grids(mut gizmos: Gizmos, grids: Query<&Grid>, state: Res<GameState>) {
    for g in &grids {
        let dt = g.age(state.actual_time).to_secs();
        let anim_dur = 0.2;
        let a = 0.1 * (anim_dur - dt.min(anim_dur)) / anim_dur + 0.03;
        draw_aabb(&mut gizmos, g.aabb(), alpha(GRAY, a));
    }
}

fn update_grids(mut grids: Query<&mut Grid>, state: Res<GameState>) {
    let mut max_draw = 30;
    let mut sorted = grids.iter_mut().collect::<Vec<_>>();
    sorted.sort_by_key(|g| -g.age(state.actual_time).inner());

    let dur = Nanotime::secs(3);
    for mut g in sorted {
        if g.last_drawn + dur < state.sim_time {
            g.last_drawn = state.sim_time;
            g.redraw = true;
            max_draw -= 1;
            if max_draw == 0 {
                break;
            }
        }
    }
}

fn redraw_grid_sprites(
    mut grids: Query<(&mut Grid, &mut Sprite)>,
    state: Res<GameState>,
    mut images: ResMut<Assets<Image>>,
) {
    let levels = (-3000..-1000).step_by(500).collect::<Vec<_>>();

    for (mut g, mut s) in &mut grids {
        if !g.redraw {
            continue;
        }

        let scalar = |p: Vec2| -> f32 { state.scenario.system.potential_at(p, state.sim_time) };

        let mut image = empty_image(TEXTURE_WIDTH, TEXTURE_WIDTH, BLACK);

        if scalar(g.aabb().center) > -900.0 {
            g.redraw = false;
            let handle = images.add(image);
            *s = Sprite::from_image(handle);
            continue;
        }

        for x in 0..TEXTURE_WIDTH {
            for y in 0..TEXTURE_WIDTH {
                let uv = Vec2::new(x as f32, (TEXTURE_WIDTH - y) as f32)
                    * (SIDELENGTH / TEXTURE_WIDTH as f32);
                let p = g.aabb().lower() + uv;
                let mut dmin: f32 = f32::INFINITY;
                for level in &levels {
                    let d = (scalar(p) - *level as f32).abs();
                    dmin = dmin.min(d);
                }
                let mag = (255.0 - dmin * 3.0) as u8;
                let px = image.pixel_bytes_mut(UVec3::new(x, y, 0)).unwrap();
                px[0] = 50;
                px[1] = mag;
                px[2] = 50;
                px[3] = mag;
            }
        }

        let handle = images.add(image);
        *s = Sprite::from_image(handle);
        g.redraw = false;
    }
}
