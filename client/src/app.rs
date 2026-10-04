use hecs::Entity;
use logic::{
    field::{GameState, field_system, set_input, spawn_field},
    hooks::Sounds,
    piece::Piece,
    well::Well,
};

use crate::{
    client::Client,
    gpu::{
        context::{Context, ContextError},
        text::TextRenderer,
    },
    graphics::Graphics,
    input::ClientInputs,
};

pub struct App<'surface> {
    client: Client,
    field: Entity,
    ticks: u64,
    graphics: Graphics,
    gpu: Context<'surface>,
    text: TextRenderer,
}

impl<'surface> App<'surface> {
    pub fn new(gpu: Context<'surface>) -> Result<App<'surface>, ContextError> {
        let mut client = Client::new();
        let field = spawn_field(&mut client.world);
        let mut text = TextRenderer::new(&gpu);
        let graphics = Graphics::new(&gpu, &mut text)?;

        Ok(App {
            client,
            field,
            ticks: 0,
            graphics,
            gpu,
            text,
        })
    }

    pub fn tick(self: &mut App<'surface>, inputs: &mut dyn ClientInputs, sounds: &mut dyn Sounds) {
        self.ticks += 1;
        set_input(&mut self.client.world, self.field, inputs.sample());
        field_system(&mut self.client.world, self.ticks, sounds);
    }

    pub fn render_world(self: &mut App<'surface>) -> Result<(), ContextError> {
        let mut frame = self.gpu.frame()?;

        for (well, level, state, next) in self
            .client
            .world
            .query_mut::<(&Well, &u32, &GameState, &Piece)>()
        {
            match state {
                GameState::ActivePiece { piece, .. } => self.graphics.render(
                    *level,
                    well,
                    Some(piece),
                    next,
                    &mut frame,
                    &mut self.text,
                )?,
                _ => self
                    .graphics
                    .render(*level, well, None, next, &mut frame, &mut self.text)?,
            }
            break;
        }

        frame.present();
        Ok(())
    }

    pub fn resize(self: &mut App<'surface>, width: u32, height: u32) {
        self.gpu.resize(width, height)
    }
}
