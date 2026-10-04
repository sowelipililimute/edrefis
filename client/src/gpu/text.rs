use crate::gpu::context::{Context, ContextError};
use crate::gpu::pass::Pass;
use glam::Vec2;
use glyphon::fontdb;
use std::sync::Arc;

pub struct TextRenderer {
    font_system: glyphon::FontSystem,
    swash_cache: glyphon::SwashCache,
    viewport: glyphon::Viewport,
    atlas: glyphon::TextAtlas,
    text_renderer: glyphon::TextRenderer,
}

impl TextRenderer {
    pub fn new(ctx: &Context) -> TextRenderer {
        let font_system = glyphon::FontSystem::new_with_fonts([
            fontdb::Source::Binary(Arc::new(include_bytes!("font/HankenGrotesk-Bold.ttf"))),
            fontdb::Source::Binary(Arc::new(include_bytes!("font/HankenGrotesk-Medium.ttf"))),
        ]);
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(&ctx.device);
        let viewport = glyphon::Viewport::new(&ctx.device, &cache);
        let mut atlas = glyphon::TextAtlas::new(&ctx.device, &ctx.queue, &cache, ctx.format);
        let text_renderer = glyphon::TextRenderer::new(
            &mut atlas,
            &ctx.device,
            wgpu::MultisampleState::default(),
            None,
        );

        TextRenderer {
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
        }
    }

    pub fn create_buffer(&mut self) -> glyphon::Buffer {
        let mut text_buffer =
            glyphon::Buffer::new(&mut self.font_system, glyphon::Metrics::new(30.0, 42.0));

        text_buffer.set_size(&mut self.font_system, None, None);

        text_buffer
    }
    pub fn set_buffer_text<'r, 's, I>(
        &mut self,
        buffer: &mut glyphon::Buffer,
        spans: I,
        default_attrs: glyphon::Attrs,
    ) where
        I: IntoIterator<Item = (&'s str, glyphon::Attrs<'r>)>,
    {
        buffer.set_rich_text(
            &mut self.font_system,
            spans,
            default_attrs,
            glyphon::Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);
    }
    pub fn draw_text(
        &mut self,
        pass: &mut Pass,
        buffer: &mut glyphon::Buffer,
        point: Vec2,
        scale: f32,
    ) -> Result<(), ContextError> {
        self.viewport.update(
            &pass.ctx.queue,
            glyphon::Resolution {
                width: pass.ctx.config.width,
                height: pass.ctx.config.height,
            },
        );
        self.text_renderer.prepare(
            &pass.ctx.device,
            &pass.ctx.queue,
            &mut self.font_system,
            &mut self.atlas,
            &mut self.viewport,
            [glyphon::TextArea {
                buffer,
                left: point.x,
                top: point.y,
                scale,
                bounds: glyphon::TextBounds {
                    left: 0,
                    top: 0,
                    right: pass.ctx.config.width as i32,
                    bottom: pass.ctx.config.height as i32,
                },
                default_color: glyphon::Color::rgb(255, 255, 255),
                custom_glyphs: &[],
            }],
            &mut self.swash_cache,
        )?;

        self.text_renderer
            .render(&self.atlas, &self.viewport, pass.raw())?;

        Ok(())
    }
}
