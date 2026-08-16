use crate::*;

pub struct Texture {
    pub bind_group: wgpu::BindGroup,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn blank_texture(rd: &Renderer, label: &str) -> Self {
        let bgl = material_bind_group_layout(&rd.device, label);

        let texture_descriptor = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: rd.config.width.max(1),
                height: rd.config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some(label),
            view_formats: &[wgpu::TextureFormat::Bgra8UnormSrgb],
        };

        let texture = rd.device.create_texture(&texture_descriptor);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = rd.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let mut builder = BindGroupBuilder::new(&rd.device);
        builder.set_layout(&bgl);
        builder.add_material(&view, &sampler);
        let bind_group = builder.build(label);

        Self {
            bind_group,
            texture,
            view,
            sampler,
        }
    }

    pub fn depth_texture(rd: &Renderer, label: &str) -> Self {
        let bgl = BindGroupLayoutBuilder::new(&rd.device).build(label);

        let size = wgpu::Extent3d {
            width: rd.config.width.max(1),
            height: rd.config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = rd.device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = rd.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        let mut builder = BindGroupBuilder::new(&rd.device);
        builder.set_layout(&bgl);
        // builder.add_material(&view, &sampler);
        let bind_group = builder.build(label);

        Self {
            texture,
            view,
            sampler,
            bind_group,
        }
    }
}
