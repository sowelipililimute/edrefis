#![cfg(test)]

use client::client::{Client, ClientPeer};
use futures_util::{SinkExt, StreamExt};
use hecs::Entity;
use logic::field::{GameState, Level, field_system};
use logic::input::{FrameInputs, Input, InputState, PendingInput};
use logic::net::{World, *};
use logic::piece::Piece;
use logic::proto::ClientToServer;
use logic::randomizer::Randomizer;
use logic::well::Well;
use logic::{field::spawn_field, proto::ServerToClient};
use nanoserde::{DeJson, SerJson};
use server::server::{HISTORY_TICKS, MAX_INPUT_LEAD, Server};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::watch::error::RecvError;
use tokio::sync::{oneshot, watch};
use tokio::time::interval;
use tokio::{io::duplex, sync::Mutex};
use tokio_stream::wrappers::IntervalStream;
use tokio_tungstenite::{accept_async, client_async, tungstenite::protocol::Message};

struct Pair {
    client: Arc<Mutex<Client>>,
    client_ticks: watch::Receiver<()>,
    server: Arc<Mutex<Server>>,
    server_ticks: watch::Receiver<()>,
}

impl Pair {
    async fn queue_server<TFunc: FnOnce(&mut Server) -> () + Send + 'static>(&self, server: TFunc) {
        self.server.lock().await.queue_run(Box::new(server));
    }

    async fn queue_client<TFunc: FnOnce(&mut Client) -> () + Send + 'static>(&self, client: TFunc) {
        self.client.lock().await.queue_run(Box::new(client));
    }

    async fn run_server<TFunc: FnOnce(&mut Server) -> () + Send + 'static>(&self, server: TFunc) {
        let (tx, rx) = oneshot::channel();
        self.queue_server(|peer| {
            tx.send(()).unwrap();
            server(peer);
        })
        .await;

        rx.await.unwrap()
    }

    async fn run_client<TFunc: FnOnce(&mut Client) -> () + Send + 'static>(&self, client: TFunc) {
        let (tx, rx) = oneshot::channel();
        self.queue_client(|peer| {
            tx.send(()).unwrap();
            client(peer);
        })
        .await;

        rx.await.unwrap()
    }

    async fn wait_client_tick(&mut self) -> Result<(), RecvError> {
        self.client_ticks.changed().await
    }

    async fn wait_client_ticks(&mut self, ticks: u32) -> Result<(), RecvError> {
        for _ in 0..ticks {
            self.wait_client_tick().await?;
        }
        Ok(())
    }

    async fn wait_server_tick(&mut self) -> Result<(), RecvError> {
        self.server_ticks.changed().await
    }

    async fn wait_server_ticks(&mut self, ticks: u32) -> Result<(), RecvError> {
        for _ in 0..ticks {
            self.wait_server_tick().await?;
        }
        Ok(())
    }

    async fn wait_both_tick(&mut self) -> Result<(), RecvError> {
        tokio::try_join!(self.client_ticks.changed(), self.server_ticks.changed(),).map(|_| ())
    }

    async fn wait_both_ticks(&mut self, ticks: u32) -> Result<(), RecvError> {
        for _ in 0..ticks {
            self.wait_both_tick().await?;
        }
        Ok(())
    }
}

