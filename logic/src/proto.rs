// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use nanoserde::{DeJson, SerJson};

use crate::net::States;

#[derive(SerJson, DeJson, Clone)]
pub enum ClientToServer {
    Join,
}

#[derive(SerJson, DeJson, Clone)]
pub enum ServerToClient {
    States(States),
}
