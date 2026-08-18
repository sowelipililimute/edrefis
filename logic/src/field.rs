// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use hecs::{Entity, World};
use nanoserde::{DeJson, SerJson};

use crate::{
    hooks::{Cubes, Sounds},
    input::{Input, Inputs},
    piece::Piece,
    randomizer::Randomizer,
    well::Well,
};

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

#[derive(SerJson, DeJson, Clone)]
pub struct Field {
    pub randomizer: Randomizer,

    pub well: Well,
    pub next: Piece,
    pub level: u32,

    pub state: GameState,
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

pub fn spawn_field(world: &mut World) -> Entity {
    let mut randomizer = Randomizer::new();

    world.spawn((Field {
        well: Well::new(),
        next: randomizer.next_piece(),
        level: 0,
        state: GameState::ActivePiece {
            piece: randomizer.next_piece(),
        },

        randomizer,
    },))
}

pub fn field_system(
    world: &mut World,
    inputs: &Inputs,
    sounds: &mut dyn Sounds,
    cubes: &mut dyn Cubes,
) {
    for field in world.query_mut::<&mut Field>() {
        if inputs.key_just_pressed(Input::DebugLevel) {
            field.level += 50;
        }
        match field.state {
            GameState::ActivePiece { ref mut piece } => {
                piece.do_sonic(&field.well, inputs);
                piece.do_rotate(&field.well, inputs);
                piece.do_horizontal(&field.well, inputs);
                piece.do_gravity(
                    &field.well,
                    inputs,
                    level_to_gravity(field.level),
                    sounds,
                    true,
                );

                if piece.do_lock(&mut field.well, inputs, sounds) {
                    let cleared_rows = field.well.do_clear();
                    if cleared_rows.len() > 0 {
                        sounds.line_clear();
                        field.level += cleared_rows.len() as u32;

                        let ticks_of_line_clear = 41;
                        let rows_to_lower = cleared_rows.iter().map(|x| x.0).collect::<Vec<i32>>();

                        for (y, row) in &cleared_rows {
                            for (x, col) in row.iter().rev().enumerate() {
                                cubes.spawn_cube(x as i32, *y as i32, col.unwrap().color);
                            }
                        }

                        field.state = GameState::ClearDelay {
                            ticks_remaining: ticks_of_line_clear,
                            rows_to_lower,
                        };
                    } else {
                        field.state = GameState::PlaceDelay {
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
                    field.well.commit_clear(rows_to_lower);
                    field.state = GameState::PlaceDelay {
                        ticks_remaining: 30,
                    };
                }
            }
            GameState::PlaceDelay {
                ref mut ticks_remaining,
            } => {
                *ticks_remaining -= 1;
                if *ticks_remaining == 0 {
                    if inputs.key_pressed(Input::CW) {
                        field.next.rotation = field.next.rotation.cw();
                    } else if inputs.key_pressed(Input::CCW) {
                        field.next.rotation = field.next.rotation.ccw();
                    }
                    if field.level % 100 != 99 {
                        field.level += 1;
                    }
                    if field
                        .next
                        .collides_with(&field.well, 0, 0, field.next.rotation)
                    {
                        field.state = GameState::GameOver {
                            ticks_remaining: 60 * 5,
                        };
                    } else {
                        field.next.do_gravity(
                            &field.well,
                            inputs,
                            level_to_gravity(field.level),
                            sounds,
                            false,
                        );
                        field.state = GameState::ActivePiece { piece: field.next };
                        field.next = field.randomizer.next_piece();
                        sounds.block_spawn(field.next.color);
                    }
                }
            }
            GameState::GameOver {
                ref mut ticks_remaining,
            } => {
                *ticks_remaining -= 1;

                if *ticks_remaining == 0 {
                    let mut randomizer = Randomizer::new();
                    field.well = Well::new();
                    field.next = randomizer.next_piece();
                    field.state = GameState::ActivePiece {
                        piece: randomizer.next_piece(),
                    };
                    field.randomizer = randomizer;
                    field.level = 0;
                }
            }
        }
    }
}
