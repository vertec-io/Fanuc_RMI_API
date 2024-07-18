use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcResetResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
}