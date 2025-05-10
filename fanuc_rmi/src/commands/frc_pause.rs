use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcPauseResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
}