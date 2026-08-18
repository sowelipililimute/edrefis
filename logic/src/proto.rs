// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

// use nanoserde::{DeJson, SerJson};

// use crate::{field::Field, input::Input};

// #[derive(SerJson, DeJson, Clone)]
// pub enum ClientToServer {
//     Join { client_id: u32 },
//     Input { input: Input, up: bool },
//     Tick {},
// }

// #[derive(SerJson, DeJson, Clone)]
// pub enum ServerToClient {
//     Join {
//         client_id: u32,
//         field: Field,
//     },
//     Leave {
//         client_id: u32,
//     },
//     Input {
//         client_id: u32,
//         input: Input,
//         up: bool,
//     },
//     Tick {
//         client_id: u32,
//     },
// }
