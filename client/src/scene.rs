use logic::hooks::Sounds;

use crate::{
    gpu::{
        context::{Context, ContextError},
        frame::Frame,
        text::TextRenderer,
    },
    graphics::Graphics,
    input::ClientInputs,
};

pub enum Transition {
    None,
    Push(Box<dyn Scene>),
    Pop,
    Replace(Box<dyn Scene>),
    Reset(Box<dyn Scene>),
    Quit,
}

pub struct DrawContext {
    pub graphics: Graphics,
    pub text: TextRenderer,
}

pub trait Scene {
    fn tick(&mut self, inputs: &mut dyn ClientInputs, sounds: &mut dyn Sounds) -> Transition;
    fn draw(
        &mut self,
        gpu: &Context<'_>,
        draw: &mut DrawContext,
        frame: &mut Frame<'_, '_>,
    ) -> Result<(), ContextError>;

    fn tick_anterior(&self) -> bool {
        false
    }
    fn draw_anterior(&self) -> bool {
        false
    }
}

pub mod game;
pub mod menu;
