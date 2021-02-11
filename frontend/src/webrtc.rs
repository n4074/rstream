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
    RtcIceCandidate,
    MediaStream,
    MediaStreamTrack,
    RtcTrackEvent,
    RtcConfiguration,
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
        let pc = RtcPeerConnection::new_with_configuration(&RtcConfiguration::new()).unwrap();
        Ok(WebRtcTask {
            peer_connection: pc
        })
    }

    pub fn log_pc(&self) {
        web_sys::console::log_1(self.peer_connection.as_ref());
    }

    pub fn add_tracks(&self, mediastream: &MediaStream) {

        for track in mediastream.get_tracks().iter() {
            web_sys::console::log_1(mediastream.as_ref());
            web_sys::console::log_1(track.as_ref());
            if let Ok(track) = track.dyn_into::<MediaStreamTrack>() {
                web_sys::console::error_2(&"add_track:".into(), track.as_ref());
                let sender = self.peer_connection.add_track_0(&track, mediastream);
            }
        }

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

        let res = JsFuture::from(promise).await;
        if let Ok(_) = res {
            log::info!("Success setting local description");
        } else {
            log::error!("Error setting local description: {:?}", res);
        }
        offer_sdp
    }

    pub async fn set_remote_description(&self, offer_sdp: &str) {
        let mut offer = RtcSessionDescriptionInit::new(RtcSdpType::Offer);

        offer.sdp(&offer_sdp);
        let promise = self.peer_connection.set_remote_description(&offer);
        let res = JsFuture::from(promise).await;
        if let Ok(_) = res {
            log::info!("Success setting remote description");
        } else {
            log::error!("Error setting remote description: {:?}", res);
        }
    }

    pub async fn create_answer(&self) -> String {
        let answer = JsFuture::from(self.peer_connection.create_answer()).await.unwrap();
        let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))
            .unwrap()
            .as_string()
            .unwrap();

        let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.sdp(&answer_sdp);
        let promise = self.peer_connection.set_local_description(&answer_obj);

        let res = JsFuture::from(promise).await;
        if let Ok(_) = res {
            log::info!("Success creating answer");
        } else {
            log::error!("Error creating answer: {:?}", res);
        }
        answer_sdp
                        

    }

    pub async fn set_answer(&self, answer_sdp: &str) { // -> Result<JsValue, Error> {
        let mut answer = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer.sdp(&answer_sdp);

        let promise = self.peer_connection.set_remote_description(&answer);
        // TODO: Fixup all these ignored promises
        let res = JsFuture::from(promise).await;

        if let Ok(_) = res {
            log::info!("Success setting remote description");
        } else {
            log::error!("Error setting remote description: {:?}", res);
        }
    }

    pub async fn add_ice_candidate(&self, candidate: common::IceCandidate) {
        let mut candidate_init = RtcIceCandidateInit::new(&candidate.candidate);
        candidate_init.sdp_m_line_index(candidate.sdp_m_line_index);
        candidate_init.sdp_mid(candidate.sdp_mid.as_deref());

        //let candidate_init = js_sys::JSON::parse(&candidate.blob).unwrap();
        
        let candidate = RtcIceCandidate::new(&candidate_init).unwrap();
        log::info!("remote_candidate: {:?}", candidate.candidate());
        let promise = self.peer_connection.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&candidate_init));
        let res = JsFuture::from(promise).await;
        if let Ok(_) = res {
            log::info!("Success setting remote candidate");
        } else {
            log::error!("Error setting remote candidate: {:?}", res);
        }
    }
    

    /*
     * Handle ICE candidate each other
     *
     */
    pub fn set_onicecandidate(&self, callback: Callback<common::IceCandidate>) {
        let onicecandidate_callback =
            Closure::wrap(
                Box::new(move |ev: RtcPeerConnectionIceEvent| 
                    if let Some(candidate) = ev.candidate() {
                        log::info!("local_candidate: {:?}", candidate.candidate());

                        callback.emit(common::IceCandidate { candidate: candidate.candidate(), sdp_mid: candidate.sdp_mid(), sdp_m_line_index: candidate.sdp_m_line_index() });
                        //callback.emit(common::IceCandidate { blob: js_sys::JSON::stringify(candidate.as_ref()).unwrap().as_string().unwrap() });
                    }
                ) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>,
            );


        self.peer_connection.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
        onicecandidate_callback.forget();
    }

    pub fn set_ontrack(&self, callback: Callback<MediaStream>) {
        let ontrack_callback =
            Closure::wrap(
                Box::new(move |ev: RtcTrackEvent| {
                    web_sys::console::log_1(ev.streams().as_ref());
                    let stream = ev.streams().get(0); 
                    if let Ok(stream) = stream.dyn_into() {
                        log::info!("ontrack: {:?}", stream);

                        callback.emit(stream)
                    }
                }) as Box<dyn FnMut(RtcTrackEvent)>,
            );

        self.peer_connection.set_ontrack(Some(ontrack_callback.as_ref().unchecked_ref()));
        ontrack_callback.forget();
    }
        //Ok(())
} 