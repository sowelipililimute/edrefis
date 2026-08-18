// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use std::collections::HashMap;

use nanoserde::{DeJson, SerJson};

pub trait InputProvider {
    fn peek(&mut self);
    fn consume(&mut self);
    fn key_just_pressed(&self, input: Input) -> bool;
    fn key_down(&self, input: Input) -> bool;
}

#[derive(DeJson, SerJson, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    CW,
    CCW,
    DebugLevel,
}

pub struct Inputs {
    inputs: HashMap<Input, u16>,
    inputs_up: HashMap<Input, u16>,
    inputs_tickstamps: HashMap<Input, u64>,
}

pub const INPUTS: &[Input] = &[
    Input::Up,
    Input::Down,
    Input::Left,
    Input::Right,
    Input::CCW,
    Input::CW,
    Input::DebugLevel,
];

impl Inputs {
    pub fn new() -> Inputs {
        Inputs {
            inputs: HashMap::new(),
            inputs_tickstamps: HashMap::new(),
            inputs_up: HashMap::new(),
        }
    }

    fn key_down(&self, code: Input, provider: &mut dyn InputProvider) -> bool {
        match code {
            Input::Left | Input::Right | Input::Up | Input::Down => {
                let left = self.inputs_tickstamps.get(&Input::Left).unwrap_or(&0);
                let right = self.inputs_tickstamps.get(&Input::Right).unwrap_or(&0);
                let up = self.inputs_tickstamps.get(&Input::Up).unwrap_or(&0);
                let down = self.inputs_tickstamps.get(&Input::Down).unwrap_or(&0);

                if code == Input::Left
                    && left >= right
                    && left >= up
                    && left >= down
                    && provider.key_down(code)
                {
                    true
                } else if code == Input::Right
                    && right >= left
                    && right >= up
                    && right >= down
                    && provider.key_down(code)
                {
                    true
                } else if code == Input::Up
                    && up >= down
                    && up >= left
                    && up >= right
                    && provider.key_down(code)
                {
                    true
                } else if code == Input::Down
                    && down >= up
                    && down >= left
                    && down >= right
                    && provider.key_down(code)
                {
                    true
                } else {
                    false
                }
            }
            _ => provider.key_down(code),
        }
    }
    pub fn tick(&mut self, tick: u64, provider: &mut dyn InputProvider) {
        provider.peek();
        for input in INPUTS {
            if provider.key_just_pressed(*input) {
                self.inputs_tickstamps.insert(*input, tick);
            }
        }
        for input in INPUTS {
            if self.key_down(*input, provider) {
                self.inputs
                    .entry(*input)
                    .and_modify(|x| *x = *x + 1)
                    .or_insert(1);
                self.inputs_up.insert(*input, 0);
            } else {
                self.inputs_up
                    .entry(*input)
                    .and_modify(|x| *x = *x + 1)
                    .or_insert(1);
                self.inputs.insert(*input, 0);
            }
        }
        provider.consume();
    }
    pub fn key_pressed(&self, input: Input) -> bool {
        self.inputs.get(&input).unwrap_or(&0) > &0
    }
    pub fn key_just_pressed(&self, input: Input) -> bool {
        self.inputs.get(&input).unwrap_or(&0) == &1
    }
    pub fn key_just_released(&self, input: Input) -> bool {
        self.inputs_up.get(&input).unwrap_or(&0) == &1
    }
    fn key_press_duration(&self, input: Input) -> u16 {
        *self.inputs.get(&input).unwrap_or(&0)
    }
    pub fn key_press_or_das(&self, input: Input, das: u16) -> bool {
        self.key_just_pressed(input) || self.key_press_duration(input) > das
    }
}

pub const RECORDABLE_INPUTS: &[Input] = &[
    Input::Up,
    Input::Down,
    Input::Left,
    Input::Right,
    Input::CCW,
    Input::CW,
];
