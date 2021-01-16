
use uuid::Uuid;
use anyhow::{Result,Context};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Action {
    List,
    Offer { sdp: String },    
    Answer { sdp: String },    
    NewIceCandidate { candidate: String }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
