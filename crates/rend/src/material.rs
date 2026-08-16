use crate::*;
use image::GenericImageView;

#[derive(Debug)]
pub struct SpriteMaterial {
    size: (u32, u32),
    pub bind_group: wgpu::BindGroup,
    #[allow(unused)]
    pub texture: wgpu::Texture,
    #[allow(unused)]
    pub view: wgpu::TextureView,
    #[allow(unused)]
    sampler: wgpu::Sampler,
}

impl SpriteMaterial {
    pub fn load(filename: &str, rd: &Renderer) -> Option<Self> {
        SpriteMaterial::new(filename, &rd.device, &rd.queue, filename)
    }

    fn new(
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
    ) -> Option<Self> {
        let bgl = material_bind_group_layout(device, filename);

        let bytes = std::fs::read(filename).ok()?;
        let loaded_image = image::load_from_memory(&bytes).ok()?;
        let converted = loaded_image.to_rgba8();
        let size = loaded_image.dimensions();
        let texture_size = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        };

        // Create the texture
        let texture_descriptor = wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(label),
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        };
        let texture = device.create_texture(&texture_descriptor);

        // Upload to it
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &converted,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.0),
                rows_per_image: Some(size.1),
            },
            texture_size,
        );

        // Get a view of the texture
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Make a sampler
        let sampler_descriptor = wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        };

        let sampler = device.create_sampler(&sampler_descriptor);

        let mut builder = BindGroupBuilder::new(device);
        builder.set_layout(&bgl);
        builder.add_material(&view, &sampler);
        let bind_group = builder.build(label);

        Some(SpriteMaterial {
            size,
            texture,
            bind_group,
            view,
            sampler,
        })
    }

    pub fn get_sample_range(&self) -> TextureSampleRange {
        TextureSampleRange {
            origin_x: 0,
            origin_y: 0,
            sample_width: self.size.0,
            sample_height: self.size.1,
            image_width: self.size.0,
            image_height: self.size.1,
        }
    }
}
