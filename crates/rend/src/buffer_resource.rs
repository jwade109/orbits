use wgpu::*;

pub struct BufferResource {
    buffer: Buffer,
    bind_group: BindGroup,
}

impl BufferResource {
    pub fn make_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("BufferResource"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::all(),
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }

    pub fn make_descriptor(size: usize) -> BufferDescriptor<'static> {
        BufferDescriptor {
            label: Some("BufferResource"),
            size: size.try_into().unwrap(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }
    }

    pub fn make_bind_group<'a>(device: &Device, buffer: &Buffer) -> BindGroup {
        let layout = Self::make_layout(device);
        let bgd = BindGroupDescriptor {
            label: Some("Color array bind group"),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        };

        device.create_bind_group(&bgd)
    }

    pub fn new(device: &Device, size: usize, label: &str) -> Self {
        println!("{label:40} >> Allocating buffer with {size} bytes");

        let bgl = Self::make_layout(device);
        let bd = Self::make_descriptor(size);
        let buffer = device.create_buffer(&bd);
        let bg = Self::make_bind_group(device, &buffer);

        BufferResource {
            buffer,
            bind_group: bg,
        }
    }

    pub fn new_array(device: &Device, n_elements: usize, elem_size: usize, label: &str) -> Self {
        let size = n_elements * elem_size;
        Self::new(device, size, label)
    }

    pub fn write(&self, queue: &Queue, data: &[u8]) {
        queue.write_buffer(&self.buffer, 0, data);
    }

    pub fn upload(&self, queue: &Queue, offset: u64, data: &[u8]) {
        queue.write_buffer(&self.buffer, offset, data);
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }
}
