use js_sys::Reflect;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    MessageEvent, 
    RtcPeerConnection, 
    RtcPeerConnectionIceEvent,
    RtcSdpType,
    RtcSessionDescriptionInit,
    RtcIceCandidate
};
use yew::callback::Callback;


#[derive(Clone, Debug, PartialEq, thiserror::Error)]
/// An error encountered by a WebSocket.
pub enum WebRtcError {
    #[error("{0}")]
    /// An error encountered when creating the WebSocket.
    CreationError(String),
}

#[derive(Debug)]
pub struct WebRtcService {
    pc: RtcPeerConnection
}

pub struct WebRtcTask {
    peer_connection: RtcPeerConnection
}

impl WebRtcTask {
    pub fn new() -> Result<WebRtcTask, WebRtcError> {
        //Err(WebRtcError::CreationError("unimplemented".to_owned()));
        let pc = RtcPeerConnection::new().unwrap();
        Ok(WebRtcTask {
            peer_connection: pc
        })
    }

    pub async fn get_offer(&self) -> String {
        let offer = JsFuture::from(self.peer_connection.create_offer()).await.unwrap();

        let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))
            .unwrap()
            .as_string()
            .unwrap();

        let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);

        offer_obj.sdp(&offer_sdp);

        let promise = self.peer_connection.set_local_description(&offer_obj);

        JsFuture::from(promise).await;

        offer_sdp
    }

    pub async fn set_offer(&self, offer_sdp: &str) -> String {
        let mut offer = RtcSessionDescriptionInit::new(RtcSdpType::Offer);

        offer.sdp(&offer_sdp);
        let promise = self.peer_connection.set_remote_description(&offer);
        JsFuture::from(promise).await;

        let answer = JsFuture::from(self.peer_connection.create_answer()).await.unwrap();
        let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))
            .unwrap()
            .as_string()
            .unwrap();

        let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.sdp(&answer_sdp);
        let promise = self.peer_connection.set_local_description(&answer_obj);

        JsFuture::from(promise).await;

        answer_sdp
    }
    

    /*
     * Handle ICE candidate each other
     *
     */
    pub fn set_onicecandidate(&mut self, callback: Callback<RtcIceCandidate>) {
        let onicecandidate_callback =
            Closure::wrap(
                Box::new(move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
                    Some(candidate) => {
                        //console_log!("pc1.onicecandidate: {:#?}", candidate.candidate());
                        callback.emit(candidate);
                    }
                    None => {}
                }) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>,
            );

        self.peer_connection.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
        onicecandidate_callback.forget();
    }

        //Ok(())
} 