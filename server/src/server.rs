use logic::{
    field::spawn_field,
    net::ServerWorld,
    peer::{Endpoint, Peer},
    proto::{ClientToServer, ServerToClient},
};

pub struct ServerEndpoint;

impl Endpoint for ServerEndpoint {
    type Incoming = (u32, ClientToServer);
    type Outgoing = (u32, ServerToClient);
    type State = ();
    type World = ServerWorld;

    fn handle_packet(peer: &mut Peer<Self>, (client_id, packet): Self::Incoming) {
        match packet {
            ClientToServer::Join => {
                spawn_field(&mut peer.world);
                let states = peer.world.serialize();
                peer.outgoing_packets
                    .push_back((client_id, ServerToClient::States(states)));
            }
            ClientToServer::Ping(secs, nanos) => {
                peer.outgoing_packets
                    .push_back((client_id, ServerToClient::Pong(secs, nanos)));
            }
            ClientToServer::Inputs { starting_at, ticks } => todo!(),
            ClientToServer::Ack(tick) => todo!(),
        }
    }
}

pub type Server = Peer<ServerEndpoint>;
