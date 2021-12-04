use uuid::Uuid;

use crate::webrtc::WebRtcTask;

pub struct Peer {
   id: Uuid,
   rtc: WebRtcTask 
}