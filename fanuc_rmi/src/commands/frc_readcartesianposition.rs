use serde::{Deserialize, Serialize};
// Ensure the mirror_dto attribute is in scope when DTO feature is on
#[cfg(feature = "DTO")]
pub use crate::mirror_dto;

use crate::{Configuration, Position};

#[cfg_attr(feature = "DTO", mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadCartesianPosition {
    #[serde(rename = "Group")]
    pub group: u8,
}
impl FrcReadCartesianPosition{
    #[allow(unused)]
    pub fn new(group: Option<u8>, ) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },        }

    }
}

#[cfg_attr(feature = "DTO", mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadCartesianPositionResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "TimeTag")]
    pub time_tag: u32,
    #[serde(rename = "Configuration")]
    pub config: Configuration,
    #[serde(rename = "Position")]
    pub pos: Position,
    #[serde(rename = "Group")]
    pub group: u8,
}