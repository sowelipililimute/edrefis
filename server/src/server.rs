use std::collections::VecDeque;

use hecs::World;
use logic::{
    field::spawn_field,
    net::NetComponentRegistry,
    proto::{ClientToServer, ServerToClient},
};

use crate::server::Events::Packet;

pub struct Server {
    pub world: World,
    events: VecDeque<Events>,
    pub outgoing_packets: VecDeque<(u32, ServerToClient)>,
    registry: NetComponentRegistry,
}

enum Events {
    Packet(u32, ClientToServer),
    DebugRun(Box<dyn FnOnce(&mut Server) -> () + Send>),
}

impl Server {
    pub fn new() -> Server {
        Server {
            world: World::new(),
            events: VecDeque::new(),
            outgoing_packets: VecDeque::new(),
            registry: NetComponentRegistry::new_with_all_components(),
        }
    }

    pub fn queue_packet(&mut self, client_id: u32, packet: ClientToServer) {
        self.events.push_back(Events::Packet(client_id, packet));
    }

    pub fn queue_run(&mut self, it: Box<dyn FnOnce(&mut Server) -> () + Send>) {
        self.events.push_back(Events::DebugRun(it));
    }

    pub fn queue_send(&mut self, client_id: u32, packet: ServerToClient) {
        self.outgoing_packets.push_back((client_id, packet));
    }

    fn handle_packet(&mut self, client_id: u32, packet: ClientToServer) {
        match packet {
            ClientToServer::Join => {
                spawn_field(&mut self.world);
                let states = self.registry.serialize_world(&mut self.world);
                self.outgoing_packets
                    .push_back((0, ServerToClient::States(states)));
            }
        };
    }

    pub fn drain_events(&mut self) {
        while let Some(event) = self.events.pop_front() {
            match event {
                Events::Packet(client_id, client_to_server) => {
                    self.handle_packet(client_id, client_to_server)
                }
                Events::DebugRun(fn_once) => fn_once(self),
            }
        }
    }
}
