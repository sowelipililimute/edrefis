// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use std::cmp::max;

use nanoserde::{DeJson, SerJson};

use crate::hooks::Sounds;
use crate::input::{Input, Inputs};
use crate::well::{Block, BlockDirections, Tile, Well, WELL_COLS, WELL_ROWS};

#[derive(Copy, Clone, Debug, SerJson, DeJson)]
pub enum Rotation {
    R0,
    R90,
    R180,
    R270,
}

#[derive(Copy, Clone, SerJson, DeJson, Debug)]
pub enum Rotations {
    IPiece,
    OPiece,
    TPiece,
    ZPiece,
    SPiece,
    JPiece,
    LPiece,
}

impl Rotations {
    pub fn piece_map(&self) -> &'static PieceMaps {
        match self {
            Rotations::IPiece => I_PIECE,
            Rotations::OPiece => O_PIECE,
            Rotations::TPiece => T_PIECE,
            Rotations::ZPiece => Z_PIECE,
            Rotations::SPiece => S_PIECE,
            Rotations::JPiece => J_PIECE,
            Rotations::LPiece => L_PIECE,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PieceMaps {
    r0: PieceMap,
    r90: PieceMap,
    r180: PieceMap,
    r270: PieceMap,
}

impl std::ops::Index<Rotation> for PieceMaps {
    type Output = PieceMap;

    fn index(&self, index: Rotation) -> &Self::Output {
        match index {
            Rotation::R0 => &self.r0,
            Rotation::R90 => &self.r90,
            Rotation::R180 => &self.r180,
            Rotation::R270 => &self.r270,
        }
    }
}

type PieceMap = &'static [&'static [bool]];

impl Rotation {
    pub fn ccw(&self) -> Rotation {
        match self {
            Rotation::R0 => Rotation::R270,
            Rotation::R90 => Rotation::R0,
            Rotation::R180 => Rotation::R90,
            Rotation::R270 => Rotation::R180,
        }
    }
    pub fn cw(&self) -> Rotation {
        match self {
            Rotation::R0 => Rotation::R90,
            Rotation::R90 => Rotation::R180,
            Rotation::R180 => Rotation::R270,
            Rotation::R270 => Rotation::R0,
        }
    }
}

#[derive(Copy, Clone, Debug, SerJson, DeJson)]
pub struct Piece {
    pub rotation: Rotation,
    pub rotations: Rotations,
    pub color: Block,
    pub x: i32,
    pub y: i32,

    ticks_to_next_gravity: i32,
    pub ticks_to_lock: i32,
}

impl Piece {
    pub fn new(color: Block) -> Piece {
        let map = match color {
            Block::Red => Rotations::IPiece,
            Block::Orange => Rotations::LPiece,
            Block::Yellow => Rotations::OPiece,
            Block::Green => Rotations::ZPiece,
            Block::Cyan => Rotations::TPiece,
            Block::Blue => Rotations::JPiece,
            Block::Purple => Rotations::SPiece,
        };
        Piece {
            rotation: Rotation::R0,
            rotations: map,
            color,
            x: 3,
            y: 0,
            ticks_to_lock: 30,
            ticks_to_next_gravity: 256,
        }
    }
    pub fn do_sonic(&mut self, well: &Well, inputs: &Inputs) {
        if inputs.key_just_pressed(Input::Up) {
            while !self.collides_with(well, 0, 1, self.rotation) {
                self.y += 1;
                self.ticks_to_lock = 30;
                self.ticks_to_next_gravity = 256;
            }
        }
    }
    pub fn do_horizontal(&mut self, well: &Well, inputs: &Inputs) {
        if inputs.key_press_or_das(Input::Left, 16) {
            if !self.collides_with(well, -1, 0, self.rotation) {
                self.x = self.x - 1;
            }
        } else if inputs.key_press_or_das(Input::Right, 16) {
            if !self.collides_with(well, 1, 0, self.rotation) {
                self.x = self.x + 1;
            }
        }
    }
    pub fn do_gravity(
        &mut self,
        well: &Well,
        inputs: &Inputs,
        rate: i32,
        sound: &mut dyn Sounds,
        inputs_active: bool,
    ) {
        if inputs.key_pressed(Input::Down) && inputs_active {
            self.ticks_to_next_gravity -= max(rate, 256);
        } else {
            self.ticks_to_next_gravity -= rate;
        }

        if self.ticks_to_next_gravity <= 0 {
            while self.ticks_to_next_gravity <= 0 {
                if !self.collides_with(well, 0, 1, self.rotation) {
                    self.y += 1;
                    self.ticks_to_lock = 30;
                }
                self.ticks_to_next_gravity += 256;
            }
            self.ticks_to_next_gravity = 256;
        }

        if self.collides_with(well, 0, 1, self.rotation) && self.ticks_to_lock == 30 {
            sound.land();
        }

        if self.collides_with(well, 0, 1, self.rotation) {
            self.ticks_to_lock -= 1;
            self.ticks_to_next_gravity = 256;
        }
    }
    pub fn do_rotate(&mut self, well: &Well, inputs: &Inputs) {
        if inputs.key_just_pressed(Input::CW) {
            if !self.collides_with(well, 0, 0, self.rotation.cw()) {
                self.rotation = self.rotation.cw();
            } else if !self.collides_with(well, 1, 0, self.rotation.cw()) {
                self.rotation = self.rotation.cw();
                self.x += 1;
            } else if !self.collides_with(well, -1, 0, self.rotation.cw()) {
                self.rotation = self.rotation.cw();
                self.x -= 1;
            }
        } else if inputs.key_just_pressed(Input::CCW) {
            if !self.collides_with(well, 0, 0, self.rotation.ccw()) {
                self.rotation = self.rotation.ccw();
            } else if !self.collides_with(well, 1, 0, self.rotation.ccw()) {
                self.rotation = self.rotation.ccw();
                self.x += 1;
            } else if !self.collides_with(well, -1, 0, self.rotation.ccw()) {
                self.rotation = self.rotation.ccw();
                self.x -= 1;
            }
        }
    }
    pub fn do_lock(&self, well: &mut Well, inputs: &Inputs, sounds: &mut dyn Sounds) -> bool {
        if self.collides_with(well, 0, 1, self.rotation)
            && (self.ticks_to_lock == 0 || inputs.key_pressed(Input::Down))
        {
            self.lock_to(well);
            sounds.lock();
            true
        } else {
            false
        }
    }
    fn lock_to(&self, well: &mut Well) {
        let current = self.rotations.piece_map()[self.rotation];
        for (ri, row) in current.iter().enumerate() {
            for (ci, col) in row.iter().enumerate() {
                if *col {
                    let check = |dx: i32, dy: i32| {
                        let row_idx = ri as i32 + dy;
                        let col_idx = ci as i32 + dx;
                        if row_idx < 0 || col_idx < 0 {
                            false
                        } else if row_idx as usize >= current.len() || col_idx as usize >= row.len()
                        {
                            false
                        } else {
                            current[row_idx as usize][col_idx as usize] != false
                        }
                    };

                    let up = check(0, -1);
                    let down = check(0, 1);
                    let left = check(-1, 0);
                    let right = check(1, 0);

                    well.blocks[(self.y + ri as i32) as usize][(self.x + ci as i32) as usize] =
                        Some(Tile {
                            color: self.color,
                            directions: BlockDirections::new(up, down, left, right),
                        });
                }
            }
        }
    }
    pub fn collides_with(&self, well: &Well, x_offset: i32, y_offset: i32, r: Rotation) -> bool {
        let current = self.rotations.piece_map()[r];
        for (ri, row) in current.iter().enumerate() {
            for (ci, col) in row.iter().enumerate() {
                let y_index = self.y + y_offset + ri as i32;
                let x_index = self.x + x_offset + ci as i32;

                if *col
                    && (y_index >= WELL_ROWS as i32 || x_index >= WELL_COLS as i32 || x_index < 0)
                {
                    return true;
                } else if y_index < 0 {
                    continue;
                } else if *col && well.blocks[y_index as usize][x_index as usize] != None {
                    return true;
                }
            }
        }
        return false;
    }
}

const F: bool = false;
const T: bool = true;

const J_PIECE: &'static PieceMaps = &PieceMaps {
    r0: &[&[F, F, F], &[T, T, T], &[F, F, T]],
    r90: &[&[F, T, F], &[F, T, F], &[T, T, F]],
    r180: &[&[F, F, F], &[T, F, F], &[T, T, T]],
    r270: &[&[F, T, T], &[F, T, F], &[F, T, F]],
};

