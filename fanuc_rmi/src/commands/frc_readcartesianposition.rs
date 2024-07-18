use serde::{Deserialize, Serialize};
use crate::{Configuration, Position};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadCartesianPosition {
    #[serde(rename = "Group")]
    group: u8,
}


impl FrcReadCartesianPosition{
    #[allow(unused)]
    fn new(groupentered: Option<u8>, ) -> Self {
        Self {
            group: match groupentered {
                Some(gm) => gm,
                None => 1
            },        }

    }
}

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
    group: u8,



}