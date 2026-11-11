use std::collections::{BTreeMap, HashMap, VecDeque};

use hecs::Entity;
use logic::{
    field::{field_system, set_input, spawn_field},
    input::FrameInputs,
    net::{ServerWorld, Tick, World, WorldSnapshot},
    peer::{Endpoint, Peer},
    proto::{ClientToServer, ServerToClient},
};

pub const HISTORY_TICKS: usize = 120;
pub const MAX_INPUT_LEAD: u32 = 120;

pub struct ServerEndpoint;

#[derive(Default)]
pub struct ServerState {
    pub clients: HashMap<u32, ClientRecord>,
    pub rollbacks: u32,
    history: VecDeque<WorldSnapshot>,
    rollback_from: Option<Tick>,
}

#[derive(Debug)]
pub struct ClientRecord {
    pub joined_at: Tick,
    pub acked: Option<Tick>,
    pub field: Entity,
    received_inputs: BTreeMap<Tick, FrameInputs>,
    used_inputs: BTreeMap<Tick, FrameInputs>,
}

impl ClientRecord {
    fn baseline_floor(&self) -> Tick {
        self.acked.unwrap_or(self.joined_at)
    }

    fn predicted_input(&self, tick: Tick) -> FrameInputs {
        self.received_inputs
            .range(..=tick)
            .next_back()
            .map(|(_, input)| *input)
            .unwrap_or_default()
    }

    pub fn inputs_acked_at(&self) -> Option<Tick> {
        self.received_inputs.last_key_value().map(|(tick, _)| *tick)
    }

    fn prune_inputs(&mut self, oldest: Tick) {
        let latest_before = self
            .received_inputs
            .range(..oldest)
            .next_back()
            .map(|(tick, _)| *tick);
        self.received_inputs
            .retain(|tick, _| *tick >= oldest || Some(*tick) == latest_before);
        self.used_inputs.retain(|tick, _| *tick >= oldest);
    }
}

fn simulate(peer: &mut Server) {
    let tick = peer.world.tick();
    peer.state.history.push_back(peer.world.snapshot());

    for client in peer.state.clients.values_mut() {
        let input = client.predicted_input(tick);
        client.used_inputs.insert(tick, input);
        set_input(&mut peer.world, client.field, input);
    }

    field_system(&mut peer.world);
}

fn rollback(peer: &mut Server, from: Tick) {
    let current_tick = peer.world.tick();
    let Some(index) = peer
        .state
        .history
        .iter()
        .position(|snapshot| snapshot.tick >= from)
    else {
        return;
    };

    if !peer.world.restore(&peer.state.history[index]) {
        return;
    }
    peer.state.history.truncate(index);

    while peer.world.tick() < current_tick {
        simulate(peer);
        peer.world.increment_tick();
    }

    peer.world.dirty_all();
    peer.state.rollbacks += 1;
}

impl Endpoint for ServerEndpoint {
    type Incoming = (u32, ClientToServer);
    type Outgoing = (u32, ServerToClient);
    type State = ServerState;
    type World = ServerWorld;

    fn handle_packet(peer: &mut Peer<Self>, (client_id, packet): Self::Incoming) {
        match packet {
            ClientToServer::Join => {
                let field = spawn_field(&mut peer.world);
                peer.state.clients.insert(
                    client_id,
                    ClientRecord {
                        joined_at: peer.world.tick(),
                        acked: None,
                        field,
                        received_inputs: BTreeMap::new(),
                        used_inputs: BTreeMap::new(),
                    },
                );
            }
            ClientToServer::Ping(secs, nanos) => {
                peer.outgoing_packets
                    .push_back((client_id, ServerToClient::Pong(secs, nanos)));
            }
            ClientToServer::Inputs { starting_at, ticks } => {
                let current_tick = peer.world.tick();
                let oldest_tick = peer
                    .state
                    .history
                    .front()
                    .map_or(current_tick, |snapshot| snapshot.tick);
                let Some(client) = peer.state.clients.get_mut(&client_id) else {
                    return;
                };

                for (offset, input) in ticks.into_iter().enumerate() {
                    let tick = Tick(starting_at.0 + offset as u32);

                    if tick.0 > current_tick.0 + MAX_INPUT_LEAD
                        || client.received_inputs.contains_key(&tick)
                    {
                        continue;
                    }
                    client.received_inputs.insert(tick, input);

                    if tick < current_tick && client.used_inputs.get(&tick) != Some(&input) {
                        let from = tick.max(oldest_tick);
                        peer.state.rollback_from = Some(
                            peer.state
                                .rollback_from
                                .map_or(from, |existing| existing.min(from)),
                        );
                    }
                }
            }
            ClientToServer::Ack(tick) => {
                if let Some(client) = peer.state.clients.get_mut(&client_id) {
                    client.acked = client.acked.max(Some(tick));
                }
            }
        }
    }

    fn tick(peer: &mut Peer<Self>) {
        if let Some(from) = peer.state.rollback_from.take() {
            rollback(peer, from);
        }
        simulate(peer);

        let current_tick = peer.world.tick();

        for (client_id, client) in &peer.state.clients {
            peer.outgoing_packets.push_back((
                *client_id,
                ServerToClient::Snapshot {
                    current_tick,
                    inputs_acked_at: client.inputs_acked_at(),
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

        while peer.state.history.len() > HISTORY_TICKS {
            peer.state.history.pop_front();
        }
        if let Some(oldest) = peer.state.history.front().map(|snapshot| snapshot.tick) {
            for client in peer.state.clients.values_mut() {
                client.prune_inputs(oldest);
            }
        }

        peer.world.increment_tick();
    }
}

pub type Server = Peer<ServerEndpoint>;
