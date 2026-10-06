use glam::{Vec2, Vec3};
use logic::{hooks::Sounds, input::Input};

use crate::{
    gpu::{
        context::{Context, ContextError},
        frame::{Frame, RenderTarget},
    },
    graphics::tile_camera,
    input::ClientInputs,
    scene::{DrawContext, Scene, Transition, game::Game},
};

pub struct Menu {
    buffer: glyphon::Buffer,
}

impl Menu {
    pub fn new(draw: &mut DrawContext) -> Menu {
        let mut buffer = draw.text.create_buffer();

        let attrs = glyphon::Attrs::new()
            .family(glyphon::Family::Name("Hanken Grotesk"))
            .weight(glyphon::Weight::MEDIUM)
            .color(glyphon::Color::rgba(255, 255, 255, 180));

        draw.text.set_buffer_text(
            &mut buffer,
            [(
                "Edrefis\n",
                attrs.metrics(glyphon::Metrics::relative(24., 1.2)),
            )],
            attrs,
        );

        Menu { buffer }
    }
}

impl Scene for Menu {
    fn tick(&mut self, inputs: &mut dyn ClientInputs, _sounds: &mut dyn Sounds) -> Transition {
        let input = inputs.sample();
        if input.is_set(Input::Up) {
            Transition::Replace(Box::new(Game::new()))
        } else {
            Transition::None
        }
    }

    fn draw(
        &mut self,
        _gpu: &Context<'_>,
        draw: &mut DrawContext,
        frame: &mut Frame<'_, '_>,
    ) -> Result<(), ContextError> {
        let mut pass = frame.pass(RenderTarget::Screen, Some(wgpu::Color::BLACK));
        pass.set_camera(&tile_camera(Vec2::ZERO));
        let point = pass.world_to_view(Vec3::ZERO).round();

        let scale = tile_camera(Vec2::ZERO).tile_px(&pass.ctx.config) / 20.;
        draw.text
            .draw_text(&mut pass, &mut self.buffer, point, scale)?;

        Ok(())
    }
}
