// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use hecs::Entity;
use nanoserde::{DeJson, SerJson};

use crate::{
    input::{FrameInputs, Input, InputState, PendingInput},
    net::World,
    piece::Piece,
    randomizer::Randomizer,
    well::Well,
};

#[derive(Copy, Clone, Eq, PartialEq, Hash, SerJson, DeJson)]
pub struct Level(pub u32);

#[derive(Debug, Clone, SerJson, DeJson)]
pub enum GameState {
    ActivePiece {
        piece: Piece,
    },
    ClearDelay {
        ticks_remaining: i32,
        rows_to_lower: Vec<i32>,
    },
    PlaceDelay {
        ticks_remaining: i32,
    },
    GameOver {
        ticks_remaining: i32,
    },
}

pub fn level_to_gravity(level: u32) -> i32 {
    if level >= 500 {
        5120
    } else if level >= 450 {
        768
    } else if level >= 420 {
        1024
    } else if level >= 400 {
        1280
    } else if level >= 360 {
        1024
    } else if level >= 330 {
        768
    } else if level >= 300 {
        512
    } else if level >= 251 {
        256
    } else if level >= 247 {
        224
    } else if level >= 243 {
        192
    } else if level >= 239 {
        160
    } else if level >= 236 {
        128
    } else if level >= 233 {
        96
    } else if level >= 230 {
        64
    } else if level >= 220 {
        32
    } else if level >= 200 {
        4
    } else if level >= 170 {
        144
    } else if level >= 160 {
        128
    } else if level >= 140 {
        112
    } else if level >= 120 {
        96
    } else if level >= 100 {
        80
    } else if level >= 90 {
        64
    } else if level >= 80 {
        48
    } else if level >= 70 {
        32
    } else if level >= 60 {
        16
    } else if level >= 50 {
        12
    } else if level >= 40 {
        10
    } else if level >= 35 {
        8
    } else if level >= 30 {
        6
    } else {
        4
    }
}

pub fn spawn_field<W: World>(world: &mut W) -> Entity {
    let mut randomizer = Randomizer::new();

    world.spawn((
        Well::new(),
        randomizer.next_piece(),
        Level(0u32),
        GameState::ActivePiece {
            piece: randomizer.next_piece(),
        },
        randomizer,
        InputState::default(),
        PendingInput(FrameInputs::default()),
    ))
}

pub fn set_input<W: World>(world: &mut W, field: Entity, frame: FrameInputs) {
    if let Ok(mut p) = world.get::<&mut PendingInput>(field) {
        p.0 = frame;
    }
}

pub fn field_system<W: World>(world: &mut W) {
    let tick = world.tick();
    for (uid, well, next, level, state, randomizer, inputs, pending_input) in world
        .query::<(
            Entity,
            &mut Well,
            &mut Piece,
            &mut Level,
            &mut GameState,
            &mut Randomizer,
            &mut InputState,
            &PendingInput,
        )>()
        .iter()
    {
        inputs.tick(tick, pending_input.0);
        world.dirty::<&InputState>(uid);

        if inputs.just_pressed(Input::DebugLevel) {
            level.0 += 50;
            world.dirty::<&Level>(uid);
        }

        world.dirty::<&GameState>(uid);
        match state {
            GameState::ActivePiece { ref mut piece } => {
                piece.do_sonic(&well, inputs);
                piece.do_rotate(&well, inputs);
                piece.do_horizontal(&well, inputs);
                piece.do_gravity(&well, inputs, level_to_gravity(level.0), true);

                if piece.do_lock(well, inputs) {
                    world.dirty::<&Well>(uid);

                    let cleared_rows = well.do_clear();
                    if cleared_rows.len() > 0 {
                        // sounds.line_clear();
                        level.0 += cleared_rows.len() as u32;
                        world.dirty::<&Level>(uid);

                        let ticks_of_line_clear = 41;
                        let rows_to_lower = cleared_rows.iter().map(|x| x.0).collect::<Vec<i32>>();

                        *state = GameState::ClearDelay {
                            ticks_remaining: ticks_of_line_clear,
                            rows_to_lower,
                        };
                    } else {
                        *state = GameState::PlaceDelay {
                            ticks_remaining: 30,
                        };
                    }
                }
            }
            GameState::ClearDelay {
                ref mut ticks_remaining,
                ref mut rows_to_lower,
            } => {
                *ticks_remaining -= 1;

                if *ticks_remaining == 0 {
                    well.commit_clear(rows_to_lower);
                    *state = GameState::PlaceDelay {
                        ticks_remaining: 30,
                    };

                    world.dirty::<&Well>(uid);
                }
            }
            GameState::PlaceDelay {
                ref mut ticks_remaining,
            } => {
                *ticks_remaining -= 1;
                if *ticks_remaining == 0 {
                    if inputs.pressed(Input::CW) || inputs.pressed(Input::CW2) {
                        next.rotation = next.rotation.cw();
                    } else if inputs.pressed(Input::CCW) || inputs.pressed(Input::CCW2) {
                        next.rotation = next.rotation.ccw();
                    }
                    world.dirty::<&Piece>(uid);

                    if level.0 % 100 != 99 {
                        level.0 += 1;
                    }
                    if next.collides_with(&well, 0, 0, next.rotation) {
                        *state = GameState::GameOver {
                            ticks_remaining: 60 * 5,
                        };
                    } else {
                        next.do_gravity(&well, inputs, level_to_gravity(level.0), false);
                        *state = GameState::ActivePiece { piece: *next };
                        *next = randomizer.next_piece();
                        world.dirty::<&Piece>(uid);
                        world.dirty::<&Randomizer>(uid);
                        // sounds.block_spawn(next.color);
                    }
                }
            }
            GameState::GameOver {
                ref mut ticks_remaining,
            } => {
                *ticks_remaining -= 1;

                if *ticks_remaining == 0 {
                    let mut new_randomizer = Randomizer::new();
                    *well = Well::new();
                    *next = new_randomizer.next_piece();
                    *state = GameState::ActivePiece {
                        piece: new_randomizer.next_piece(),
                    };
                    *randomizer = new_randomizer;
                    level.0 = 0;

                    world.dirty::<(&Piece, &Well, &GameState, &Randomizer, &Level)>(uid);
                }
            }
        }
    }
}
