use std::{collections::VecDeque, time::Duration};

use hecs::World;

use crate::{instant::Instant, net::NetComponentRegistry};

pub trait Endpoint: Sized {
    type Incoming;
    type Outgoing;
    type State: Default;

    fn handle_packet(peer: &mut Peer<Self>, packet: Self::Incoming);
}

pub struct Peer<E: Endpoint> {
    pub world: World,
    pub outgoing_packets: VecDeque<E::Outgoing>,
    pub registry: NetComponentRegistry,
    pub state: E::State,
    time_origin: Instant,
    events: VecDeque<Event<E>>,
}

enum Event<E: Endpoint> {
    Packet(E::Incoming),
    DebugRun(Box<dyn FnOnce(&mut Peer<E>) + Send>),
}

impl<E: Endpoint> Peer<E> {
    pub fn new() -> Self {
        Peer {
            world: World::new(),
            outgoing_packets: VecDeque::new(),
            registry: NetComponentRegistry::new_with_all_components(),
            state: E::State::default(),
            time_origin: Instant::now(),
            events: VecDeque::new(),
        }
    }

    pub fn queue_packet(&mut self, packet: E::Incoming) {
        self.events.push_back(Event::Packet(packet));
    }

    pub fn queue_run(&mut self, it: Box<dyn FnOnce(&mut Self) + Send>) {
        self.events.push_back(Event::DebugRun(it));
    }

    pub fn queue_send(&mut self, packet: E::Outgoing) {
        self.outgoing_packets.push_back(packet);
    }

    pub fn drain_events(&mut self) {
        while let Some(event) = self.events.pop_front() {
            match event {
                Event::Packet(p) => E::handle_packet(self, p),
                Event::DebugRun(f) => f(self),
            }
        }
    }

    pub fn elapsed_time(&self) -> Duration {
        self.time_origin.elapsed()
    }
}
