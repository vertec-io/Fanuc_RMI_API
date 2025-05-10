use serde::{Deserialize, Serialize};
use crate::{Configuration, Position, SpeedType, TermType};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcJointMotion {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
    #[serde(rename = "Configuration")]
    pub configuration: Configuration,
    #[serde(rename = "Position")]
    pub position: Position,
    #[serde(rename = "SpeedType")]
    pub speed_type: SpeedType,
    #[serde(rename = "Speed")]
    pub speed: f64,
    #[serde(rename = "TermType")]
    pub term_type: TermType,
    #[serde(rename = "TermValue")]
    pub term_value: u8,
}


impl FrcJointMotion{
    pub fn new(
        sequence_id: u32,
        configuration: Configuration,
        position: Position,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,

    ) -> Self {
        Self {
            sequence_id,
            configuration,
            position,
            speed_type,
            speed,
            term_type,
            term_value,
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcJointMotionResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}