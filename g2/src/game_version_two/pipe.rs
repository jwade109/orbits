use bevy::color::palettes::tailwind::GRAY_400;
use game::ui::apply_egui_style;

use crate::game_version_two::*;

pub struct InventoryTransferPlugin;

impl Plugin for InventoryTransferPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, process_pipes.in_set(Sets::Physics))
            .add_systems(PostUpdate, draw_pipes.in_set(Sets::Draw));
    }
}

#[derive(Component, Debug)]
pub struct Pipe {
    from: Entity,
    to: Entity,
    item: Item,
    count: u64,
    status: MachineStatus,
}

fn process_pipes(mut pipe: Query<&mut Pipe>, mut inventories: Query<&mut Inventory>) {
    for mut pipe in pipe {
        if pipe.from == pipe.to {
            continue;
        }

        let [mut src, mut dst] = ok_or_continue!(inventories.get_many_mut([pipe.from, pipe.to]));

        pipe.status = atomic_transfer(&mut src, &mut dst, pipe.item, pipe.count);
    }
}

const Z_PIPE_LAYER: f32 = 0.06;

fn draw_pipes(
    mut painter: ShapePainter,
    pipes: Query<&Pipe>,
    transforms: Query<&GlobalTransform, With<Inventory>>,
) {
    for transfer in pipes {
        let a = ok_or_continue!(transforms.get(transfer.from));
        let b = ok_or_continue!(transforms.get(transfer.to));

        painter.reset();
        painter.set_translation(Vec3::Z * Z_PIPE_LAYER);

        for (color, thickness) in [(GRAY_400, 0.07)] {
            painter.set_color(color);
            painter.thickness = thickness;
            painter.cap = Cap::Square;
            painter.line(
                a.translation().with_z(Z_PIPE_LAYER),
                b.translation().with_z(Z_PIPE_LAYER),
            );
        }
    }
}
