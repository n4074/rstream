use js_sys::{Reflect, Error};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    MessageEvent, 
    RtcPeerConnection, 
    RtcPeerConnectionIceEvent,
    RtcSdpType,
    RtcSessionDescriptionInit,
    RtcIceCandidateInit,
    RtcIceCandidate
};
use yew::callback::Callback;

use anyhow::{Result, Context};


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

#[derive(Debug)]
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

    pub async fn set_answer(&self, answer_sdp: &str) { // -> Result<JsValue, Error> {
        let mut answer = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        answer.sdp(&answer_sdp);

        let promise = self.peer_connection.set_remote_description(&answer);
        // TODO: Fixup all these ignored promises
        JsFuture::from(promise).await;
    }

    pub async fn add_ice_candidate(&self, candidate: &str) {
        let candidate = RtcIceCandidateInit::new(candidate);
        let promise = self.peer_connection.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&candidate));
        JsFuture::from(promise).await;
    }
    

    /*
     * Handle ICE candidate each other
     *
     */
    pub fn set_onicecandidate(&self, callback: Callback<String>) {
        let onicecandidate_callback =
            Closure::wrap(
                Box::new(move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
                    Some(candidate) => {
                        log::debug!("pc1.onicecandidate: {:#?}", candidate.candidate());
                        callback.emit(candidate.candidate());
                    }
                    None => {}
                }) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>,
            );

        self.peer_connection.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
        onicecandidate_callback.forget();
    }

        //Ok(())
} 