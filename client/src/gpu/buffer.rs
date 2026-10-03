use wgpu::util::align_to;

use crate::gpu::context::Layouts;

pub struct GpuBuffers {
    pub vertices: GpuBuffer,
    pub indices: GpuBuffer,
    pub uniforms: GpuBuffer,
    pub uniform_bind_group: wgpu::BindGroup,
}

const INITIAL_UNIFORM_SIZE: u64 = 1 << 16;
const INITIAL_VERTEX_SIZE: u64 = 1 << 16;
const INITIAL_INDEX_SIZE: u64 = 1 << 16;

impl GpuBuffers {
    pub fn new(device: &wgpu::Device, layouts: &Layouts) -> GpuBuffers {
        let uniforms = GpuBuffer::new(
            INITIAL_UNIFORM_SIZE,
            "Uniform Buffer",
            device,
            wgpu::BufferUsages::UNIFORM,
            device.limits().min_uniform_buffer_offset_alignment as u64,
        );

        GpuBuffers {
            vertices: GpuBuffer::new(
                INITIAL_VERTEX_SIZE,
                "Vertex Buffer",
                device,
                wgpu::BufferUsages::VERTEX,
                4,
            ),
            indices: GpuBuffer::new(
                INITIAL_INDEX_SIZE,
                "Index Buffer",
                device,
                wgpu::BufferUsages::INDEX,
                4,
            ),
            uniform_bind_group: uniforms.make_uniform_bind_group(device, layouts),
            uniforms,
        }
    }

    pub fn reserve_uniforms(&mut self, device: &wgpu::Device, layouts: &Layouts, bytes: u64) {
        if self.uniforms.reserve(device, bytes) {
            self.uniform_bind_group = self.uniforms.make_uniform_bind_group(device, layouts);
        }
    }
}

pub struct GpuBuffer {
    pub buffer: wgpu::Buffer,
    size: u64,
    cursor: u64,
    usage: wgpu::BufferUsages,
    label: &'static str,
    alignment: u64,
}

impl GpuBuffer {
    pub fn new(
        size: u64,
        label: &'static str,
        device: &wgpu::Device,
        usages: wgpu::BufferUsages,
        alignment: u64,
    ) -> GpuBuffer {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: usages | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        GpuBuffer {
            buffer,
            size,
            cursor: 0,
            usage: usages | wgpu::BufferUsages::COPY_DST,
            label,
            alignment,
        }
    }

    pub fn reserve(&mut self, device: &wgpu::Device, bytes: u64) -> bool {
        let start = align_to(self.cursor, self.alignment);
        if start + bytes <= self.size {
            return false;
        }

        self.size = (self.size * 2).max(bytes.next_power_of_two());
        self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(self.label),
            size: self.size,
            usage: self.usage,
            mapped_at_creation: false,
        });
        self.cursor = 0;

        true
    }

    pub fn push(&mut self, queue: &wgpu::Queue, data: &[u8]) -> (u64, u64) {
        let start = align_to(self.cursor, self.alignment);
        queue.write_buffer(&self.buffer, start, data);
        self.cursor = start + data.len() as u64;
        (start, self.cursor)
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
    }

    pub fn make_uniform_bind_group(
        &self,
        device: &wgpu::Device,
        layouts: &Layouts,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &layouts.camera,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(64),
                }),
            }],
        })
    }
}
