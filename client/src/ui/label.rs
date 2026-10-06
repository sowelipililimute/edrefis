use crate::{
    gpu::{
        context::{Context, ContextError},
        pass::Pass,
        text::TextRenderer,
    },
    scene::DrawContext,
    ui::widget::Widget,
};

pub struct Label {
    buffer: glyphon::Buffer,
    style: taffy::Style,
}

impl Label {
    pub fn new(text: &mut TextRenderer) -> Label {
        Label {
            buffer: text.create_buffer(),
            style: taffy::Style::default(),
        }
    }
    pub fn set_text<'r, 's>(
        &mut self,
        text: &mut TextRenderer,
        spans: impl IntoIterator<Item = (&'s str, glyphon::Attrs<'r>)>,
        default: glyphon::Attrs,
    ) {
        text.set_buffer_text(&mut self.buffer, spans, default);
    }
}

impl Widget for Label {
    fn style(&self) -> taffy::Style {
        self.style.clone()
    }

    fn measure(
        &mut self,
        known: taffy::Size<Option<f32>>,
        available: taffy::Size<taffy::AvailableSpace>,
        text: &mut TextRenderer,
    ) -> taffy::Size<f32> {
        let max_w = known.width.or(match available.width {
            taffy::AvailableSpace::Definite(w) => Some(w),
            taffy::AvailableSpace::MinContent => Some(0.),
            taffy::AvailableSpace::MaxContent => None,
        });
        let px = text.measure(&mut self.buffer, max_w);
        taffy::Size {
            width: px.x,
            height: px.y,
        }
    }

    fn draw(
        &mut self,
        rect: taffy::Rect<f32>,
        _gpu: &Context<'_>,
        draw: &mut DrawContext,
        pass: &mut Pass<'_, '_, '_>,
    ) -> Result<(), ContextError> {
        draw.text.draw_text(
            pass,
            &mut self.buffer,
            glam::Vec2::new(rect.left, rect.top),
            draw.scale,
        )?;

        Ok(())
    }
}
