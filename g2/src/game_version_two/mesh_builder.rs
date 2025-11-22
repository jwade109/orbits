use crate::game_version_two::*;

#[derive(Default)]
pub struct MeshMaker {
    color: Srgba,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub colors: Vec<[f32; 4]>,
}

fn to_arr(p: Vec2) -> [f32; 3] {
    [p.x, p.y, 0.0]
}

impl MeshMaker {
    pub fn build(self) -> Mesh {
        let usage = RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD;
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, usage);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }

    pub fn set_color(&mut self, color: Srgba) {
        self.color = color.with_alpha(0.4);
    }

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

        self.colors.extend_from_slice(&[
            self.color.to_f32_array(),
            self.color.to_f32_array(),
            self.color.to_f32_array(),
        ]);
    }

    pub fn rectangle(&mut self, points: [Vec2; 4]) {

        let n = self.positions.len() as u32;

        self.positions.push(to_arr(points[0]));
        self.positions.push(to_arr(points[1]));
        self.positions.push(to_arr(points[2]));
        self.positions.push(to_arr(points[3]));

        for _ in 0..4 {
            self.normals.push([0.0, 0.0, 1.0]);
            self.uvs.push([0.0, 0.0]);
        }

        self.indices.extend_from_slice(&[n, n + 1, n + 2, n, n + 2, n + 3]);

        self.colors.extend_from_slice(&[
            self.color.to_f32_array(),
            self.color.to_f32_array(),
            self.color.to_f32_array(),
            self.color.to_f32_array(),
        ]);
    }

    pub fn pentagon(&mut self, points: [Vec2; 5]) {
        let n = self.positions.len() as u32;

        self.positions.push(to_arr(points[0]));
        self.positions.push(to_arr(points[1]));
        self.positions.push(to_arr(points[2]));
        self.positions.push(to_arr(points[3]));
        self.positions.push(to_arr(points[4]));

        for _ in 0..5 {
            self.normals.push([0.0, 0.0, 1.0]);
            self.uvs.push([0.0, 0.0]);
        }

        self.indices.push(n);
        self.indices.push(n + 1);
        self.indices.push(n + 2);

        self.indices.push(n);
        self.indices.push(n + 2);
        self.indices.push(n + 3);

        self.indices.push(n);
        self.indices.push(n + 3);
        self.indices.push(n + 4);

        let color = RED;

        self.colors.extend_from_slice(&[
            self.color.to_f32_array(),
            self.color.to_f32_array(),
            self.color.to_f32_array(),
            self.color.to_f32_array(),
            self.color.to_f32_array(),
        ]);
    }
}