fn make_pair() -> Pair {
    let (client_io, server_io) = duplex(64 * 1024);
    let client = Arc::new(Mutex::new(Client::new()));
    let server = Arc::new(Mutex::new(Server::new()));
    let client_clone = client.clone();
    let server_clone = server.clone();
    let (client_tick_tx, client_tick_rx) = watch::channel(());
    let (server_tick_tx, server_tick_rx) = watch::channel(());

    tokio::spawn(async move {
        let (mut client_ws_stream, _response) =
            client_async("ws://localhost/", client_io).await.unwrap();
        let mut tick_stream = IntervalStream::new(interval(Duration::from_secs_f64(1f64 / 60f64)));

        loop {
            tokio::select! {
                tick = tick_stream.next() => {
                    if let Some(_) = tick {
                        let mut cli = client.lock().await;
                        cli.drain_events();
                        while let Some(packet) = cli.outgoing_packets.pop_front() {
                            client_ws_stream.feed(Message::Text(packet.serialize_json().into())).await.unwrap();
                        }
                        client_ws_stream.flush().await.unwrap();
                        client_tick_tx.send(()).unwrap();
                    } else {
                        break
                    }
                }
                msg = client_ws_stream.next() => {
                    if let Some(msg) = msg {
                        if let Message::Text(text) = msg.unwrap() {
                            let packet: ServerToClient = DeJson::deserialize_json(text.as_str()).unwrap();
                            client.lock().await.queue_packet(packet);
                        }
                    } else {
                        break
                    }
                }
            }
        }
    });

    tokio::spawn(async move {
        let mut server_ws_stream = accept_async(server_io).await.unwrap();
        let mut tick_stream = IntervalStream::new(interval(Duration::from_secs_f64(1f64 / 60f64)));

        loop {
            tokio::select! {
                tick = tick_stream.next() => {
                    if let Some(_) = tick {
                        let mut serv = server.lock().await;
                        serv.drain_events();
                        serv.tick();
                        while let Some(packet) = serv.outgoing_packets.pop_front() {
                            server_ws_stream.feed(Message::Text(packet.1.serialize_json().into())).await.unwrap();
                        }
                        server_ws_stream.flush().await.unwrap();
                        server_tick_tx.send(()).unwrap();
                    } else {
                        break
                    }
                }
                msg = server_ws_stream.next() => {
                    if let Some(msg) = msg {
                        if let Message::Text(text) = msg.unwrap() {
                            let packet: ClientToServer = DeJson::deserialize_json(text.as_str()).unwrap();
                            server.lock().await.queue_packet((0u32, packet));
                        }
                    } else {
                        break
                    }
                }
            }
        }
    });

    Pair {
        client: client_clone,
        client_ticks: client_tick_rx,
        server: server_clone,
        server_ticks: server_tick_rx,
    }
}

#[test]
fn state_sync_basics() {
    let mut server_world = ServerWorld::new();
    spawn_field(&mut server_world);

    let states = server_world.serialize(None);
    assert_eq!(states.entity_states.len(), 1);

    let mut client_world = ClientWorld::new();
    let mut server_to_client_ids = HashMap::<NetId, Entity>::new();
    client_world.apply_states(&mut server_to_client_ids, &states);

    assert_eq!(server_to_client_ids.len(), 1);
}

fn changed_at<T: Send + Sync + 'static>(world: &ServerWorld, entity: Entity) -> Tick {
    world
        .get::<&ReplicatedComp<T>>(entity)
        .expect("replicated component should have a tracker")
        .changed_at
}

#[test]
fn spawn_stamps_trackers_at_spawn_tick() {
    let mut world = ServerWorld::new();
    world.increment_tick();
    world.increment_tick();
    let field = spawn_field(&mut world);

    assert_eq!(changed_at::<Well>(&world, field), Tick(2));
    assert_eq!(changed_at::<Piece>(&world, field), Tick(2));
    assert_eq!(changed_at::<Level>(&world, field), Tick(2));
    assert_eq!(changed_at::<GameState>(&world, field), Tick(2));
    assert_eq!(changed_at::<Randomizer>(&world, field), Tick(2));
    assert_eq!(changed_at::<InputState>(&world, field), Tick(2));
    assert!(world.get::<&ReplicatedComp<PendingInput>>(field).is_err());
}

#[test]
fn dirty_stamps_only_mentioned_components() {
    let mut world = ServerWorld::new();
    let field = spawn_field(&mut world);
    for _ in 0..3 {
        world.increment_tick();
    }

    world.dirty::<(&Level, Option<&mut Well>, &PendingInput)>(field);

    assert_eq!(changed_at::<Level>(&world, field), Tick(3));
    assert_eq!(changed_at::<Well>(&world, field), Tick(3));
    assert_eq!(changed_at::<Piece>(&world, field), Tick(0));
    assert_eq!(changed_at::<GameState>(&world, field), Tick(0));
    assert_eq!(changed_at::<Randomizer>(&world, field), Tick(0));
    assert_eq!(changed_at::<InputState>(&world, field), Tick(0));
}

