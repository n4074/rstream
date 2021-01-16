#![recursion_limit="1024"]

use anyhow::Error;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use yew::format::Json;
use yew::services::ConsoleService;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};

use common::Action;

pub mod webrtc;

struct Model {
	ws: Option<WebSocketTask>,
	link: ComponentLink<Model>,
	text: String,                    // text in our input box
	server_data: String,             // data received from the server
}

enum Msg {
	Connect,                         // connect to websocket server
	Disconnected,                    // disconnected from server
	Ignore,                          // ignore this message
	TextInput(String),               // text was input in the input box
	SendText,                        // send our text to server
	Received(Result<String, Error>), // data received from server
}

impl Component for Model {
	type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		Model {
			ws: None,
			link: link,
			text: String::new(),
			server_data: String::new(),
		}
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Connect => {
				ConsoleService::log("Connecting");
				let cbout = self.link.callback(|Json(data)| Msg::Received(data));
				let cbnot = self.link.callback(|input| {
					ConsoleService::log(&format!("Notification: {:?}", input));
					match input {
						WebSocketStatus::Closed | WebSocketStatus::Error => {
							Msg::Disconnected
						}
						_ => Msg::Ignore,
					}
				});
				if self.ws.is_none() {
					let task = WebSocketService::connect("ws://127.0.0.1:8080/ws/", cbout, cbnot.into()).unwrap();
					self.ws = Some(task);
				}
				true
			}
			Msg::Disconnected => {
				self.ws = None;
				true
			}
			Msg::Ignore => {
				false
			}
			Msg::TextInput(e) => {
				self.text = e; // note input box value
				true
			}
			Msg::SendText => {
				match self.ws {
					Some(ref mut task) => {
						task.send(Json(&self.text));
						self.text = "".to_string();
						true // clear input box
					}
					None => {
						false
					}
				}
			}
			Msg::Received(Ok(s)) => {
                self.server_data.push_str(&format!("{}\n", &s));
                let action : common::Action = serde_json::from_str(&s).unwrap();
                match action {
                    common::Action::List => {
                        ConsoleService::log(&format!("List: {:?}", s));
                    }

                    common::Action::Answer { sdp } => {
                        ConsoleService::log(&format!("Answer: {:?}", sdp));
                    }

                    common::Action::Offer { sdp } => {
                        ConsoleService::log(&format!("Offer: {:?}", sdp));
                    }
                    _ => {

                    }
                }
				true
			}
			Msg::Received(Err(s)) => {
				self.server_data.push_str(&format!("Error when reading data from server: {}\n", &s.to_string()));
				true
			}
		}
    }

    fn view(&self) -> Html {
		html! {
            <>
			// connect button
			<p><button onclick=self.link.callback(|_| Msg::Connect)>{ "Connect" }</button></p><br/>
			// text showing whether we're connected or not
			<p>{ "Connected: " } { !self.ws.is_none() } </p><br/>
			// input box for sending text
			<p><input type="text", value=&self.text, oninput=self.link.callback(|e : yew::events::InputData | Msg::TextInput(e.value))/></p><br/>
			// button for sending text
			<p><button onclick=self.link.callback(|_| Msg::SendText)>{ "Send" }</button></p><br/>
			// text area for showing data from the server
            <p><textarea value=&self.server_data,></textarea></p><br/>
            </>
		}
	}
}

#[wasm_bindgen(start)]
pub fn run_app() {
    App::<Model>::new().mount_to_body();
}