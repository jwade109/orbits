use crate::game_version_two::*;

#[derive(Default)]
pub struct MeshMaker {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

fn to_arr(p: Vec2) -> [f32; 3] {
    [p.x, p.y, 0.0]
}

impl MeshMaker {
    pub fn triangle(&mut self, points: [Vec2; 3]) {
        self.positions.push(to_arr(points[0]));
        self.positions.push(to_arr(points[1]));
        self.positions.push(to_arr(points[2]));

        for _ in 0..3 {
            self.normals.push([0.0, 0.0, 1.0]);
            self.uvs.push([0.0, 0.0]);
        }

        self.indices.push((self.positions.len() - 3) as u32);
        self.indices.push((self.positions.len() - 2) as u32);
        self.indices.push((self.positions.len() - 1) as u32);
    }
}
