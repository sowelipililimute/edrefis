mod sounds;

use crate::sounds::ClientSounds;
use client::input::KeyboardInputs;
use client::{app::App, gpu};
use logic::{input::Input, well::WELL_COLS};
use sdl::{event::Event, event::WindowEvent, keyboard::Keycode};
use sdl3::{self as sdl};
use std::time::Duration;
use thiserror::Error;

fn input_to_sdl_key(keycode: Input) -> Keycode {
    match keycode {
        Input::Up => Keycode::Up,
        Input::Down => Keycode::Down,
        Input::Left => Keycode::Left,
        Input::Right => Keycode::Right,
        Input::CW => Keycode::X,
        Input::CCW => Keycode::Z,
        Input::CW2 => Keycode::V,
        Input::CCW2 => Keycode::C,
        Input::DebugLevel => Keycode::A,
    }
}

#[derive(Error, Debug)]
pub enum NativeAppError {
    #[error("SDL error")]
    SDLError(#[from] sdl::Error),
    #[error("window error")]
    WindowError(#[from] sdl::video::WindowBuildError),
    #[error("graphics error")]
    GraphicsError(#[from] gpu::context::ContextError),
}

pub fn main() -> Result<(), NativeAppError> {
    let ctx = sdl::init()?;

    let video = ctx.video()?;
    let _audio = ctx.audio()?;

    let mixer = sdl::mixer::Mixer::open_device(None)?;

    let window = video
        .window("Edrefis", WELL_COLS as u32 * 60, WELL_COLS as u32 * 60)
        .position_centered()
        .resizable()
        .high_pixel_density()
        .metal_view()
        .build()?;

    let (width, height) = window.size_in_pixels();

    let gpu_state = pollster::block_on(gpu::context::Context::new(
        width,
        height,
        |instance| unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap())
        },
        Box::new(|error| eprintln!("Unhandled GPU error {error}")),
    ))?;

    let mut app = App::new(gpu_state, window.display_scale())?;
    let mut input_provider = KeyboardInputs::new(input_to_sdl_key);

    let mut event_pump = ctx.event_pump()?;
    let mut sounds = ClientSounds::new(&mixer)?;

    let mut stepper = nanotime::StepData::new(Duration::from_secs_f64(1. / 60.));

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Window {
                    window_id,
                    win_event: WindowEvent::PixelSizeChanged(width, height),
                    ..
                } if window_id == window.id() => {
                    app.resize(width as u32, height as u32, window.display_scale())
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => input_provider.push_key(key),
                Event::KeyUp {
                    keycode: Some(key), ..
                } => input_provider.release_key(&key),
                Event::Quit { .. } => {
                    break 'running;
                }
                _ => {}
            }
        }

        app.tick(&mut input_provider, &mut sounds);
        app.render_world()?;

        stepper.step();
    }

    Ok(())
}
