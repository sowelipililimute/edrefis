use std::any::Any;

use crate::{
    gpu::{
        context::{Context, ContextError},
        pass::Pass,
        text::TextRenderer,
    },
    scene::DrawContext,
};

pub trait Widget: Any {
    fn style(&self) -> taffy::Style;
    fn measure(
        &mut self,
        known: taffy::Size<Option<f32>>,
        available: taffy::Size<taffy::AvailableSpace>,
        text: &mut TextRenderer,
    ) -> taffy::Size<f32>;
    fn draw(
        &mut self,
        rect: taffy::Rect<f32>,
        gpu: &Context<'_>,
        draw: &mut DrawContext,
        pass: &mut Pass<'_, '_, '_>,
    ) -> Result<(), ContextError>;
}
