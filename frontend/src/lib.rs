#![recursion_limit="1024"]

use anyhow::Error;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew::format::Json;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yewtil::future::LinkFuture;
use wasm_bindgen_futures::{JsFuture, spawn_local};


//use futures::executor::block_on;
use std::sync::{Arc, Mutex};
type Connections = Arc<Mutex<HashMap<Uuid, WebRtcTask>>>;

use web_sys::{window, Location, Url};

use common::{ClientMsg, ServerMsg, Signal};

use peer::Peer;

use std::collections::HashMap;
use uuid::Uuid;
use webrtc::WebRtcTask;

use log::{debug, info, error};

mod webrtc;
mod peer;


struct Model {
	ws: Option<WebSocketTask>,
	link: ComponentLink<Model>,
	text: String,                    // text in our input box
	server_data: String,             // data received from the server
	peers: Vec<Uuid>,
	connections: HashMap<Uuid, Arc<WebRtcTask>>
}

#[derive(Debug)]
enum Action {
	Connect,                         // connect to websocket server
	Disconnected,                    // disconnected from server
	Ignore,                          // ignore this message
	Signal(ServerMsg),
	ConnectPeer(Uuid), // fix this to uuid
	TextInput(String),               // text was input in the input box
	SendText,                        // send our text to server
	Received(Result<ClientMsg, Error>), // data received from server
}

impl From<ServerMsg> for Action {
	fn from(msg: ServerMsg) -> Self {
		Self::Signal(msg)
	}
}

impl From<ClientMsg> for Action {
	fn from(msg: ClientMsg) -> Self {
		Self::Received(Ok(msg))
	}
}

impl Model {

	fn peer_view(&self) -> Html {
		self.peers.iter().map(|id|  {
			let id = id.clone();
			html!{
				<p><button onclick=self.link.callback(move |_| Action::ConnectPeer(id))>{ id.to_string() }</button></p>
			}
		}).collect::<Html>()
	}
}

