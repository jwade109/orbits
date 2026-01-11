use bevy::{
    prelude::Entity,
    transform::components::{GlobalTransform, Transform},
};
use game::starling::parts::PartCoord;

#[derive(Debug, Clone, Copy)]
pub struct GridCoord {
    pub grid: Entity,
    pub coord: PartCoord,
}

impl GridCoord {
    pub fn get_transform(&self, grid_tf: Transform) -> Transform {
        let offset = self.coord.to_meters_center();
        let offset = grid_tf.right() * offset.x + grid_tf.up() * offset.y;
        let translation = grid_tf.translation + offset;
        Transform::from_translation(translation)
    }
}
