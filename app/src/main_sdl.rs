use crate::gpu;
use crate::graphics_gpu::Graphics;
use crate::sounds_sdl::ClientSounds;
use hecs::World;
use logic::{
    field::{field_system, spawn_field, Field, GameState},
    hooks::Cubes,
    input::{Input, InputProvider, Inputs},
    well::WELL_COLS,
};
use sdl::{event::Event, event::WindowEvent, keyboard::Keycode};
use sdl3::{self as sdl};
use std::{collections::HashSet, time::Duration};

#[derive(Clone, Copy)]
struct DummyImpl;
impl Cubes for DummyImpl {
    fn spawn_cube(&mut self, _x: i32, _y: i32, _color: logic::well::Block) {}
}

struct SDLInputs {
    just_pressed_key: HashSet<Keycode>,
    current_key: HashSet<Keycode>,
}

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

impl SDLInputs {
    fn new() -> SDLInputs {
        SDLInputs {
            just_pressed_key: HashSet::new(),
            current_key: HashSet::new(),
        }
    }
    fn push_key(&mut self, keycode: Keycode) {
        self.just_pressed_key.insert(keycode);
        self.current_key.insert(keycode);
    }
    fn release_key(&mut self, keycode: Keycode) {
        self.just_pressed_key.remove(&keycode);
        self.current_key.remove(&keycode);
    }
}

impl InputProvider for SDLInputs {
    fn peek(&mut self) {}

    fn consume(&mut self) {
        self.just_pressed_key.clear();
    }

    fn key_just_pressed(&self, input: Input) -> bool {
        self.just_pressed_key.contains(&input_to_sdl_key(input))
    }

    fn key_down(&self, input: Input) -> bool {
        self.current_key.contains(&input_to_sdl_key(input))
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

    let mut gpu_state = pollster::block_on(gpu::State::new(width, height, |instance| unsafe {
        instance
            .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap())
            .map_err(|e| e.to_string())
    }))?;
    let mut graphics = Graphics::new(&mut gpu_state)?;

    let mut world = World::new();
    spawn_field(&mut world);
    let mut input_provider = SDLInputs::new();
    let mut inputs = Inputs::new();

    let mut event_pump = ctx.event_pump().map_err(|e| e.to_string())?;
    let mut sounds = ClientSounds::new(&mixer).map_err(|e| e.to_string())?;
    let mut cubes = DummyImpl {};

    let mut ticks = 0u64;

    let mut stepper = nanotime::StepData::new(Duration::from_secs_f64(1. / 60.));

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Window {
                    window_id,
                    win_event: WindowEvent::PixelSizeChanged(width, height),
                    ..
                } if window_id == window.id() => {
                    gpu_state.resize(width as u32, height as u32)?;
                }
                Event::KeyDown {
                    keycode:
                        Some(
                            x @ (Keycode::X
                            | Keycode::Z
                            | Keycode::Up
                            | Keycode::Down
                            | Keycode::Left
                            | Keycode::Right
                            | Keycode::C),
                        ),
                    ..
                } => {
                    input_provider.push_key(x);
                }
                Event::KeyUp {
                    keycode:
                        Some(
                            x @ (Keycode::X
                            | Keycode::Z
                            | Keycode::Up
                            | Keycode::Down
                            | Keycode::Left
                            | Keycode::Right
                            | Keycode::C),
                        ),
                    ..
                } => {
                    input_provider.release_key(x);
                }
                Event::Quit { .. } => {
                    break 'running;
                }
                _ => {}
            }
        }

        ticks += 1;
        inputs.tick(ticks, &mut input_provider);

        field_system(&mut world, &inputs, &mut sounds, &mut cubes);

        for field in world.query_mut::<&Field>() {
            match field.state {
                GameState::ActivePiece { piece, .. } => {
                    graphics.render(
                        &field,
                        &field.well,
                        Some(&piece),
                        &field.next,
                        &mut gpu_state,
                    )?;
                }
                _ => {
                    graphics.render(&field, &field.well, None, &field.next, &mut gpu_state)?;
                }
            }
        }

        stepper.step();
    }

    Ok(())
}
