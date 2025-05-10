use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetOverride {
    #[serde(rename = "Value")]
    pub value: u8,
}

impl FrcSetOverride{
    #[allow(unused)]
    pub fn new(value: u8) -> Self {
        Self {
            value
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetOverrideResponse {   
    #[serde(rename = "ErrorID")]
    error_id: u16,
}