use std::rc::Rc;

use glam::{Mat4, Vec2, Vec3, Vec3Swizzles};

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
    indices: Vec<u32>,
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
    pub fn queue_draw<const V: usize, const I: usize>(&mut self, data: ([AVertex; V], [u32; I])) {
        let (v, i) = data;
        let count = self.vertices.len() as u32;
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

        let mut buffers = self.ctx.buffers.borrow_mut();
        buffers.vertices.reserve(
            &self.ctx.device,
            (self.vertices.len() * size_of::<AVertex>()) as u64,
        );
        buffers.indices.reserve(
            &self.ctx.device,
            (self.indices.len() * size_of::<u32>()) as u64,
        );
        buffers.reserve_uniforms(
            &self.ctx.device,
            &self.ctx.layouts,
            size_of::<MatrixUniform>() as u64,
        );

        let matrix = MatrixUniform::from(&self.camera_matrix);

        let (uniform_start, _) = buffers
            .uniforms
            .push(&self.ctx.queue, bytemuck::cast_slice(&[matrix]));

        let (vertex_start, vertex_end) = buffers
            .vertices
            .push(&self.ctx.queue, bytemuck::cast_slice(&self.vertices));

        let (index_start, index_end) = buffers
            .indices
            .push(&self.ctx.queue, bytemuck::cast_slice(&self.indices));

        let num_indices = self.indices.len() as u32;

        self.pass
            .set_bind_group(0, self.active_bind_group.as_ref(), &[]);
        self.pass
            .set_bind_group(1, &buffers.uniform_bind_group, &[uniform_start as u32]);
        self.pass
            .set_vertex_buffer(0, buffers.vertices.buffer.slice(vertex_start..vertex_end));
        self.pass.set_index_buffer(
            buffers.indices.buffer.slice(index_start..index_end),
            wgpu::IndexFormat::Uint32,
        );
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
