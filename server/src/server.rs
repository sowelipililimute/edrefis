use logic::{
    field::spawn_field,
    peer::{Endpoint, Peer},
    proto::{ClientToServer, ServerToClient},
};

pub struct ServerEndpoint;

impl Endpoint for ServerEndpoint {
    type Incoming = (u32, ClientToServer);
    type Outgoing = (u32, ServerToClient);
    type State = ();

    fn handle_packet(peer: &mut Peer<Self>, (_client_id, packet): Self::Incoming) {
        match packet {
            ClientToServer::Join => {
                spawn_field(&mut peer.world);
                let states = peer.registry.serialize_world(&mut peer.world);
                peer.outgoing_packets
                    .push_back((0, ServerToClient::States(states)));
            }
        }
    }
}

pub type Server = Peer<ServerEndpoint>;
