use std::{env, io::Error};

use futures_util::{StreamExt,SinkExt, stream::SplitSink};
use futures_util::sink::Sink;
use log::info;
use tokio::net::{TcpListener, TcpStream};
use tokio::stream::{Stream, StreamMap};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Sender, Receiver};
use tokio::sync::broadcast;
use bytes::Bytes;
use clap::{App, crate_name,crate_version,crate_authors,crate_description};
use tungstenite::{Message, Message::Text};
use tokio_tungstenite::{WebSocketStream};

use uuid::Uuid;
use std::collections::{HashMap, hash_map::Keys};
use std::sync::{Arc, Mutex};
use anyhow::{Result,Context};

use serde::{Deserialize, Serialize};
//use serde_json::Result;


type PeerMap = Arc<Mutex<HashMap<Uuid, Sender<PeerMsg>>>>;

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum Action {
    List,
    Offer { sdp: String },    
    Answer { sdp: String },    
    NewIceCandidate { candidate: String }
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
#[serde(tag = "type", rename_all = "kebab-case")]
struct Msg {
    action: Action,
    recipient: Option<Uuid>
}

#[derive(Debug)]
struct PeerMsg {
    action: Action,
    sender: Uuid
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();

    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .author(crate_authors!())
        .arg("-h, --host=[address]   'Host IP to listen on'")
        .arg("-p, --port=[port]      'Host port to listen on'")
        .get_matches();

    let addr = matches.value_of("address").unwrap_or("127.0.0.1");
    let port: u16 = matches.value_of_t("port").unwrap_or(8080);

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind((addr, port)).await;

    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    let peers: PeerMap = Arc::new(Mutex::new(HashMap::new()));
    
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(client_handler(stream, peers.clone()));
    }

    Ok(())
}

async fn client_handler(stream: TcpStream, peers: PeerMap) {
    let addr = stream.peer_addr()
        .expect("connected streams should have a peer address");

    info!("Peer address: {}", addr);

    let id = Uuid::new_v4();
    let (peer_tx, mut peer_rx) = mpsc::channel(16);

    peers.lock().unwrap().insert(id, peer_tx);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (mut outbound, mut inbound) = ws_stream.split();
    loop {
        tokio::select! {
            Some(msg) = inbound.next() => {
                println!("Client msg");
                let msg = msg.unwrap();
                if msg.is_text() {
                    handle_client(id, msg.to_text().unwrap(), &mut outbound, &peers).await;
                };
            }
            Some(msg) = peer_rx.recv() => {
                println!("New Peer msg");
                // insert receive into peer map
                //handle_peer(msg, cam_tx);
            }
        }
    }
}

async fn handle_client(id: Uuid, msg: &str, outbound: &mut (impl SinkExt<Message> + Unpin), peers: &PeerMap) -> anyhow::Result<()> {
    let msg: Msg = serde_json::from_str(msg).unwrap();
    let peers = peers.lock().unwrap().clone();
    Ok(match msg {
        Msg { action: Action::List, .. } => {
            //let peers = peers.lock().unwrap().clone();
            let ids = peers.keys().collect::<Vec<&Uuid>>();
            let json = serde_json::to_string(&ids).unwrap();
            outbound.send(Text(json.to_owned())).await?

        }
        Msg { recipient: Some(recipient), action } => {
            println!("Forwarding msg");
            peers.get(&recipient).unwrap().send(PeerMsg { action: action, sender: id }).await?
        }
        _ => {
            ()
        }
    })
}