#[test]
fn serialize_sends_only_changes_since_baseline() {
    let mut world = ServerWorld::new();
    let field = spawn_field(&mut world);
    let component_count = |states: &States| states.entity_states.values().next().unwrap().len();

    assert_eq!(world.serialize(None).entity_states.len(), 1);
    assert_eq!(component_count(&world.serialize(None)), 6);
    assert!(world.serialize(Some(Tick(0))).entity_states.is_empty());

    world.increment_tick();
    world.dirty::<&Level>(field);

    assert_eq!(component_count(&world.serialize(Some(Tick(0)))), 1);
    assert!(world.serialize(Some(Tick(1))).entity_states.is_empty());
    assert_eq!(component_count(&world.serialize(None)), 6);
}

#[test]
fn serialize_sends_everything_for_entities_spawned_after_baseline() {
    let mut world = ServerWorld::new();
    spawn_field(&mut world);
    world.increment_tick();
    spawn_field(&mut world);

    let states = world.serialize(Some(Tick(0)));

    assert_eq!(states.entity_states.len(), 1);
    assert_eq!(states.entity_states.values().next().unwrap().len(), 6);
}

#[test]
fn despawn_sends_tombstone() {
    let mut world = ServerWorld::new();
    spawn_field(&mut world);
    let doomed = spawn_field(&mut world);
    let doomed_id = world.net_id(doomed).unwrap();

    world.increment_tick();
    world.despawn(doomed).unwrap();

    let states = world.serialize(Some(Tick(0)));
    assert!(states.entity_states.is_empty());
    assert_eq!(states.removed, vec![(doomed_id, None)]);
    assert!(world.serialize(Some(Tick(1))).removed.is_empty());
    assert!(world.serialize(None).removed.is_empty());

    world.trim_tombstones(Tick(1));
    assert!(world.serialize(Some(Tick(0))).removed.is_empty());
}

#[test]
fn remove_one_sends_tombstone_and_drops_tracker() {
    let mut world = ServerWorld::new();
    let field = spawn_field(&mut world);
    let id = world.net_id(field).unwrap();

    world.increment_tick();
    world.remove_one::<Level>(field).unwrap();
    world.remove_one::<PendingInput>(field).unwrap();

    assert!(world.get::<&ReplicatedComp<Level>>(field).is_err());

    let states = world.serialize(Some(Tick(0)));
    assert_eq!(states.removed.len(), 1);
    assert_eq!(states.removed[0].0, id);
    assert!(states.removed[0].1.is_some());
    assert_eq!(world.serialize(None).entity_states[&id].len(), 5);
}

#[test]
fn client_applies_removals() {
    let mut server = ServerWorld::new();
    let field = spawn_field(&mut server);
    let doomed = spawn_field(&mut server);
    let field_id = server.net_id(field).unwrap();

    let mut client = ClientWorld::new();
    let mut ids = HashMap::<NetId, Entity>::new();
    client.apply_states(&mut ids, &server.serialize(None));
    assert_eq!(ids.len(), 2);

    server.increment_tick();
    server.remove_one::<Level>(field).unwrap();
    server.despawn(doomed).unwrap();
    client.apply_states(&mut ids, &server.serialize(Some(Tick(0))));

    assert_eq!(ids.len(), 1);
    assert_eq!(client.iter().count(), 1);
    assert!(client.get::<&Level>(ids[&field_id]).is_err());
    assert!(client.get::<&Well>(ids[&field_id]).is_ok());
}

#[test]
fn client_applies_removal_before_reinsert() {
    let mut server = ServerWorld::new();
    let field = spawn_field(&mut server);
    let field_id = server.net_id(field).unwrap();

    let mut client = ClientWorld::new();
    let mut ids = HashMap::<NetId, Entity>::new();
    client.apply_states(&mut ids, &server.serialize(None));

    server.increment_tick();
    server.remove_one::<Level>(field).unwrap();
    server.increment_tick();
    server.insert(field, (Level(7),)).unwrap();
    client.apply_states(&mut ids, &server.serialize(Some(Tick(0))));

    assert_eq!(client.get::<&Level>(ids[&field_id]).unwrap().0, 7);
}

#[test]
fn field_system_dirties_changed_components() {
    let mut world = ServerWorld::new();
    let field = spawn_field(&mut world);

    world.increment_tick();
    field_system(&mut world);

    assert_eq!(changed_at::<GameState>(&world, field), Tick(1));
    assert_eq!(changed_at::<InputState>(&world, field), Tick(1));
    assert_eq!(changed_at::<Well>(&world, field), Tick(0));
    assert_eq!(changed_at::<Randomizer>(&world, field), Tick(0));
}

