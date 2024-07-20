use serde::{Deserialize, Serialize};
use crate::{Configuration, Position, SpeedType, TermType};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcJointMotion {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,    
    #[serde(rename = "Configuration")]
    configuration: Configuration,
    #[serde(rename = "Position")]
    position: Position,
    #[serde(rename = "SpeedType")]
    speed_type: SpeedType,
    #[serde(rename = "Speed")]
    speed: f64,
    #[serde(rename = "TermType")]
    term_type: TermType,
    #[serde(rename = "TermValue")]
    term_value: u8,
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