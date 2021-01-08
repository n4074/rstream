//! A simple echo server.
//!
//! You can test this out by running:
//!
//!     cargo run --example server 127.0.0.1:12345
//!
//! And then in another window run:
//!
//!     cargo run --example client ws://127.0.0.1:12345/

use std::{env, io::Error};

use futures_util::StreamExt;
use log::info;
use tokio::net::{TcpListener, TcpStream};
use clap::{App, crate_name,crate_version,crate_authors,crate_description};


#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();

    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .author(crate_authors!())
        .arg("-h, --host=[address]   'Host IP to listen on'")
        .arg("-p, --port=[port]      'Host port to listen on'")
        .get_matches();

    //let addr = env::args().nth(1).unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let addr = matches.value_of("address").unwrap_or("127.0.0.1");
    let port: u16 = matches.value_of_t("port").unwrap_or(8080);

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind((addr, port)).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }

    Ok(())
}

async fn accept_connection(stream: TcpStream) {
    let addr = stream.peer_addr().expect("connected streams should have a peer address");
    info!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (write, read) = ws_stream.split();
    read.forward(write).await.expect("Failed to forward message")
}