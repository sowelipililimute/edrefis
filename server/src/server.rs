use std::collections::HashMap;

use logic::{
    field::spawn_field,
    net::{ServerWorld, Tick, World},
    peer::{Endpoint, Peer},
    proto::{ClientToServer, ServerToClient},
};

pub struct ServerEndpoint;

#[derive(Debug, Default)]
pub struct ServerState {
    pub clients: HashMap<u32, ClientRecord>,
}

#[derive(Debug)]
pub struct ClientRecord {
    pub joined_at: Tick,
    pub acked: Option<Tick>,
}

impl ClientRecord {
    fn baseline_floor(&self) -> Tick {
        self.acked.unwrap_or(self.joined_at)
    }
}

impl Endpoint for ServerEndpoint {
    type Incoming = (u32, ClientToServer);
    type Outgoing = (u32, ServerToClient);
    type State = ServerState;
    type World = ServerWorld;

    fn handle_packet(peer: &mut Peer<Self>, (client_id, packet): Self::Incoming) {
        match packet {
            ClientToServer::Join => {
                spawn_field(&mut peer.world);
                peer.state.clients.insert(
                    client_id,
                    ClientRecord {
                        joined_at: peer.world.tick(),
                        acked: None,
                    },
                );
            }
            ClientToServer::Ping(secs, nanos) => {
                peer.outgoing_packets
                    .push_back((client_id, ServerToClient::Pong(secs, nanos)));
            }
            ClientToServer::Inputs { starting_at, ticks } => todo!(),
            ClientToServer::Ack(tick) => {
                if let Some(client) = peer.state.clients.get_mut(&client_id) {
                    client.acked = client.acked.max(Some(tick));
                }
            }
        }
    }

    fn tick(peer: &mut Peer<Self>) {
        let current_tick = peer.world.tick();

        for (client_id, client) in &peer.state.clients {
            peer.outgoing_packets.push_back((
                *client_id,
                ServerToClient::Snapshot {
                    current_tick,
                    inputs_acked_at: None,
                    starting_from: client.acked,
                    states: peer.world.serialize(client.acked),
                },
            ));
        }

        let oldest_baseline = peer
            .state
            .clients
            .values()
            .map(ClientRecord::baseline_floor)
            .min()
            .unwrap_or(current_tick);
        peer.world.trim_tombstones(oldest_baseline);

        peer.world.increment_tick();
    }
}

pub type Server = Peer<ServerEndpoint>;
