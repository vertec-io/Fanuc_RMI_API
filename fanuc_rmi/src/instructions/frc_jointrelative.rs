use serde::{Deserialize, Serialize};
use crate::{Configuration, Position, SpeedType, TermType};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcJointRelative {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,    
    #[serde(rename = "Configuration")]
    configuration: Configuration,
    #[serde(rename = "Position")]
    position: Position,
    #[serde(rename = "SpeedType")]
    speed_type: SpeedType,
    #[serde(rename = "Speed")]
    speed: u16,
    #[serde(rename = "TermType")]
    term_type: TermType,
    #[serde(rename = "TermValue")]
    term_value: u8,
}


impl FrcJointRelative{
    pub fn new(    
        sequenceid: u32,    
        config: Configuration,
        pos: Position,
        speed_t: SpeedType,
        speed: u16,
        term_t: TermType,
        term_va: u8,
    
    ) -> Self {
        Self {
            sequence_id: sequenceid,    
            configuration: config,
            position: pos,
            speed_type: speed_t,
            speed: speed,
            term_type: term_t,
            term_value: term_va,
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcJointRelativeResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}