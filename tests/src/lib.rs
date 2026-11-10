#![cfg(test)]

use client::client::{Client, ClientPeer};
use futures_util::{SinkExt, StreamExt};
use hecs::Entity;
use logic::field::{GameState, Level, field_system};
use logic::input::{InputState, PendingInput};
use logic::net::{World, *};
use logic::piece::Piece;
use logic::proto::ClientToServer;
use logic::randomizer::Randomizer;
use logic::well::Well;
use logic::{field::spawn_field, proto::ServerToClient};
use nanoserde::{DeJson, SerJson};
use server::server::Server;
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
