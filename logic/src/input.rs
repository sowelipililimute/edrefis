// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use nanoserde::{DeJson, SerJson};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum Input {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    CW = 4,
    CW2 = 5,
    CCW = 6,
    CCW2 = 7,
    DebugLevel = 8,
}

impl Input {
    pub const COUNT: usize = 9;
    pub const ALL: [Input; Input::COUNT] = [
        Input::Up,
        Input::Down,
        Input::Left,
        Input::Right,
        Input::CW,
        Input::CW2,
        Input::CCW,
        Input::CCW2,
        Input::DebugLevel,
    ];
}

#[derive(Debug, Default, Copy, Clone, SerJson, DeJson)]
#[repr(transparent)]
pub struct FrameInputs(u16);

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct PendingInput(pub FrameInputs);

impl FrameInputs {
    pub fn is_set(self, i: Input) -> bool {
        self.0 & 1 << i as u16 != 0
    }
    pub fn with(self, i: Input) -> FrameInputs {
        FrameInputs(self.0 | 1 << i as u16)
    }
}

#[derive(Debug, Clone, SerJson, DeJson, Default)]
pub struct InputState {
    previous: FrameInputs,
    inputs: [u64; Input::COUNT],
    input_tickstamps: [u64; Input::COUNT],
    inputs_up: [u64; Input::COUNT],
}

impl InputState {
    pub fn key_down(&self, code: Input, frame: FrameInputs) -> bool {
        match code {
            Input::Left | Input::Right | Input::Up | Input::Down => {
                let left = self.input_tickstamps[Input::Left as usize];
                let right = self.input_tickstamps[Input::Right as usize];
                let up = self.input_tickstamps[Input::Up as usize];
                let down = self.input_tickstamps[Input::Down as usize];

                if code == Input::Left
                    && left >= right
                    && left >= up
                    && left >= down
                    && frame.is_set(code)
                {
                    true
                } else if code == Input::Right
                    && right >= left
                    && right >= up
                    && right >= down
                    && frame.is_set(code)
                {
                    true
                } else if code == Input::Up
                    && up >= down
                    && up >= left
                    && up >= right
                    && frame.is_set(code)
                {
                    true
                } else if code == Input::Down
                    && down >= up
                    && down >= left
                    && down >= right
                    && frame.is_set(code)
                {
                    true
                } else {
                    false
                }
            }
            _ => frame.is_set(code),
        }
    }
    pub fn tick(&mut self, tick: u64, frame: FrameInputs) {
        let rising = frame.0 & !self.previous.0;
        for i in Input::ALL {
            if rising & 1 << i as u16 != 0 {
                self.input_tickstamps[i as usize] = tick;
            }
        }
        for input in Input::ALL {
            if self.key_down(input, frame) {
                self.inputs[input as usize] += 1;
                self.inputs_up[input as usize] = 0;
            } else {
                self.inputs_up[input as usize] += 1;
                self.inputs[input as usize] = 0;
            }
        }
    }
    pub fn pressed(&self, input: Input) -> bool {
        self.inputs[input as usize] > 0
    }
    pub fn just_pressed(&self, input: Input) -> bool {
        self.inputs[input as usize] == 1
    }
    pub fn just_released(&self, input: Input) -> bool {
        self.inputs_up[input as usize] == 1
    }
    pub fn press_duration(&self, input: Input) -> u64 {
        self.inputs[input as usize]
    }
    pub fn just_pressed_or_das(&self, input: Input, das: u64) -> bool {
        self.just_pressed(input) || self.press_duration(input) > das
    }
}
