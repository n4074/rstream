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

use serde::{Deserialize, Serialize};
//use serde_json::Result;


type Participants = Arc<Mutex<HashMap<Uuid, Sender<String>>>>;

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

    let addr = matches.value_of("address").unwrap_or("127.0.0.1");
    let port: u16 = matches.value_of_t("port").unwrap_or(8080);

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind((addr, port)).await;

    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    let cams: Participants = Arc::new(Mutex::new(HashMap::new()));
    //let (b_tx, b_rx) = broadcast::channel(16);
    
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(client_handler(stream, cams.clone()));
    }

    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum Msg {
    List,
    Register { id: String },
    Offer { sdp: String, recipient: Uuid },    
    Answer { sdp: String, recipient: Uuid },    
    NewIceCandidate { candidate: String, recipient: Uuid }
}





async fn client_handler(stream: TcpStream, cams: Participants) {
    let addr = stream.peer_addr()
        .expect("connected streams should have a peer address");

    info!("Peer address: {}", addr);

    let id = Uuid::new_v4();
    let (cam_tx, mut cam_rx) = mpsc::channel(16);

    cams.lock().unwrap().insert(id, cam_tx);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (mut outbound, mut inbound) = ws_stream.split();
    let mut peermap = StreamMap::new();

    loop {
        tokio::select! {
            Some(msg) = inbound.next() => {
                println!("Client msg");
                let msg = msg.unwrap();
                if msg.is_text() {
                    handle_client(msg.to_text().unwrap(), &mut outbound, &mut peermap, &cams).await;
                };
            }
            Some(msg) = cam_rx.recv() => {
                println!("Registration msg");
                //handle_peer(msg, cam_tx);
            }
            Some((key, msg)) = peermap.next() => {
                println!("Peer msg: {}", msg);
            }

        }
    }
}

//async fn handle_peer(msg: String, cam_tx: Sender<) {
//    
//}

async fn handle_client(msg: &str, outbound: &mut (impl SinkExt<Message> + Unpin), peers: &mut StreamMap<Uuid,Receiver<String>>, cams: &Participants) {
    let msg: Msg = serde_json::from_str(msg).unwrap();
    match msg {
        Msg::List => {
            let cams_: HashMap<Uuid,Sender<String>>;
            {
                let cams = cams.lock().unwrap();
                cams_ = cams.clone();
            }
            let ids = cams_.keys().collect::<Vec<&Uuid>>();
            let json = serde_json::to_string(&ids).unwrap();
            outbound.send(Text(json.to_owned())).await;

        },
        Msg::Register { id } => {
            let cams_: HashMap<Uuid,Sender<String>>;
            {
                let cams = cams.lock().unwrap();
                cams_ = cams.clone();
            }
            for (key,cam) in cams_.iter() {
                // send new channel 

                cam.send(id.clone()).await;
            }
        }
        Msg::Offer { sdp, recipient } => {
            println!("offer");
        }
        Msg::Answer { sdp, recipient } => {
            println!("answer");
        }
        Msg::NewIceCandidate { candidate, recipient } => {
            println!("candidate");
        }
        _ => {}
    }
}