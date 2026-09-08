#![cfg(test)]

use app::client::Client;
use futures_util::{SinkExt, StreamExt};
use hecs::{Entity, World};
use logic::net::*;
use logic::proto::ClientToServer;
use logic::{field::spawn_field, proto::ServerToClient};
use nanoserde::{DeJson, SerJson};
use server::server::Server;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio::{io::duplex, sync::Mutex};
use tokio_stream::wrappers::IntervalStream;
use tokio_tungstenite::{accept_async, client_async, tungstenite::protocol::Message};

struct Pair {
    client: Arc<Mutex<Client>>,
    server: Arc<Mutex<Server>>,
}

impl Pair {
    async fn run_server<TFunc: FnOnce(&mut Server) -> () + Send + 'static>(&self, server: TFunc) {
        self.server.lock().await.queue_run(Box::new(server));
    }

    async fn run_client<TFunc: FnOnce(&mut Client) -> () + Send + 'static>(&self, client: TFunc) {
        self.client.lock().await.queue_run(Box::new(client));
    }
}

fn make_pair() -> Pair {
    let (client_io, server_io) = duplex(64 * 1024);
    let client = Arc::new(Mutex::new(Client::new()));
    let server = Arc::new(Mutex::new(Server::new()));
    let client_clone = client.clone();
    let server_clone = server.clone();

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
                            server_ws_stream.feed(Message::Text(packet.serialize_json().into())).await.unwrap();
                        }
                        server_ws_stream.flush().await.unwrap();
                    } else {
                        break
                    }
                }
                msg = server_ws_stream.next() => {
                    if let Some(msg) = msg {
                        if let Message::Text(text) = msg.unwrap() {
                            let packet: ClientToServer = DeJson::deserialize_json(text.as_str()).unwrap();
                            server.lock().await.queue_packet(0u32, packet);
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
        server: server_clone,
    }
}

#[test]
fn state_sync_basics() {
    let mut server_world = World::new();
    let server_field = spawn_field(&mut server_world);
    server_world
        .insert(server_field, (Replicated,))
        .expect("should be able to mark it as replicated");
    let registry = NetComponentRegistry::new_with_all_components();

    let states = registry.serialize_world(&mut server_world);

    let mut client_world = World::new();
    let mut server_to_client_ids = HashMap::<u32, Entity>::new();
    registry.apply_states(&mut client_world, &mut server_to_client_ids, &states);

    assert_eq!(server_to_client_ids.len(), 1);
}

#[tokio::test]
async fn client_server_basics() {
    let pair = make_pair();
    pair.run_client(|client| {
        client.queue_send(ClientToServer::Join);
    })
    .await;

    pair.run_server(|server| {
        assert_eq!(server.world.iter().count(), 1);
    })
    .await;
}