#[tokio::test]
async fn client_server_basics() {
    let pair = make_pair();

    pair.run_server(|server| {
        assert_eq!(server.world.iter().count(), 0);
    })
    .await;

    pair.run_client(|client| {
        assert_eq!(client.world.iter().count(), 0);
        client.connect();
    })
    .await;

    pair.run_server(|server| {
        assert_eq!(server.world.iter().count(), 1);
    })
    .await;

    pair.run_client(|client| {
        assert_eq!(client.world.iter().count(), 1);
    })
    .await;
}

#[tokio::test]
async fn snapshots_replicate_changes_and_removals() {
    let mut pair = make_pair();

    pair.run_client(|client| client.connect()).await;
    pair.wait_both_ticks(10).await.unwrap();

    pair.run_server(|server| {
        assert!(server.state.clients[&0].acked.is_some());

        let field = server
            .world
            .query::<(Entity, &Replicated)>()
            .iter()
            .next()
            .unwrap()
            .0;
        server.world.insert(field, (Level(42),)).unwrap();
    })
    .await;
    pair.wait_both_ticks(10).await.unwrap();

    pair.run_client(|client| {
        let level = client.world.query::<&Level>().iter().next().unwrap().0;
        assert_eq!(level, 42);
    })
    .await;

    pair.run_server(|server| {
        let field = server
            .world
            .query::<(Entity, &Replicated)>()
            .iter()
            .next()
            .unwrap()
            .0;
        server.world.despawn(field).unwrap();
    })
    .await;
    pair.wait_both_ticks(10).await.unwrap();

    pair.run_client(|client| {
        assert_eq!(client.world.iter().count(), 0);
    })
    .await;

    pair.run_server(|server| {
        assert!(server.world.serialize(Some(Tick(0))).removed.is_empty());
    })
    .await;
}

#[tokio::test]
async fn client_server_ping() {
    let mut pair = make_pair();

    pair.run_client(|client| {
        client.connect();
    })
    .await;

    pair.wait_both_tick().await.unwrap();

    pair.run_client(|client| {
        println!("poang! {:?}", client.state.ping);
    })
    .await;

    pair.wait_both_tick().await.unwrap();
}

fn left() -> FrameInputs {
    FrameInputs::default().with(Input::Left)
}

fn joined_server() -> Server {
    let mut server = Server::new();
    server.queue_packet((0, ClientToServer::Join));
    server.drain_events();
    server
}

fn send_inputs(server: &mut Server, starting_at: u32, ticks: Vec<FrameInputs>) {
    server.queue_packet((
        0,
        ClientToServer::Inputs {
            starting_at: Tick(starting_at),
            ticks,
        },
    ));
    server.drain_events();
}

fn run_ticks(server: &mut Server, ticks: usize) {
    for _ in 0..ticks {
        server.tick();
    }
}

fn active_piece_x<W: World>(world: &W) -> i32 {
    world
        .query::<&GameState>()
        .iter()
        .find_map(|state| match state {
            GameState::ActivePiece { piece } => Some(piece.x),
            _ => None,
        })
        .expect("field should have an active piece")
}

#[test]
fn late_input_rolls_back_to_match_on_time_input() {
    let mut on_time = joined_server();
    send_inputs(&mut on_time, 3, vec![left(); 4]);
    run_ticks(&mut on_time, 10);

    let mut late = joined_server();
    run_ticks(&mut late, 8);
    send_inputs(&mut late, 3, vec![left(); 4]);
    run_ticks(&mut late, 2);

    let mut idle = joined_server();
    run_ticks(&mut idle, 10);

    assert_eq!(on_time.state.rollbacks, 0);
    assert_eq!(late.state.rollbacks, 1);
    assert!(active_piece_x(&on_time.world) < active_piece_x(&idle.world));
    assert_eq!(
        on_time.world.serialize(None).entity_states,
        late.world.serialize(None).entity_states
    );
}

