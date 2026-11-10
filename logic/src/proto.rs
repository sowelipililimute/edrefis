// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use nanoserde::{DeJson, SerJson};

use crate::{
    input::FrameInputs,
    net::{States, Tick},
};

#[derive(SerJson, DeJson, Clone)]
pub enum ClientToServer {
    Join,
    Inputs {
        starting_at: Tick,
        ticks: Vec<FrameInputs>,
    },
    Ack(Tick),
    Ping(u64, u32),
}

#[derive(SerJson, DeJson, Clone)]
pub enum ServerToClient {
    Welcome {
        tick: Tick,
    },
    Snapshot {
        current_tick: Tick,
        inputs_acked_at: Option<Tick>,
        starting_from: Option<Tick>,
        states: States,
    },
    Pong(u64, u32),
}
