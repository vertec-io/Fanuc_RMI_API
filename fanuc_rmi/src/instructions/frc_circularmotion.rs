use serde::{Deserialize, Serialize};
use crate::{Configuration, Position, SpeedType, TermType};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcCircularMotion {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,    
    #[serde(rename = "Configuration")]
    pub configuration: Configuration,
    #[serde(rename = "Position")]
    pub position: Position,
    #[serde(rename = "ViaConfiguration")]
    pub via_configuration: Configuration,
    #[serde(rename = "ViaPosition")]
    pub via_position: Position,
    #[serde(rename = "SpeedType")]
    pub speed_type: SpeedType,
    #[serde(rename = "Speed")]
    pub speed: f64,
    #[serde(rename = "TermType")]
    pub term_type: TermType,
    #[serde(rename = "TermValue")]
    pub term_value: u8,
}

impl FrcCircularMotion{
    pub fn new(    
        sequence_id: u32,    
        configuration: Configuration,
        position: Position,
        via_configuration: Configuration,
        via_position: Position,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        Self {
            sequence_id,    
            configuration,
            position,
            via_configuration,
            via_position,
            speed_type,
            speed,
            term_type,
            term_value,
        }
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcCircularMotionResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}
