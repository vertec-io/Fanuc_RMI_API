use serde::{Deserialize, Serialize};
use crate::{Configuration, Position};


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcWritePositionRegister {
    #[serde(rename = "RegisterNumber")]
    register_number: u16,
    #[serde(rename = "Configuration")]
    congifuration: Configuration,
    #[serde(rename = "Position")]
    pub position: Position,
    #[serde(rename = "Group")]
    pub group: u8,
}


impl FrcWritePositionRegister{
    #[allow(unused)]
    fn new(group: Option<u8>, register_number:u16, configuration:Configuration , position:Position) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },
            register_number,
            position,
            congifuration: configuration
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWritePositionRegisterResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
}