// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use nanoserde::{DeJson, SerJson};

#[derive(Copy, Clone, Debug, Eq, PartialEq, SerJson, DeJson)]
pub enum Block {
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Purple,
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, SerJson, DeJson)]
pub struct BlockDirections(u8);

impl BlockDirections {
    const U: u8 = 0b1000;
    const D: u8 = 0b0100;
    const L: u8 = 0b0010;
    const R: u8 = 0b0001;

    pub const NONE: BlockDirections = BlockDirections(0);

    pub fn new(up: bool, down: bool, left: bool, right: bool) -> BlockDirections {
        let u = if up { Self::U } else { 0 };
        let d = if down { Self::D } else { 0 };
        let l = if left { Self::L } else { 0 };
        let r = if right { Self::R } else { 0 };

        BlockDirections(u | d | l | r)
    }
    #[inline]
    pub fn up(&self) -> bool {
        self.0 & Self::U != 0
    }
    #[inline]
    pub fn down(&self) -> bool {
        self.0 & Self::D != 0
    }
    #[inline]
    pub fn left(&self) -> bool {
        self.0 & Self::L != 0
    }
    #[inline]
    pub fn right(&self) -> bool {
        self.0 & Self::R != 0
    }
    pub fn match_with(
        &self,
        up: Option<BlockDirections>,
        down: Option<BlockDirections>,
        left: Option<BlockDirections>,
        right: Option<BlockDirections>,
    ) -> BlockDirections {
        BlockDirections::new(
            self.up() && up.map(|it| it.down()).unwrap_or(false),
            self.down() && down.map(|it| it.up()).unwrap_or(false),
            self.left() && left.map(|it| it.right()).unwrap_or(false),
            self.right() && right.map(|it| it.left()).unwrap_or(false),
        )
    }
    pub fn bits(&self) -> u8 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, SerJson, DeJson)]
pub struct Tile {
    pub color: Block,
    pub directions: BlockDirections,
}

pub const WELL_COLS: usize = 10;
pub const WELL_ROWS: usize = 21;

#[derive(SerJson, DeJson, Clone)]
pub struct Well {
    pub blocks: [[Option<Tile>; WELL_COLS]; WELL_ROWS],
}

impl Well {
    pub fn new() -> Well {
        Well {
            blocks: [[None; WELL_COLS]; WELL_ROWS],
        }
    }
    pub fn do_clear(&mut self) -> Vec<(i32, [Option<Tile>; WELL_COLS])> {
        let mut cleared = vec![];
        for ri in 0..self.blocks.len() {
            if self.blocks[ri].iter().all(|b| b.is_some()) {
                cleared.push((ri as i32, self.blocks[ri]));
                self.blocks[ri] = [None; WELL_COLS];
            }
        }
        cleared
    }
    pub fn commit_clear(&mut self, vec: &Vec<i32>) {
        for idx in vec {
            self.blocks[0..*idx as usize + 1].rotate_right(1);
        }
    }
}
