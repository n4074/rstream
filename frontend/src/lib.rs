#![recursion_limit="1024"]
#![warn(clippy::all)]

use anyhow::Error;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use yew::prelude::*;
use yew::format::Json;
use yew::html::NodeRef;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};
use yewtil::future::LinkFuture;
use wasm_bindgen_futures::{JsFuture, spawn_local};


//use futures::executor::block_on;
use std::sync::{Arc, Mutex};
type Connections = Arc<Mutex<HashMap<Uuid, WebRtcTask>>>;

use web_sys::{window, Location, Url, MediaStream,HtmlVideoElement};

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
	peers: Vec<Uuid>,
	connections: HashMap<Uuid, Arc<WebRtcTask>>,
	mediastream: Option<MediaStream>,
	//mediastream2: Arc<MediaStream>,
	//in_streams: Vec<(NodeRef, MediaStream)>,
	self_video: NodeRef,
	other_video: NodeRef,
}

#[derive(Debug)]
enum Action {
	Connect,                         // connect to websocket server
	Disconnected,                    // disconnected from server
	Ignore,                          // ignore this message
	Signal(ServerMsg),
	ConnectPeer(Uuid), // fix this to uuid
	Received(Result<ClientMsg, Error>), // data received from server
	SetMediaStream(MediaStream),
	MediaStreamAdded(Uuid, MediaStream),
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
				<button onclick=self.link.callback(move |_| Action::ConnectPeer(id))>{ id.to_string() }</button>
			}
		}).collect::<Html>()
	}

	fn video_view(&self) -> Html {
		html!{
			<>
			<video id="localvideo" autoplay=true ref=self.self_video.clone() />
			<video id="remotevideo" autoplay=true ref=self.other_video.clone() />
			</>
		}
	}

	fn new_peer(&mut self, id: Uuid) -> Arc<WebRtcTask> {
		let pc = self.connections.entry(id).or_insert_with(|| 
			Arc::new(WebRtcTask::new().unwrap())
		).clone();

		if let Some(mediastream) = &self.mediastream {
			pc.add_tracks(&mediastream);
		}

		let onicecandidate_callback = self.link.callback(move |candidate| {
			ServerMsg::Signal { signal: Signal::NewIceCandidate { candidate: candidate }, recipient: id }
		});

		let ontrack_callback = self.link.callback(move |stream| {
			Action::MediaStreamAdded(id, stream)
		});

		pc.set_ontrack(ontrack_callback);
		pc.set_onicecandidate(onicecandidate_callback);
		pc
	}

	fn accept_connection(&mut self, sdp: String, id: Uuid) {
		let pc = self.new_peer(id);

		self.link.send_future(async move {
			&pc.set_remote_description(&sdp).await;
			let sdp = &pc.create_answer().await;

			ServerMsg::Signal { signal: Signal::Answer { sdp: sdp.to_string() } , recipient: id }
		});
	}

	fn request_connection(&mut self, id: Uuid) {
		let pc = self.new_peer(id);

		self.link.send_future(async move {
			let sdp = &pc.get_offer().await;
			ServerMsg::Signal { signal: Signal::Offer { sdp: sdp.to_string() } , recipient: id }
		});
	}

}

async fn get_user_media() -> Result<MediaStream, JsValue> {
	let window = web_sys::window().unwrap();
	let navigator = window.navigator();;
	let mut constraints = web_sys::MediaStreamConstraints::new();

	constraints.audio(&JsValue::TRUE);
	constraints.video(&JsValue::TRUE);

	let promise = JsFuture::from(navigator.media_devices().unwrap().get_user_media_with_constraints(&constraints).unwrap());
	promise.await.and_then(|val| val.dyn_into::<MediaStream>())
}

impl Component for Model {
	type Message = Action;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {

		link.send_future(async {
			let mediastream = get_user_media().await;
			match mediastream {
				Ok(mediastream) => {
					log::debug!("{:?}", mediastream);
					Action::SetMediaStream(mediastream)
				}
				Err(err) => {
					log::debug!("{:?}", err);
					Action::Ignore
				}
			}
		});

		Model {
			ws: None,
			link: link,
			peers: Vec::new(),
			connections: HashMap::new(),
			mediastream: None,
			self_video: NodeRef::default(),
			other_video: NodeRef::default(),
		}
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
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
				self.request_connection(id);
				false
			}

			Action::Signal(signal) => {
				if let Some(ref mut task) = self.ws {
					task.send(Json(&signal));
				}
				false
			}
			Action::Received(Ok(s)) => {
                match s {
                    common::ClientMsg::ListPeers { peers }  => {
						self.peers = peers;
                    }

                    common::ClientMsg::Signal { signal: common::Signal::Answer { sdp }, sender, .. } => {
						if let Some(pc) = self.connections.get(&sender).map(|pc| pc.clone()) {
							spawn_local(async move {
								&pc.set_answer(&sdp).await;
							})
						}
					}

                    common::ClientMsg::Signal { signal: common::Signal::Offer { sdp }, sender, .. } => {
						log::debug!("Offer: {:?}", sdp);
						self.accept_connection(sdp, sender);
					}

					common::ClientMsg::Signal { signal: common::Signal::NewIceCandidate { candidate }, sender, .. } => {
						if let Some(pc) = self.connections.get(&sender).map(|pc| pc.clone()) {
							spawn_local(async move {
								&pc.add_ice_candidate(candidate).await;
							})
						}
                    }
                    _ => {

                    }
                }
				true
			}
			Action::MediaStreamAdded(id, stream) => {
				if let Some(video) = self.other_video.cast::<HtmlVideoElement>() {
					video.set_src_object(Some(&stream));
				}
				false
			}
			Action::Received(Err(s)) => {
				log::error!("error:{:?}", s);
				true
			}
			Action::SetMediaStream(mediastream) => {

				if let Some(video) = self.self_video.cast::<HtmlVideoElement>() {
					video.set_src_object(Some(&mediastream));
				}

				web_sys::console::log_1(mediastream.as_ref());
				self.mediastream = Some(mediastream);

				false
			}
		}
	}

    fn view(&self) -> Html {
		html! {
            <>
			// connect button
			<button onclick=self.link.callback(|_| Action::Connect)>{ "Connect" }</button>
			<button onclick=self.link.callback(|_| Action::Signal(ServerMsg::ListPeers))>{ "Get Peers" }</button>
			{ self.peer_view() }
			// text showing whether we're connected or not
			<p>{ "Connected: " } { !self.ws.is_none() } </p><br/>
			<p>{ self.video_view() }</p>
            </>
		}
	}
}

#[wasm_bindgen(start)]
pub fn run_app() {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));
	wasm_logger::init(wasm_logger::Config::default());
	
	yew::start_app::<Model>();
}