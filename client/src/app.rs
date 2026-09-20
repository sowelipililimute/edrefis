use logic::{
    field::{GameState, field_system, spawn_field},
    hooks::{Cubes, Sounds},
    input::{InputProvider, Inputs},
    piece::Piece,
    well::Well,
};

use crate::{client::Client, gpu::State, graphics::Graphics};

pub struct App<'a> {
    client: Client,
    inputs: Inputs,
    ticks: u64,
    graphics: Graphics,
    gpu: State<'a>,
}

impl<'a> App<'a> {
    pub fn new(graphics: Graphics, gpu: State<'a>) -> App<'a> {
        let mut client = Client::new();
        spawn_field(&mut client.world);

        App {
            client,
            inputs: Inputs::new(),
            ticks: 0,
            graphics,
            gpu,
        }
    }

    pub fn tick(
        self: &mut App<'a>,
        input_provider: &mut dyn InputProvider,
        sounds: &mut dyn Sounds,
        cubes: &mut dyn Cubes,
    ) {
        self.ticks += 1;
        self.inputs.tick(self.ticks, input_provider);
        field_system(&mut self.client.world, &self.inputs, sounds, cubes);
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
