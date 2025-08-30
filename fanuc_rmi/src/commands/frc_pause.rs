use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcPauseResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
}