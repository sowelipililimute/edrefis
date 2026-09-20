// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use std::collections::HashSet;

use crate::{app::App, gpu::State, graphics_gpu::Graphics};
use logic::{
    hooks::{Cubes, Sounds},
    input::{Input, InputProvider},
};
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::{console, HtmlCanvasElement};
use wgpu::SurfaceTarget;

struct DummyImpl;
impl Cubes for DummyImpl {
    fn spawn_cube(&mut self, _x: i32, _y: i32, _color: logic::well::Block) {}
}
impl Sounds for DummyImpl {
    fn block_spawn(&mut self, _color: logic::well::Block) {}
    fn line_clear(&mut self) {}
    fn lock(&mut self) {}
    fn land(&mut self) {}
}

#[wasm_bindgen]
pub struct WebApp {
    app: App<'static>,
    input_provider: WebInputs,
}

fn input_to_web_code(key: Input) -> &'static str {
    match key {
        Input::Up => "ArrowUp",
        Input::Down => "ArrowDown",
        Input::Left => "ArrowLeft",
        Input::Right => "ArrowRight",
        Input::CW => "KeyX",
        Input::CCW => "KeyZ",
        Input::DebugLevel => "KeyC",
    }
}

struct WebInputs {
    just_pressed_key: HashSet<String>,
    current_key: HashSet<String>,
}

impl WebInputs {
    fn new() -> WebInputs {
        WebInputs {
            just_pressed_key: HashSet::new(),
            current_key: HashSet::new(),
        }
    }
    fn push_key(&mut self, keycode: String) {
        self.just_pressed_key.insert(keycode.clone());
        self.current_key.insert(keycode);
    }
    fn release_key(&mut self, keycode: String) {
        self.just_pressed_key.remove(&keycode);
        self.current_key.remove(&keycode);
    }
}

impl InputProvider for WebInputs {
    fn peek(&mut self) {}

    fn consume(&mut self) {
        self.just_pressed_key.clear();
    }

    fn key_just_pressed(&self, input: Input) -> bool {
        self.just_pressed_key.contains(input_to_web_code(input))
    }

    fn key_down(&self, input: Input) -> bool {
        self.current_key.contains(input_to_web_code(input))
    }
}

impl WebApp {
    pub async fn new(canvas: HtmlCanvasElement) -> Result<WebApp, String> {
        let mut gpu = State::new(
            canvas.width(),
            canvas.height(),
            |instance| {
                instance
                    .create_surface(SurfaceTarget::Canvas(canvas))
                    .map_err(|e| format!("failed to create instance for canvas: {}", e))
            },
            Box::new(|error| {
                let desc = error.to_string();
                let log = format!("Unhandled GPU error {desc}");
                console::error_1(&log.into());
            }),
        )
        .await
        .map_err(|e| format!("failed to set up gpu: {}", e))?;
        let graphics =
            Graphics::new(&mut gpu).map_err(|e| format!("failed to load graphics: {}", e))?;

        let app = App::new(graphics, gpu);

        Ok(WebApp {
            app,
            input_provider: WebInputs::new(),
        })
    }
}

#[wasm_bindgen]
impl WebApp {
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.app
            .resize(width, height)
            .map_err(|e| format!("failed to resize canvas: {}", e))
    }
    pub fn tick(&mut self) {
        let mut sounds = DummyImpl;
        let mut cubes = DummyImpl;

        self.app
            .tick(&mut self.input_provider, &mut sounds, &mut cubes);
    }
    pub fn draw(&mut self) -> Result<(), String> {
        self.app.render_world()
    }
    pub fn key_down(&mut self, event: web_sys::KeyboardEvent) {
        self.input_provider.push_key(event.code());
    }
    pub fn key_up(&mut self, event: web_sys::KeyboardEvent) {
        self.input_provider.release_key(event.code());
    }
}
