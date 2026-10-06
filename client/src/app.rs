use logic::hooks::Sounds;

use crate::{
    gpu::{
        context::{Context, ContextError},
        text::TextRenderer,
    },
    graphics::Graphics,
    input::ClientInputs,
    scene::{DrawContext, Scene, Transition, menu::Menu},
};

pub struct App<'surface> {
    gpu: Context<'surface>,
    draw: DrawContext,
    scenes: Vec<Box<dyn Scene>>,
}

impl<'surface> App<'surface> {
    pub fn new(gpu: Context<'surface>) -> Result<App<'surface>, ContextError> {
        let mut text = TextRenderer::new(&gpu);
        let graphics = Graphics::new(&gpu, &mut text)?;
        let mut draw = DrawContext { graphics, text };

        Ok(App {
            scenes: vec![Box::new(Menu::new(&mut draw))],
            gpu,
            draw,
        })
    }

    pub fn tick(self: &mut App<'surface>, inputs: &mut dyn ClientInputs, sounds: &mut dyn Sounds) {
        for idx in (0..self.scenes.len()).rev() {
            let transition = self.scenes[idx].tick(inputs, sounds);
            let cont = self.scenes[idx].tick_anterior();
            match transition {
                Transition::None => {}
                Transition::Push(scene) => {
                    if idx == self.scenes.len() - 1 {
                        self.scenes.push(scene);
                    } else {
                        self.scenes.splice(idx + 1..idx + 1, [scene]);
                    }
                }
                Transition::Pop => {
                    self.scenes.remove(idx);
                }
                Transition::Replace(scene) => {
                    self.scenes[idx] = scene;
                }
                Transition::Reset(scene) => {
                    self.scenes.clear();
                    self.scenes.push(scene);
                    break;
                }
                Transition::Quit => {
                    panic!("i did not implement quitting");
                }
            }
            if !cont {
                break;
            }
        }
    }

    pub fn render_world(self: &mut App<'surface>) -> Result<(), ContextError> {
        let mut frame = self.gpu.frame()?;

        for scene in self.scenes.iter_mut().rev() {
            scene.draw(&self.gpu, &mut self.draw, &mut frame)?;
            if !scene.draw_anterior() {
                break;
            }
        }

        frame.present();
        Ok(())
    }

    pub fn resize(self: &mut App<'surface>, width: u32, height: u32) {
        self.gpu.resize(width, height)
    }
}