#[test]
fn missing_inputs_repeat_the_last_received_input() {
    let mut predicted = joined_server();
    send_inputs(&mut predicted, 0, vec![left()]);
    run_ticks(&mut predicted, 10);

    let mut explicit = joined_server();
    send_inputs(&mut explicit, 0, vec![left(); 10]);
    run_ticks(&mut explicit, 10);

    assert_eq!(
        predicted.world.serialize(None).entity_states,
        explicit.world.serialize(None).entity_states
    );
}

#[test]
fn late_input_matching_prediction_does_not_roll_back() {
    let mut server = joined_server();
    send_inputs(&mut server, 0, vec![left()]);
    run_ticks(&mut server, 6);
    send_inputs(&mut server, 1, vec![left(); 4]);
    run_ticks(&mut server, 2);

    assert_eq!(server.state.rollbacks, 0);
    assert_eq!(server.state.clients[&0].inputs_acked_at(), Some(Tick(4)));
}

#[test]
fn rollback_resends_corrected_state_to_acked_clients() {
    let mut server = joined_server();
    run_ticks(&mut server, 8);
    server.queue_packet((0, ClientToServer::Ack(Tick(7))));
    server.drain_events();
    send_inputs(&mut server, 3, vec![left(); 4]);
    server.outgoing_packets.clear();
    run_ticks(&mut server, 1);

    let Some((_, ServerToClient::Snapshot {
        starting_from,
        states,
        ..
    })) = server.outgoing_packets.pop_back()
    else {
        panic!("expected a snapshot");
    };
    assert_eq!(starting_from, Some(Tick(7)));
    assert_eq!(states.entity_states.values().next().unwrap().len(), 6);
}

#[test]
fn inputs_older_than_history_replay_from_oldest_snapshot() {
    let mut server = joined_server();
    run_ticks(&mut server, HISTORY_TICKS + 10);
    send_inputs(&mut server, 0, vec![left()]);
    run_ticks(&mut server, 1);

    assert_eq!(server.state.rollbacks, 1);
    assert_eq!(server.state.clients[&0].inputs_acked_at(), Some(Tick(0)));
}

#[test]
fn inputs_too_far_ahead_are_dropped() {
    let mut server = joined_server();
    send_inputs(&mut server, MAX_INPUT_LEAD + 1, vec![left()]);

    assert_eq!(server.state.clients[&0].inputs_acked_at(), None);
}

#[test]
fn client_stamps_inputs_ahead_of_server_and_trims_acked() {
    let snapshot = |current_tick: u32, inputs_acked_at: Option<u32>| ServerToClient::Snapshot {
        current_tick: Tick(current_tick),
        inputs_acked_at: inputs_acked_at.map(Tick),
        starting_from: None,
        states: States {
            entity_states: HashMap::new(),
            removed: Vec::new(),
        },
    };
    let last_inputs = |client: &mut Client| match client.outgoing_packets.pop_back() {
        Some(ClientToServer::Inputs { starting_at, ticks }) => (starting_at, ticks.len()),
        _ => panic!("expected inputs"),
    };

    let mut client = Client::new();
    client.send_input(left());
    assert!(client.outgoing_packets.is_empty());

    client.queue_packet(snapshot(10, None));
    client.drain_events();
    client.send_input(left());
    client.send_input(left());
    assert_eq!(last_inputs(&mut client), (Tick(13), 2));

    client.queue_packet(snapshot(11, Some(13)));
    client.drain_events();
    client.send_input(left());
    assert_eq!(last_inputs(&mut client), (Tick(14), 2));

    client.queue_packet(snapshot(20, None));
    client.drain_events();
    client.send_input(left());
    assert_eq!(last_inputs(&mut client), (Tick(23), 1));
}

#[tokio::test]
async fn client_inputs_drive_server_field() {
    let mut pair = make_pair();

    pair.run_client(|client| client.connect()).await;
    pair.wait_both_ticks(5).await.unwrap();

    let (start_tx, start_rx) = oneshot::channel();
    pair.run_server(move |server| {
        start_tx.send(active_piece_x(&server.world)).unwrap();
    })
    .await;
    let start_x = start_rx.await.unwrap();

    pair.run_client(|client| client.send_input(left())).await;
    pair.wait_both_ticks(10).await.unwrap();

    let (end_tx, end_rx) = oneshot::channel();
    pair.run_server(move |server| {
        end_tx.send(active_piece_x(&server.world)).unwrap();
    })
    .await;
    assert!(end_rx.await.unwrap() < start_x);
}
