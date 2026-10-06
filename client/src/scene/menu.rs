use logic::{hooks::Sounds, input::Input};

use crate::{
    gpu::{
        context::{Context, ContextError},
        frame::{Frame, RenderTarget},
    },
    input::ClientInputs,
    scene::{DrawContext, Scene, Transition, game::Game},
    ui::{label::Label, widget_container::WidgetContainer},
};

pub struct Menu {
    container: WidgetContainer,
}

impl Menu {
    pub fn new(draw: &mut DrawContext) -> Result<Menu, taffy::TaffyError> {
        let attrs = glyphon::Attrs::new()
            .family(glyphon::Family::Name("Hanken Grotesk"))
            .weight(glyphon::Weight::MEDIUM)
            .color(glyphon::Color::rgba(255, 255, 255, 180));

        let mut container = WidgetContainer::new()?;
        let mut label = Label::new(&mut draw.text);
        label.set_text(
            &mut draw.text,
            [(
                "Edrefis\n",
                attrs.metrics(glyphon::Metrics::relative(24., 1.2)),
            )],
            attrs,
        );
        container.add(container.root(), label)?;

        Ok(Menu { container })
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
        gpu: &Context<'_>,
        draw: &mut DrawContext,
        frame: &mut Frame<'_, '_>,
    ) -> Result<(), ContextError> {
        let mut pass = frame.pass(RenderTarget::Screen, Some(wgpu::Color::BLACK));
        self.container.draw(gpu, draw, &mut pass)?;

        Ok(())
    }
}
