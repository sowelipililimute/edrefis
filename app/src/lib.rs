// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

mod app;
pub mod client;
mod gpu;
mod graphics_gpu;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(target_family = "wasm")]
mod main_web;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
pub async fn new_app(canvas: web_sys::HtmlCanvasElement) -> Result<main_web::WebApp, String> {
    main_web::WebApp::new(canvas).await
}