const L_PIECE: &'static PieceMaps = &PieceMaps {
    r0: &[&[F, F, F], &[T, T, T], &[T, F, F]],
    r90: &[&[T, T, F], &[F, T, F], &[F, T, F]],
    r180: &[&[F, F, F], &[F, F, T], &[T, T, T]],
    r270: &[&[F, T, F], &[F, T, F], &[F, T, T]],
};

const S_PIECE: &'static PieceMaps = &PieceMaps {
    r0: &[&[F, F, F], &[F, T, T], &[T, T, F]],
    r90: &[&[T, F, F], &[T, T, F], &[F, T, F]],
    r180: &[&[F, F, F], &[F, T, T], &[T, T, F]],
    r270: &[&[T, F, F], &[T, T, F], &[F, T, F]],
};

const Z_PIECE: &'static PieceMaps = &PieceMaps {
    r0: &[&[F, F, F], &[T, T, F], &[F, T, T]],
    r90: &[&[F, F, T], &[F, T, T], &[F, T, F]],
    r180: &[&[F, F, F], &[T, T, F], &[F, T, T]],
    r270: &[&[F, F, T], &[F, T, T], &[F, T, F]],
};

const O_PIECE: &'static PieceMaps = &PieceMaps {
    r0: &[&[F, F, F, F], &[F, T, T, F], &[F, T, T, F], &[F, F, F, F]],
    r90: &[&[F, F, F, F], &[F, T, T, F], &[F, T, T, F], &[F, F, F, F]],
    r180: &[&[F, F, F, F], &[F, T, T, F], &[F, T, T, F], &[F, F, F, F]],
    r270: &[&[F, F, F, F], &[F, T, T, F], &[F, T, T, F], &[F, F, F, F]],
};

const I_PIECE: &'static PieceMaps = &PieceMaps {
    r0: &[&[F, F, F, F], &[T, T, T, T], &[F, F, F, F], &[F, F, F, F]],
    r90: &[&[F, F, T, F], &[F, F, T, F], &[F, F, T, F], &[F, F, T, F]],
    r180: &[&[F, F, F, F], &[T, T, T, T], &[F, F, F, F], &[F, F, F, F]],
    r270: &[&[F, F, T, F], &[F, F, T, F], &[F, F, T, F], &[F, F, T, F]],
};

const T_PIECE: &'static PieceMaps = &PieceMaps {
    r0: &[&[F, F, F], &[T, T, T], &[F, T, F]],
    r90: &[&[F, T, F], &[T, T, F], &[F, T, F]],
    r180: &[&[F, F, F], &[F, T, F], &[T, T, T]],
    r270: &[&[F, T, F], &[F, T, T], &[F, T, F]],
};
