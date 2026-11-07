use hecs::Entity;
use logic::{
    field::{GameState, Level, field_system, set_input, spawn_field},
    hooks::Sounds,
    net::World,
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
}

impl Game {
    pub fn new() -> Game {
        let mut client = Client::new();
        let field = spawn_field(&mut client.world);

        Game { client, field }
    }
}

impl Scene for Game {
    fn tick(&mut self, inputs: &mut dyn ClientInputs, _sounds: &mut dyn Sounds) -> Transition {
        self.client.world.increment_tick();
        set_input(&mut self.client.world, self.field, inputs.sample());
        field_system(&mut self.client.world);

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
            .query_mut::<(&Well, &Level, &GameState, &Piece)>()
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
