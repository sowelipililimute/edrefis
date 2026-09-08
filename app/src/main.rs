// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

mod client;
mod gpu;
mod graphics_gpu;

#[cfg(not(target_family = "wasm"))]
mod main_sdl;
#[cfg(not(target_family = "wasm"))]
mod sounds_sdl;

#[cfg(not(target_family = "wasm"))]
fn main() -> Result<(), String> {
    main_sdl::main()
}
