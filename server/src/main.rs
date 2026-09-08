mod server;

use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;

async fn accept_connection(peer: SocketAddr, stream: TcpStream) {
    handle_connection(peer, stream).await;
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream) {
    let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

    println!("New WebSocket connection: {}", peer);

    while let Some(msg) = ws_stream.next().await {
        let msg = msg.expect("message expect");
        if msg.is_text() || msg.is_binary() {
            ws_stream
                .send(msg)
                .await
                .expect("Failed to handle connection");
        }
    }
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8088";
    let listener = TcpListener::bind(&addr).await.expect("Can't listen");
    println!("Listening on {addr}");

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");

        println!("Received connection from {peer}");
        tokio::spawn(accept_connection(peer, stream));
    }
}
