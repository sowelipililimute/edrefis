use std::rc::Rc;

use glam::{Mat4, Vec2, Vec3, Vec3Swizzles};
use wgpu::util::DeviceExt;

use crate::gpu::{
    camera::Camera,
    context::{Context, Texture},
    geometry::AVertex,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct MatrixUniform {
    matrix: [[f32; 4]; 4],
}

impl MatrixUniform {
    fn from(mtx: &Mat4) -> MatrixUniform {
        MatrixUniform {
            matrix: (*mtx).to_cols_array_2d(),
        }
    }
}

pub struct Pass<'context, 'surface, 'frame> {
    pub pass: wgpu::RenderPass<'frame>,
    pub ctx: &'context Context<'surface>,

    vertices: Vec<AVertex>,
    indices: Vec<u16>,
    camera_matrix: Mat4,
    active_bind_group: Rc<wgpu::BindGroup>,
}

impl<'context, 'surface, 'frame> Pass<'context, 'surface, 'frame> {
    pub fn new(
        pass: wgpu::RenderPass<'frame>,
        ctx: &'context Context<'surface>,
    ) -> Pass<'context, 'surface, 'frame> {
        Pass {
            pass,
            ctx,
            vertices: Vec::new(),
            indices: Vec::new(),
            camera_matrix: Mat4::IDENTITY,
            active_bind_group: ctx.white.bind_group.clone(),
        }
    }
    pub fn queue_draw<const V: usize, const I: usize>(&mut self, data: ([AVertex; V], [u16; I])) {
        let (v, i) = data;
        let count = self.vertices.len() as u16;
        self.indices.extend(i.iter().map(|x| *x + count));
        self.vertices.extend_from_slice(&v);
    }
    pub fn set_camera(&mut self, camera: &dyn Camera) {
        self.camera_matrix = camera.matrix(&self.ctx.config);
    }
    pub fn set_texture(&mut self, texture: Option<&Texture>) {
        self.active_bind_group = texture.unwrap_or(&self.ctx.white).bind_group.clone();
    }
    pub fn do_draw(&mut self) -> Result<(), String> {
        if self.vertices.is_empty() {
            return Ok(());
        }

        let matrix = MatrixUniform::from(&self.camera_matrix);

        let matrix_buffer = self
            .ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Matrix Buffer"),
                contents: bytemuck::cast_slice(&[matrix]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let matrix_bind_group = self
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.ctx.layouts.camera,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: matrix_buffer.as_entire_binding(),
                }],
                label: Some("matrix_bind_group"),
            });

        let vertex_buffer = self
            .ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Well Vertex Buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Well Index Buffer"),
                contents: bytemuck::cast_slice(&self.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        let num_indices = self.indices.len() as u32;

        self.pass
            .set_bind_group(0, self.active_bind_group.as_ref(), &[]);
        self.pass.set_bind_group(1, &matrix_bind_group, &[]);
        self.pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        self.pass
            .set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        self.pass.draw_indexed(0..num_indices, 0, 0..1);

        self.vertices.clear();
        self.indices.clear();

        Ok(())
    }

    pub fn world_to_view(&self, point: Vec3) -> Vec2 {
        let transformed = self.camera_matrix.project_point3(point).xy() / Vec2::new(2., -2.)
            + Vec2::new(0.5, 0.5);
        let screen_size = Vec2::new(self.ctx.config.width as f32, self.ctx.config.height as f32);

        transformed * screen_size
    }
}
