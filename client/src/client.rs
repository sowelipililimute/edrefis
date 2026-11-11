use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};

use hecs::Entity;
use logic::{
    input::FrameInputs,
    net::{ClientWorld, NetId, Tick},
    peer::{Endpoint, Peer},
    proto::{ClientToServer, ServerToClient},
};

pub const TICKS_PER_SECOND: f64 = 60.0;
pub const INPUT_BUFFER_TICKS: u32 = 2;

pub struct ClientEndpoint;

#[derive(Debug, Default)]
pub struct ClientState {
    server_to_client_entities: HashMap<NetId, Entity>,
    pub ping: Duration,
    pub server_tick: Option<Tick>,
    next_input_tick: Option<Tick>,
    unacked_inputs: VecDeque<(Tick, FrameInputs)>,
}

impl ClientState {
    fn input_lead(&self) -> u32 {
        (self.ping.as_secs_f64() * TICKS_PER_SECOND).ceil() as u32 + INPUT_BUFFER_TICKS
    }
}

pub trait ClientPeer {
    fn connect(&mut self);
    fn send_input(&mut self, input: FrameInputs);
}

impl Endpoint for ClientEndpoint {
    type Incoming = ServerToClient;
    type Outgoing = ClientToServer;
    type State = ClientState;
    type World = ClientWorld;

    fn handle_packet(peer: &mut Peer<Self>, packet: Self::Incoming) {
        match packet {
            ServerToClient::Pong(secs, nanos) => {
                peer.state.ping = peer.elapsed_time() - Duration::new(secs, nanos);
            }
            ServerToClient::Welcome { tick } => todo!(),
            ServerToClient::Snapshot {
                current_tick,
                inputs_acked_at,
                states,
                ..
            } => {
                peer.world
                    .apply_states(&mut peer.state.server_to_client_entities, &states);
                peer.queue_send(ClientToServer::Ack(current_tick));
                peer.state.server_tick = Some(current_tick);

                if let Some(acked) = inputs_acked_at {
                    while peer
                        .state
                        .unacked_inputs
                        .front()
                        .is_some_and(|(tick, _)| *tick <= acked)
                    {
                        peer.state.unacked_inputs.pop_front();
                    }
                }

                if peer
                    .state
                    .next_input_tick
                    .is_none_or(|next| next <= current_tick)
                {
                    peer.state.next_input_tick =
                        Some(Tick(current_tick.0 + 1 + peer.state.input_lead()));
                    peer.state.unacked_inputs.clear();
                }
            }
        }
    }
}

impl ClientPeer for Peer<ClientEndpoint> {
    fn connect(self: &mut Peer<ClientEndpoint>) {
        let instant = self.elapsed_time();

        self.queue_send(ClientToServer::Join);
        self.queue_send(ClientToServer::Ping(
            instant.as_secs(),
            instant.subsec_nanos(),
        ));
    }

    fn send_input(&mut self, input: FrameInputs) {
        let Some(tick) = self.state.next_input_tick else {
            return;
        };
        self.state.unacked_inputs.push_back((tick, input));
        self.state.next_input_tick = Some(Tick(tick.0 + 1));

        let starting_at = self.state.unacked_inputs[0].0;
        let ticks = self
            .state
            .unacked_inputs
            .iter()
            .map(|(_, input)| *input)
            .collect();
        self.queue_send(ClientToServer::Inputs { starting_at, ticks });
    }
}

pub type Client = Peer<ClientEndpoint>;
