use std::rc::Rc;

use glam::{Mat4, Vec2, Vec3, Vec3Swizzles};
use wgpu::util::align_to;

use crate::gpu::{
    camera::Camera,
    context::{Context, Texture},
    geometry::{AVertex, Mesh},
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
    pass: wgpu::RenderPass<'frame>,
    pub ctx: &'context Context<'surface>,

    current_texture: Rc<wgpu::BindGroup>,
    current_camera_matrix: Mat4,

    vertices: Vec<AVertex>,
    indices: Vec<u32>,
    batches: Vec<Batch>,
}

struct Batch {
    texture: Rc<wgpu::BindGroup>,
    camera_matrix: Mat4,
    vertex_start: usize,
    vertex_len: usize,
    index_start: usize,
    index_len: usize,
}

impl<'context, 'surface, 'frame> Pass<'context, 'surface, 'frame> {
    pub fn new(
        pass: wgpu::RenderPass<'frame>,
        ctx: &'context Context<'surface>,
    ) -> Pass<'context, 'surface, 'frame> {
        Pass {
            pass,
            ctx,
            current_camera_matrix: Mat4::IDENTITY,
            current_texture: ctx.white.bind_group.clone(),
            vertices: Vec::new(),
            indices: Vec::new(),
            batches: Vec::new(),
        }
    }
    pub fn set_camera(&mut self, camera: &dyn Camera) {
        self.current_camera_matrix = camera.matrix(&self.ctx.config);
    }
    pub fn set_texture(&mut self, texture: Option<&Texture>) {
        self.current_texture = texture.unwrap_or(&self.ctx.white).bind_group.clone();
    }
    fn batch_dirty(&mut self) -> bool {
        if let Some(it) = self.batches.last() {
            if !Rc::ptr_eq(&self.current_texture, &it.texture) {
                return true;
            }
            if it.camera_matrix != self.current_camera_matrix {
                return true;
            }

            return false;
        }

        return true;
    }
    fn push_batch(&mut self) {
        self.batches.push(Batch {
            texture: self.current_texture.clone(),
            camera_matrix: self.current_camera_matrix,
            vertex_start: self.vertices.len(),
            vertex_len: 0,
            index_start: self.indices.len(),
            index_len: 0,
        });
    }
    pub fn draw(&mut self, data: &dyn Mesh) {
        let v = data.vertices();
        let i = data.indices();
        if self.batch_dirty() {
            self.push_batch();
        }
        let batch = self.batches.last_mut().unwrap();

        self.vertices.extend(v);
        self.indices
            .extend(i.iter().map(|x| *x + batch.vertex_len as u32));

        batch.vertex_len += v.len();
        batch.index_len += i.len();
    }
    pub fn flush(&mut self) {
        if self.batches.is_empty() {
            return;
        }

        self.pass.set_pipeline(&self.ctx.render_pipeline);

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
            (self.batches.len()
                * align_to(
                    size_of::<MatrixUniform>(),
                    self.ctx.device.limits().min_uniform_buffer_offset_alignment as usize,
                )) as u64,
        );

        let (vertex_start, _) = buffers
            .vertices
            .push(&self.ctx.queue, bytemuck::cast_slice(&self.vertices));

        let (index_start, _) = buffers
            .indices
            .push(&self.ctx.queue, bytemuck::cast_slice(&self.indices));

        for batch in self.batches.iter() {
            let matrix = MatrixUniform::from(&batch.camera_matrix);

            let (uniform_start, _) = buffers
                .uniforms
                .push(&self.ctx.queue, bytemuck::cast_slice(&[matrix]));

            let vertex_slice = {
                let v0 = vertex_start as usize + batch.vertex_start * size_of::<AVertex>();
                let v1 = v0 + batch.vertex_len * size_of::<AVertex>();

                buffers.vertices.buffer.slice(v0 as u64..v1 as u64)
            };
            let index_slice = {
                let i0 = index_start as usize + batch.index_start * size_of::<u32>();
                let i1 = i0 + batch.index_len * size_of::<u32>();

                buffers.indices.buffer.slice(i0 as u64..i1 as u64)
            };

            self.pass.set_bind_group(0, batch.texture.as_ref(), &[]);
            self.pass
                .set_bind_group(1, &buffers.uniform_bind_group, &[uniform_start as u32]);

            self.pass.set_vertex_buffer(0, vertex_slice);
            self.pass
                .set_index_buffer(index_slice, wgpu::IndexFormat::Uint32);
            self.pass.draw_indexed(0..batch.index_len as u32, 0, 0..1);
        }

        self.vertices.clear();
        self.indices.clear();
        self.batches.clear();
    }
    pub fn world_to_view(&self, point: Vec3) -> Vec2 {
        let transformed = self.current_camera_matrix.project_point3(point).xy()
            / Vec2::new(2., -2.)
            + Vec2::new(0.5, 0.5);
        let screen_size = Vec2::new(self.ctx.config.width as f32, self.ctx.config.height as f32);

        transformed * screen_size
    }
    pub fn raw(&mut self) -> &mut wgpu::RenderPass<'frame> {
        self.flush();
        &mut self.pass
    }
}

impl<'context, 'surface, 'frame> Drop for Pass<'context, 'surface, 'frame> {
    fn drop(&mut self) {
        self.flush();
    }
}
