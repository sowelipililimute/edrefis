use hecs::Entity;
use logic::{
    field::{GameState, field_system, set_input, spawn_field},
    hooks::{Cubes, Sounds},
    piece::Piece,
    well::Well,
};

use crate::{client::Client, gpu::State, graphics::Graphics, input::ClientInputs};

pub struct App<'a> {
    client: Client,
    field: Entity,
    ticks: u64,
    graphics: Graphics,
    gpu: State<'a>,
}

impl<'a> App<'a> {
    pub fn new(graphics: Graphics, gpu: State<'a>) -> App<'a> {
        let mut client = Client::new();
        let field = spawn_field(&mut client.world);

        App {
            client,
            field,
            ticks: 0,
            graphics,
            gpu,
        }
    }

    pub fn tick(
        self: &mut App<'a>,
        inputs: &mut dyn ClientInputs,
        sounds: &mut dyn Sounds,
        cubes: &mut dyn Cubes,
    ) {
        self.ticks += 1;
        set_input(&mut self.client.world, self.field, inputs.sample());
        field_system(&mut self.client.world, self.ticks, sounds, cubes);
    }

    pub fn render_world(self: &mut App<'a>) -> Result<(), String> {
        for (well, level, state, next) in self
            .client
            .world
            .query_mut::<(&Well, &u32, &GameState, &Piece)>()
        {
            match state {
                GameState::ActivePiece { piece, .. } => {
                    self.graphics
                        .render(*level, well, Some(piece), next, &mut self.gpu)?
                }
                _ => self
                    .graphics
                    .render(*level, well, None, next, &mut self.gpu)?,
            }
        }
        Ok(())
    }

    pub fn resize(self: &mut App<'a>, width: u32, height: u32) -> Result<(), String> {
        self.gpu.resize(width, height)
    }
}
