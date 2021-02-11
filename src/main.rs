use std::{env, io::Error};

use futures_util::{StreamExt,SinkExt, stream::SplitSink};
use futures_util::sink::Sink;
use log::{info,debug};
use tokio::net::{TcpListener, TcpStream};
use tokio::stream::{Stream, StreamMap};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Sender, Receiver};
use tokio::sync::broadcast;
use bytes::Bytes;
use clap::{App, crate_name,crate_version,crate_authors,crate_description};
use warp::Filter;

use uuid::Uuid;
use std::collections::{HashMap, hash_map::Keys};
use std::sync::{Arc, Mutex};
use anyhow::{Result,Context};

use serde::{Deserialize, Serialize};

//static INDEX_HTML: &str = include_str!("static/index.html");

type PeerMap = Arc<Mutex<HashMap<Uuid, Sender<PeerMsg>>>>;

//use common::{Action, Signal};

// #[derive(Debug)]
// #[derive(Serialize, Deserialize)]
// #[serde(tag = "type", rename_all = "kebab-case")]
// enum Action {
//     List,
//     Offer { sdp: String },    
//     Answer { sdp: String },    
//     NewIceCandidate { candidate: String }
// }

// #[derive(Debug)]
// #[derive(Serialize, Deserialize)]
// #[serde(rename_all = "kebab-case")]
// struct Msg {
//     action: Action,
//     peer: Option<Uuid>
// }

#[derive(Debug)]
//#[derive(Serialize, Deserialize)]
//#[serde(rename_all = "kebab-case")]
struct PeerMsg {
    signal: common::Signal,
    sender: Uuid,
    recipient: Uuid
}

macro_rules! warp_embed_file {
    ($urlpath:expr, $filepath:expr) => {warp::path::path($filepath)
        .and(warp::path::end())
        .map(|| warp::reply::html(include_str!($urlpath)))}
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

    let addr: std::net::Ipv4Addr = matches.value_of("host")
        .unwrap_or("127.0.0.1").parse().unwrap();

    let port: u16 = matches.value_of_t("port").unwrap_or(8080);
    let peers: PeerMap = Arc::new(Mutex::new(HashMap::new()));

    let websockets = warp::path("ws")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let peers = peers.clone();
            ws.on_upgrade(move | socket | {
                client_handler(socket, peers)
            })
        });

    let routes = warp::get().and(
        websockets
        .or(warp::fs::dir("./src/static")) // TODO: embed resources in binary
    );

    log::debug!("{:?}", addr);
    warp::serve(routes)
        .tls()
        .key_path("./localhost.key")
        .cert_path("./localhost.crt")
        .run((addr, port))
        .await;
}

async fn client_handler(socket: warp::ws::WebSocket, peers: PeerMap) {
    log::debug!("New socket connection: {:?}", socket);
    let (mut client_tx, mut client_rx) = socket.split();

    let id = Uuid::new_v4();
    let (peer_tx, mut peer_rx) = mpsc::channel(16);

    peers.lock().unwrap().insert(id, peer_tx);

    loop {
        tokio::select! {
            Some(msg) = client_rx.next() => {
                log::debug!("ClientMsg: {:?}", msg);
                match msg {
                    Ok(msg) => {
                        if msg.is_text() {
                            handle_client(id, msg.to_str().unwrap(), &mut client_tx, &peers).await.unwrap();
                        } else if msg.is_close() {
                            peers.lock().unwrap().remove(&id);
                            return;
                        };

                    }
                    Err(err) => {
                        log::error!("{:?}", err);
                        peers.lock().unwrap().remove(&id);
                        return;
                    }

                }
            }
            Some(PeerMsg { signal, sender, .. }) = peer_rx.recv() => {
                log::debug!("PeerMsg: {:?} {:?}", signal, sender);
                let msg = common::ClientMsg::Signal { signal, sender };
                let msg = serde_json::to_string(&msg).unwrap();
                client_tx.send(warp::filters::ws::Message::text(msg)).await;
            }
        }
    }
}


async fn handle_client(sender: Uuid, msg: &str, client_tx: &mut futures_util::stream::SplitSink<warp::ws::WebSocket, warp::ws::Message>, peers: &PeerMap) -> anyhow::Result<()> {
    let msg: common::ServerMsg = serde_json::from_str(msg).unwrap();
    let peers : HashMap<Uuid, Sender<PeerMsg>> = peers.lock().unwrap().clone();
    match msg {
        common::ServerMsg::ListPeers => {
            let peers = peers.keys()
                .map(|key| *key)
                .filter(|key| *key != sender)
                .collect::<Vec<Uuid>>();
            let json = serde_json::to_string(&common::ClientMsg::ListPeers { peers }).unwrap();
            client_tx.send(warp::filters::ws::Message::text(json)).await
                .context("Failed sending message to client")

        }
        common::ServerMsg::Signal { recipient, signal } => {
            let peer_msg = PeerMsg { signal, recipient, sender };
            peers.get(&recipient).unwrap().clone().send(peer_msg).await
                .context("Something")
        }
        _ => {
            Ok(())
        }
    }
}
