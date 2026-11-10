use std::{collections::HashMap, time::Duration};

use hecs::Entity;
use logic::{
    net::{ClientWorld, NetId},
    peer::{Endpoint, Peer},
    proto::{ClientToServer, ServerToClient},
};

pub struct ClientEndpoint;

#[derive(Debug, Default)]
pub struct ClientState {
    server_to_client_entities: HashMap<NetId, Entity>,
    pub ping: Duration,
}

pub trait ClientPeer {
    fn connect(&mut self);
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
                states,
                ..
            } => {
                peer.world
                    .apply_states(&mut peer.state.server_to_client_entities, &states);
                peer.queue_send(ClientToServer::Ack(current_tick));
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
