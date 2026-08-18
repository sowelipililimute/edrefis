// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

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

impl Field {
    pub fn new() -> Field {
        let mut randomizer = Randomizer::new();

        Field {
            well: Well::new(),
            next: randomizer.next_piece(),
            level: 0,
            state: GameState::ActivePiece {
                piece: randomizer.next_piece(),
            },

            randomizer,
        }
    }
    pub fn update(&mut self, inputs: &Inputs, sounds: &mut dyn Sounds, cubes: &mut dyn Cubes) {
        match self.state {
            GameState::ActivePiece { ref mut piece } => {
                piece.do_sonic(&self.well, inputs);
                piece.do_rotate(&self.well, inputs);
                piece.do_horizontal(&self.well, inputs);
                piece.do_gravity(
                    &self.well,
                    inputs,
                    level_to_gravity(self.level),
                    sounds,
                    true,
                );

                if piece.do_lock(&mut self.well, inputs, sounds) {
                    let cleared_rows = self.well.do_clear();
                    if cleared_rows.len() > 0 {
                        sounds.line_clear();
                        self.level += cleared_rows.len() as u32;

                        let ticks_of_line_clear = 41;
                        let rows_to_lower = cleared_rows.iter().map(|x| x.0).collect::<Vec<i32>>();

                        for (y, row) in &cleared_rows {
                            for (x, col) in row.iter().rev().enumerate() {
                                cubes.spawn_cube(x as i32, *y as i32, col.unwrap().color);
                            }
                        }

                        self.state = GameState::ClearDelay {
                            ticks_remaining: ticks_of_line_clear,
                            rows_to_lower,
                        };
                    } else {
                        self.state = GameState::PlaceDelay {
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
                    self.well.commit_clear(rows_to_lower);
                    self.state = GameState::PlaceDelay {
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
                        self.next.rotation = self.next.rotation.cw();
                    } else if inputs.key_pressed(Input::CCW) {
                        self.next.rotation = self.next.rotation.ccw();
                    }
                    if self.level % 100 != 99 {
                        self.level += 1;
                    }
                    if self
                        .next
                        .collides_with(&self.well, 0, 0, self.next.rotation)
                    {
                        self.state = GameState::GameOver {
                            ticks_remaining: 60 * 5,
                        };
                    } else {
                        self.next.do_gravity(
                            &self.well,
                            inputs,
                            level_to_gravity(self.level),
                            sounds,
                            false,
                        );
                        self.state = GameState::ActivePiece { piece: self.next };
                        self.next = self.randomizer.next_piece();
                        sounds.block_spawn(self.next.color);
                    }
                }
            }
            GameState::GameOver {
                ref mut ticks_remaining,
            } => {
                *ticks_remaining -= 1;

                if *ticks_remaining == 0 {
                    let mut randomizer = Randomizer::new();
                    self.well = Well::new();
                    self.next = randomizer.next_piece();
                    self.state = GameState::ActivePiece {
                        piece: randomizer.next_piece(),
                    };
                    self.randomizer = randomizer;
                    self.level = 0;
                }
            }
        }
    }
}
