use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetOverRide {
    #[serde(rename = "Value")]
    pub value: u8,
}

impl FrcSetOverRide {
    #[allow(unused)]
    pub fn new(value: u8) -> Self {
        Self { value }
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetOverRideResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u16,
}
