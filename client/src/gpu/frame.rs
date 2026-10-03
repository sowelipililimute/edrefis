use crate::gpu::{
    context::{Context, Texture},
    pass::Pass,
};

pub enum RenderTarget<'context> {
    Screen,
    Texture(&'context Texture),
}

pub struct Frame<'context, 'surface> {
    ctx: &'context Context<'surface>,
    surface: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
}

impl<'context, 'surface> Frame<'context, 'surface> {
    pub fn new(
        ctx: &'context Context<'surface>,
        surface: wgpu::SurfaceTexture,
        view: wgpu::TextureView,
        encoder: wgpu::CommandEncoder,
    ) -> Frame<'context, 'surface> {
        Frame {
            ctx,
            surface,
            view,
            encoder,
        }
    }
    pub fn pass<'frame>(
        &'frame mut self,
        target: RenderTarget,
        clear: Option<wgpu::Color>,
    ) -> Pass<'context, 'surface, 'frame> {
        let target_view: &wgpu::TextureView = match target {
            RenderTarget::Texture(texture) => texture.view.as_ref(),
            RenderTarget::Screen => &self.view,
        };

        let mut pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: clear.map(wgpu::LoadOp::Clear).unwrap_or(wgpu::LoadOp::Load),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            label: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.ctx.render_pipeline);
        Pass::new(pass, self.ctx)
    }
    pub fn present(self) {
        self.ctx
            .queue
            .submit(std::iter::once(self.encoder.finish()));
        self.surface.present();
    }
}
