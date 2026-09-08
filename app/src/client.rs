use std::collections::{HashMap, VecDeque};

use hecs::{Entity, World};
use logic::{
    net::NetComponentRegistry,
    proto::{ClientToServer, ServerToClient},
};

pub struct Client {
    pub world: World,
    events: VecDeque<Events>,
    pub outgoing_packets: VecDeque<ClientToServer>,
    registry: NetComponentRegistry,
    server_to_client_ids: HashMap<u32, Entity>,
}

enum Events {
    Packet(ServerToClient),
    DebugRun(Box<dyn FnOnce(&mut Client) -> () + Send>),
}

impl Client {
    pub fn new() -> Client {
        Client {
            world: World::new(),
            events: VecDeque::new(),
            outgoing_packets: VecDeque::new(),
            registry: NetComponentRegistry::new_with_all_components(),
            server_to_client_ids: HashMap::new(),
        }
    }

    pub fn queue_packet(&mut self, packet: ServerToClient) {
        self.events.push_back(Events::Packet(packet));
    }

    pub fn queue_run(&mut self, it: Box<dyn FnOnce(&mut Client) -> () + Send>) {
        self.events.push_back(Events::DebugRun(it));
    }

    pub fn queue_send(&mut self, packet: ClientToServer) {
        self.outgoing_packets.push_back(packet);
    }

    fn handle_packet(&mut self, packet: ServerToClient) {
        match packet {
            ServerToClient::States(states) => {
                self.registry.apply_states(
                    &mut self.world,
                    &mut self.server_to_client_ids,
                    &states,
                );
            }
        }
    }

    pub fn drain_events(&mut self) {
        while let Some(event) = self.events.pop_front() {
            match event {
                Events::Packet(server_to_client) => self.handle_packet(server_to_client),
                Events::DebugRun(fn_once) => fn_once(self),
            }
        }
    }
}
