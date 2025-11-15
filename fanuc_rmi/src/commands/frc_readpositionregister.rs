use serde::{Deserialize, Serialize};
use crate::{Configuration, Position};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadPositionRegister {
    #[serde(rename = "Group")]
    pub group: u8,
    #[serde(rename = "RegisterNumber")]
    pub register_number: u16,
}
impl FrcReadPositionRegister{
    #[allow(unused)]
    pub fn new(group: Option<u8>, register_number:u16) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },
            register_number
        }
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
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