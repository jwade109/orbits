use crate::{
    event_bus::{EventBus, TrainEvent},
    world::World,
};
use bary_core::prelude::{Components, Ent, EntitySpawner};
use log::*;
use rend::*;
use std::{collections::BTreeMap, rc::Rc};

pub struct RenderWorld {
    pub fonts: Components<(FontInfo, Texture)>,
    pub meshes: Components<Mesh>,
    pub textures: Components<Texture>,
    pub memory: Components<BufferResource>,

    pub rect_data: RectDataBuffer,
    pub height_data_chunks: BufferResource,
    pub invincible: Texture,

    spawner: EntitySpawner,
}

impl RenderWorld {
    pub fn new(rect: RectDataBuffer, height: BufferResource, invincible: Texture) -> Self {
        Self {
            fonts: Components::default(),
            meshes: Components::default(),
            textures: Components::default(),
            memory: Components::default(),
            rect_data: rect,
            height_data_chunks: height,
            invincible,
            spawner: EntitySpawner::default(),
        }
    }

    pub fn spawn_mesh(&mut self, mesh: Mesh) -> Ent {
        let id = self.spawner.spawn();
        self.meshes.spawn(id, mesh);
        id
    }

    pub fn load_texture(&mut self, rd: &Renderer, path: &str) -> Ent {
        let sprite = Texture::load_sprite(path, rd).unwrap();
        let id = self.spawner.spawn();
        self.textures.spawn(id, sprite);
        id
    }

    pub fn load_font(&mut self, rd: &Renderer, name: &str) -> Ent {
        let data_path = format!("assets/font_textures/{name}/font_data.json");
        let texture_path = format!("assets/font_textures/{name}/font.png");
        let texture = Texture::load_sprite(&texture_path, rd).unwrap();
        let font = FontInfo::from_file(&data_path).unwrap();
        let id = self.spawner.spawn();
        self.fonts.spawn(id, (font, texture));
        id
    }

    pub fn create_memory_arena(
        &mut self,
        rd: &Renderer,
        name: impl Into<String>,
        size: usize,
    ) -> Ent {
        let name = name.into();
        let resource = BufferResource::new(&rd.device, size, &name);
        let id = self.spawner.spawn();
        self.memory.spawn(id, resource);
        id
    }

    pub fn handle_events(&mut self, rd: &Renderer, events: &EventBus, world: &mut World) {
        for event in events.iter() {
            if let TrainEvent::ChunkUpdate(id) = event {
                // warn!("Chunk update: {id}");

                // let Ok(chunk) = world.chunks.try_get_mut(*id) else {
                //     continue;
                // };

                // if chunk.gpu_data().is_some() {
                //     continue;
                // }

                // warn!("Spawning mesh for chunk {:?}", chunk.index());
                // let mesh = make_rough_ground_plane(&rd.device, chunk.isometry().tr(), 10);
                // let id = self.spawn_mesh(mesh);
                // chunk.set_gpu_data(id);

                // if let Ok(chunk) = world.chunks.try_get_mut(*id) {
                //     if let Some(id) = chunk.gpu_data() {
                //         info!("Already has GPU data at buffer {id}");
                //     } else {
                //         let name = format!("chunk_{}_data", id);
                //         let gpu_data = self.create_memory_arena(rd, name, 1000);
                //         chunk.set_gpu_data(gpu_data);
                //     }
                // }
            }
        }
    }
}