impl Component for Model {
	type Message = Action;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		Model {
			ws: None,
			link: link,
			text: String::new(),
			server_data: String::new(),
			peers: Vec::new(),
			connections: HashMap::new()
		}
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		log::debug!("Processing Msg: {:?}", msg);
		match msg {
			Action::Connect => {
				log::debug!("Connecting");
				let cbout = self.link.callback(|Json(data)| Action::Received(data));
				let cbnot = self.link.callback(|input| {
					log::debug!("Notification: {:?}", input);
					match input {
						WebSocketStatus::Closed | WebSocketStatus::Error => {
							Action::Disconnected
						}
						_ => Action::Ignore,
					}
				});
				if self.ws.is_none() {
					let url = Url::new(&window().unwrap().location().origin().unwrap()).unwrap(); 
					url.set_protocol(&url.protocol().replace("http", "ws"));
					url.set_pathname("/ws");
					let task = WebSocketService::connect(&url.href(), cbout, cbnot.into()).unwrap();
					self.ws = Some(task);
				}
				true
			}
			Action::Disconnected => {
				self.ws = None;
				true
			}
			Action::Ignore => {
				false
			}

			Action::ConnectPeer(id) => {

				let pc = self.connections.entry(id).or_insert_with(|| Arc::new(WebRtcTask::new().unwrap())).clone();

				let onicecandidate_callback = self.link.callback(move |candidate| {
					ServerMsg::Signal { signal: Signal::NewIceCandidate { candidate: candidate }, recipient: id }
				});


				self.link.send_future(async move {
					pc.set_onicecandidate(onicecandidate_callback);
					let sdp = &pc.get_offer().await;
					log::debug!("Sending Signal::Offer: {:?}", sdp);
					ServerMsg::Signal { signal: Signal::Offer { sdp: sdp.to_string() } , recipient: id }
				});

				false
			}

			Action::TextInput(e) => {
				self.text = e; // note input box value
				true
			}
			Action::Signal(signal) => {
				log::debug!("Sending Signal: {:?}", signal);
				match self.ws {
					Some(ref mut task) => {
						//let signal : common::ServerMsg = common::ServerMsg::ListPeers;
						//let json : String = serde_json::to_string(&signal).unwrap();
						task.send(Json(&signal));
						self.text = "".to_string();
						true // clear input box
					}
					None => {
						false
					}
				}
			}
			Action::SendText => {
				match self.ws {
					Some(ref mut task) => {
						let signal : common::ServerMsg = common::ServerMsg::ListPeers;
						//let json : String = serde_json::to_string(&signal).unwrap();
						task.send(Json(&signal));
						self.text = "".to_string();
						true // clear input box
					}
					None => {
						false
					}
				}
			}
			Action::Received(Ok(s)) => {
                self.server_data.push_str(&format!("{:?}\n", &s.clone()));
                match s {
                    common::ClientMsg::ListPeers { peers }  => {
						log::debug!("Peers: {:?}", peers);
						self.peers = peers;
                    }

                    common::ClientMsg::Signal { signal: common::Signal::Answer { sdp }, sender, .. } => {
						log::debug!("Answer: {:?}", sdp);
						let pc = self.connections.get(&sender);

						match pc {
							Some(pc) => {
								let pc = pc.clone();
								spawn_local(async move {
									&pc.set_answer(&sdp).await;
								})
							}
							None => {									
								log::error!("Unsolicited answer from {}", sender);
							}
							
						}
					}

                    common::ClientMsg::Signal { signal: common::Signal::Offer { sdp }, sender, .. } => {
						log::debug!("Offer: {:?}", sdp);

						let pc = self.connections.entry(sender).or_insert_with(|| Arc::new(WebRtcTask::new().unwrap())).clone();

						let onicecandidate_callback = self.link.callback(move |candidate| {
							ServerMsg::Signal { signal: Signal::NewIceCandidate { candidate: candidate }, recipient: sender }
						});

						log::debug!("Received offer from {}", sender);

						self.link.send_future(async move {
							pc.set_onicecandidate(onicecandidate_callback);
							let sdp = &pc.set_offer(&sdp).await;
							log::debug!("Sending Answer: {:?} {:?}", sdp, sender);
							ServerMsg::Signal { signal: Signal::Answer { sdp: sdp.to_string() } , recipient: sender }
						});
					}

					common::ClientMsg::Signal { signal: common::Signal::NewIceCandidate { candidate }, sender, .. } => {
						log::debug!("Candidate: {:?}", candidate);

						let pc = self.connections.get(&sender);

						match pc {
							Some(pc) => {
								let pc = pc.clone();
								spawn_local(async move {
									&pc.add_ice_candidate(&candidate).await;
								})
							}
							None => {									
								log::error!("Unsolicited ice candidate from {}", sender);
							}
							
						}
                    }
                    _ => {

                    }
                }
				true
			}
			Action::Received(Err(s)) => {
				log::debug!("Error here: {:?}", s);
				self.server_data.push_str(&format!("Error when reading data from server: {}\n", &s.to_string()));
				true
			}
		}
	}

    fn view(&self) -> Html {
		html! {
            <>
			// connect button
			<p><button onclick=self.link.callback(|_| Action::Connect)>{ "Connect" }</button></p><br/>
			// text showing whether we're connected or not
			<p>{ "Connected: " } { !self.ws.is_none() } </p><br/>
			// input box for sending text
			<p><input type="text", value=&self.text, oninput=self.link.callback(|e : yew::events::InputData | Action::TextInput(e.value))/></p><br/>
			<p>{ self.peer_view() }</p>
			// button for sending text
			<p><button onclick=self.link.callback(|_| Action::Signal(ServerMsg::ListPeers))>{ "Get Peers" }</button></p><br/>
			// text area for showing data from the server
            <p><textarea value=&self.server_data,></textarea></p><br/>
            </>
		}
	}
}

#[wasm_bindgen(start)]
pub fn run_app() {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));
	wasm_logger::init(wasm_logger::Config::default());
	let window = web_sys::window().unwrap();
	let navigator = window.navigator();;
	let mut constraints = web_sys::MediaStreamConstraints::new();

	constraints.audio(&JsValue::from(true));
	constraints.video(&JsValue::from(true));

	let media_devices = navigator.media_devices().unwrap().get_user_media_with_constraints(&constraints).unwrap();
	//let media_devices = futures::executor::block_on(media_devices);



	//App::<Model>::new().mount_to_body();
	yew::start_app::<Model>();
}