// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use std::panic::{self, PanicHookInfo};

use client::{app::App, gpu::Context, input::KeyboardInputs};
use logic::{hooks::NoopHooks, input::Input};
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::{HtmlCanvasElement, console};
use wgpu::SurfaceTarget;

#[wasm_bindgen]
pub struct WebApp {
    app: App<'static>,
    input_provider: KeyboardInputs<String>,
}

fn input_to_web_code(key: Input) -> &'static str {
    match key {
        Input::Up => "ArrowUp",
        Input::Down => "ArrowDown",
        Input::Left => "ArrowLeft",
        Input::Right => "ArrowRight",
        Input::CW => "KeyX",
        Input::CCW => "KeyZ",
        Input::CW2 => "KeyV",
        Input::CCW2 => "KeyC",
        Input::DebugLevel => "KeyA",
    }
}

impl WebApp {
    pub async fn new(canvas: HtmlCanvasElement) -> Result<WebApp, String> {
        let gpu = Context::new(
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

        let app = App::new(gpu).map_err(|e| format!("failed to load app: {e}"))?;

        Ok(WebApp {
            app,
            input_provider: KeyboardInputs::new(|input| input_to_web_code(input).to_string()),
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
        self.app
            .tick(&mut self.input_provider, &mut NoopHooks, &mut NoopHooks);
    }
    pub fn draw(&mut self) -> Result<(), String> {
        self.app.render_world()
    }
    pub fn key_down(&mut self, event: web_sys::KeyboardEvent) {
        self.input_provider.push_key(event.code());
    }
    pub fn key_up(&mut self, event: web_sys::KeyboardEvent) {
        self.input_provider.release_key(&event.code());
    }
}
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(msg: String);

    type Error;

    #[wasm_bindgen(constructor)]
    fn new() -> Error;

    #[wasm_bindgen(structural, method, getter)]
    fn stack(error: &Error) -> String;
}

fn hook_impl(info: &PanicHookInfo) {
    let mut msg = info.to_string();

    msg.push_str("\n\nStack:\n\n");
    let e = Error::new();
    let stack = e.stack();
    msg.push_str(&stack);
    msg.push_str("\n\n");

    console::error_1(&msg.into());
}

#[wasm_bindgen]
pub async fn new_app(canvas: web_sys::HtmlCanvasElement) -> Result<WebApp, String> {
    panic::set_hook(Box::new(hook_impl));

    WebApp::new(canvas).await
}
