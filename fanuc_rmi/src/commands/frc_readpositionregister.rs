use serde::{Deserialize, Serialize};
use crate::{Configuration, Position};


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadPositionRegister {
    #[serde(rename = "Group")]
    group: u8,
    #[serde(rename = "RegisterNumber")]
    register_number: u16,
}


impl FrcReadPositionRegister{
    #[allow(unused)]
    fn new(groupentered: Option<u8>, register:u16) -> Self {
        Self {
            group: match groupentered {
                Some(gm) => gm,
                None => 1
            },
            register_number: register
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadPositionRegisterResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "RegisterNumber")]
    pub register_number: i16,
    #[serde(rename = "Configuration")]
    pub config: Configuration,
    #[serde(rename = "Position")]
    pub position: Position,
    #[serde(rename = "Group")]
    pub group: i16,


}