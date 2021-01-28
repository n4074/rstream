
use uuid::Uuid;
use anyhow::{Result,Context};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Signal {
    Offer { sdp: String },    
    Answer { sdp: String },    
    NewIceCandidate { candidate: String }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ClientMsg {
    Signal { signal: Signal, sender: Uuid },
    ListPeers { peers: Vec<Uuid> }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub struct PeerList {
    pub peers: Vec<Uuid>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServerMsg {
    Signal { signal: Signal, recipient: Uuid },
    ListPeers
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
