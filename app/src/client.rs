use std::collections::HashMap;

use hecs::Entity;
use logic::{
    peer::{Endpoint, Peer},
    proto::{ClientToServer, ServerToClient},
};

pub struct ClientEndpoint;

impl Endpoint for ClientEndpoint {
    type Incoming = ServerToClient;
    type Outgoing = ClientToServer;
    type State = HashMap<u32, Entity>;

    fn handle_packet(peer: &mut Peer<Self>, packet: Self::Incoming) {
        match packet {
            ServerToClient::States(states) => {
                peer.registry
                    .apply_states(&mut peer.world, &mut peer.state, &states);
            }
        }
    }
}

pub type Client = Peer<ClientEndpoint>;
