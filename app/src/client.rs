use std::{collections::HashMap, time::Duration};

use hecs::Entity;
use logic::{
    peer::{Endpoint, Peer},
    proto::{ClientToServer, ServerToClient},
};

pub struct ClientEndpoint;

#[derive(Debug, Default)]
pub struct ClientState {
    server_to_client_entities: HashMap<u32, Entity>,
    pub ping: Duration,
}

pub trait ClientPeer {
    fn connect(&mut self);
}

impl Endpoint for ClientEndpoint {
    type Incoming = ServerToClient;
    type Outgoing = ClientToServer;
    type State = ClientState;

    fn handle_packet(peer: &mut Peer<Self>, packet: Self::Incoming) {
        match packet {
            ServerToClient::States(states) => {
                peer.registry.apply_states(
                    &mut peer.world,
                    &mut peer.state.server_to_client_entities,
                    &states,
                );
            }
            ServerToClient::Pong(secs, nanos) => {
                peer.state.ping = peer.elapsed_time() - Duration::new(secs, nanos);
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
}

pub type Client = Peer<ClientEndpoint>;
