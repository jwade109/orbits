use bary_core::prelude::PartCoord;
use bevy::prelude::*;

#[derive(Clone, Copy)]
pub struct GridCoord {
    pub entity: Entity,
    pub coord: PartCoord,
    pub position: Vec2,
}

impl GridCoord {
    pub fn get_transform(&self, grid_tf: Transform) -> Transform {
        let offset = self.coord.to_meters_center();
        let offset = grid_tf.right() * offset.x + grid_tf.up() * offset.y;
        let translation = grid_tf.translation + offset;
        Transform::from_translation(translation)
    }
}

impl std::fmt::Debug for GridCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({}, {}) ({})",
            self.entity,
            self.coord.inner().x,
            self.coord.inner().y,
            self.position,
        )
    }
}
