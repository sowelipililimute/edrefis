// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use nanoserde::{DeJson, SerJson};

use crate::{piece::Piece, well::Block};

#[derive(SerJson, DeJson, Clone)]
pub enum Randomizer {
    TTATGM2P { seed: u32, history: [u8; 4] },
}

impl Randomizer {
    pub fn new() -> Randomizer {
        Randomizer::TTATGM2P {
            seed: 10,
            history: [1, 1, 2, 2],
        }
    }
    pub fn next_piece(&mut self) -> Piece {
        match self {
            Randomizer::TTATGM2P {
                ref mut seed,
                ref mut history,
            } => {
                let mut rand = || -> u32 {
                    const M: u32 = 0x41C64E6D;
                    const C: u32 = 0x3039;
                    const MSK: u32 = 0x7FFF;

                    *seed = seed.overflowing_mul(M).0 + C;
                    return (*seed >> 10) & MSK;
                };
                let mut r: u8 = 0;

                for _ in 0..5 {
                    r = (rand() % 7) as u8;

                    if !history.contains(&r) {
                        break;
                    }

                    r = (rand() % 7) as u8;
                }

                history[3] = history[2];
                history[2] = history[1];
                history[1] = history[0];
                history[0] = r;

                match r {
                    0 => Piece::new(Block::Red),
                    1 => Piece::new(Block::Green),
                    2 => Piece::new(Block::Purple),
                    3 => Piece::new(Block::Blue),
                    4 => Piece::new(Block::Orange),
                    5 => Piece::new(Block::Yellow),
                    6 => Piece::new(Block::Cyan),
                    _ => unreachable!("invalid piece"),
                }
            }
        }
    }
}
