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
        frame::Frame,
    },
    input::ClientInputs,
    scene::{DrawContext, Scene, Transition},
};

pub struct Game {
    client: Client,
    field: Entity,
    ticks: u64,
}

impl Game {
    pub fn new() -> Game {
        let mut client = Client::new();
        let field = spawn_field(&mut client.world);

        Game {
            client,
            field,
            ticks: 0,
        }
    }
}

impl Scene for Game {
    fn tick(&mut self, inputs: &mut dyn ClientInputs, sounds: &mut dyn Sounds) -> Transition {
        self.ticks += 1;
        set_input(&mut self.client.world, self.field, inputs.sample());
        field_system(&mut self.client.world, self.ticks, sounds);

        Transition::None
    }

    fn draw(
        &mut self,
        _gpu: &Context<'_>,
        draw: &mut DrawContext,
        frame: &mut Frame<'_, '_>,
    ) -> Result<(), ContextError> {
        for (well, level, state, next) in self
            .client
            .world
            .query_mut::<(&Well, &u32, &GameState, &Piece)>()
        {
            match state {
                GameState::ActivePiece { piece, .. } => {
                    draw.graphics
                        .render(*level, well, Some(piece), next, frame, &mut draw.text)?
                }
                _ => draw
                    .graphics
                    .render(*level, well, None, next, frame, &mut draw.text)?,
            }
            break;
        }

        Ok(())
    }
}
