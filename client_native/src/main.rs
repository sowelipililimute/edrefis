mod sounds;

use crate::sounds::ClientSounds;
use client::app::App;
use client::gpu;
use client::graphics::Graphics;
use client::input::KeyboardInputs;
use logic::{hooks::NoopHooks, input::Input, well::WELL_COLS};
use sdl::{event::Event, event::WindowEvent, keyboard::Keycode};
use sdl3::{self as sdl};
use std::time::Duration;

fn input_to_sdl_key(keycode: Input) -> Keycode {
    match keycode {
        Input::Up => Keycode::Up,
        Input::Down => Keycode::Down,
        Input::Left => Keycode::Left,
        Input::Right => Keycode::Right,
        Input::CW => Keycode::X,
        Input::CCW => Keycode::Z,
        Input::DebugLevel => Keycode::C,
    }
}

pub fn main() -> Result<(), String> {
    let ctx = sdl::init().map_err(|e| e.to_string())?;

    let video = ctx.video().map_err(|e| e.to_string())?;
    let _audio = ctx.audio().map_err(|e| e.to_string())?;

    // let frequency = 44_100;
    // let format = sdl::mixer::AUDIO_S16LSB;
    // let channels = sdl::mixer::DEFAULT_CHANNELS;
    // let chunk_size = 1_024;

    let mixer = sdl::mixer::Mixer::open_device(None).map_err(|e| e.to_string())?;

    let window = video
        .window("Edrefis", WELL_COLS as u32 * 60, WELL_COLS as u32 * 60)
        .position_centered()
        .resizable()
        .metal_view()
        .build()
        .map_err(|e| e.to_string())?;

    let (width, height) = window.size();

    let mut gpu_state = pollster::block_on(gpu::State::new(
        width,
        height,
        |instance| unsafe {
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap())
                .map_err(|e| e.to_string())
        },
        Box::new(|error| eprintln!("Unhandled GPU error {error}")),
    ))?;
    let graphics = Graphics::new(&mut gpu_state)?;

    let mut app = App::new(graphics, gpu_state);
    let mut input_provider = KeyboardInputs::new(input_to_sdl_key);

    let mut event_pump = ctx.event_pump().map_err(|e| e.to_string())?;
    let mut sounds = ClientSounds::new(&mixer).map_err(|e| e.to_string())?;

    let mut stepper = nanotime::StepData::new(Duration::from_secs_f64(1. / 60.));

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Window {
                    window_id,
                    win_event: WindowEvent::PixelSizeChanged(width, height),
                    ..
                } if window_id == window.id() => app.resize(width as u32, height as u32)?,
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

        app.tick(&mut input_provider, &mut sounds, &mut NoopHooks);
        app.render_world()?;

        stepper.step();
    }

    Ok(())
}
