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
//use tungstenite::{Message, Message::Text};
//use tokio_tungstenite::{WebSocketStream};
use warp::Filter;

use uuid::Uuid;
use std::collections::{HashMap, hash_map::Keys};
use std::sync::{Arc, Mutex};
use anyhow::{Result,Context};

use serde::{Deserialize, Serialize};
//use serde_json::Result;


type PeerMap = Arc<Mutex<HashMap<Uuid, Sender<PeerMsg>>>>;

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum Action {
    List,
    Offer { sdp: String },    
    Answer { sdp: String },    
    NewIceCandidate { candidate: String }
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Msg {
    action: Action,
    recipient: Option<Uuid>
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct PeerMsg {
    action: Action,
    sender: Uuid
}


#[tokio::main]
async fn main() {
    let _ = env_logger::try_init();

    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .author(crate_authors!())
        .arg("-h, --host=[address]   'Host IP to listen on'")
        .arg("-p, --port=[port]      'Host port to listen on'")
        .get_matches();

    let addr: std::net::Ipv4Addr = matches.value_of("address")
        .unwrap_or("127.0.0.1").parse().unwrap();

    let port: u16 = matches.value_of_t("port").unwrap_or(8080);
    //let addr_: std::net::Ipv4Addr = addr.parse().unwrap();

    //let listener = try_socket.expect("Failed to bind");
    //info!("Listening on: {}", addr);

    let peers: PeerMap = Arc::new(Mutex::new(HashMap::new()));

    let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));
    let websockets = warp::path("ws")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let peers = peers.clone();
            ws.on_upgrade(move | socket | {
                client_handler(socket, peers)
            })
        });

    let routes = warp::get().and(
        index
        .or(websockets)
    );

    warp::serve(routes).run((addr, port)).await;
}

async fn client_handler(socket: warp::ws::WebSocket, peers: PeerMap) {
    //let addr = stream.peer_addr()
    //    .expect("connected streams should have a peer address");

    //info!("Peer address: {}", addr);
    let (mut client_tx, mut client_rx) = socket.split();

    let id = Uuid::new_v4();
    let (peer_tx, mut peer_rx) = mpsc::channel(16);

    peers.lock().unwrap().insert(id, peer_tx);

    //let mut ws_stream = tokio_tungstenite::accept_async(stream)
    //    .await
    //    .expect("Error during the websocket handshake occurred");

    //info!("New WebSocket connection: {}", addr);

    loop {
        tokio::select! {
            Some(msg) = client_rx.next() => {
                println!("Client msg");
                let msg = msg.unwrap();
                if msg.is_text() {
                    handle_client(id, msg.to_str().unwrap(), &mut client_tx, &peers).await;
                };
            }
            Some(msg) = peer_rx.recv() => {
                println!("New Peer msg");
                let msg = serde_json::to_string(&msg).unwrap();
                client_tx.send(warp::filters::ws::Message::text(msg)).await;
            }
        }
    }
}


async fn handle_client(id: Uuid, msg: &str, client_tx: &mut futures_util::stream::SplitSink<warp::ws::WebSocket, warp::ws::Message>, peers: &PeerMap) -> anyhow::Result<()> {
    let msg: Msg = serde_json::from_str(msg).unwrap();
    //let peers: erMap = peers.lock().unwrap().clone();
    let peers : HashMap<Uuid, Sender<PeerMsg>> = peers.lock().unwrap().clone();
    //let peers_ : &mut HashMap<Uuid, Sender<PeerMsg>> = &mut peers;
    match msg {
        Msg { action: Action::List, .. } => {
            //let peers = peers.lock().unwrap().clone();
            let ids = peers.keys().filter(|key| **key != id).collect::<Vec<&Uuid>>();
            let json = serde_json::to_string(&ids).unwrap();
            client_tx.send(warp::filters::ws::Message::text(json.to_owned())).await
                .context("Failed sending message to client")

        }
        Msg { recipient: Some(recipient), action } => {
            println!("Forwarding msg");
            //let mut to: Sender<PeerMsg> = 
            peers.get(&recipient).unwrap().clone().send(PeerMsg { action: action, sender: id }).await
            //peers.get(&recipient_.to_owned()).unwrap().send(PeerMsg { action: action, sender: id }).await
                .context("Something")
        }
        _ => {
            Ok(())
        }
    }
}

static INDEX_HTML: &str = include_str!("index.html");