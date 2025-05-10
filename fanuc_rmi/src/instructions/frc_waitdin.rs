use serde::{Deserialize, Serialize};
use crate::packets::OnOff;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWaitDIN {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
    #[serde(rename = "PortNumber")]
    pub port_number: u32,
    #[serde(rename = "PortValue")]
    pub port_value: OnOff,
}


impl FrcWaitDIN{
    #[allow(unused)]
    pub fn new(sequence_id:u32,port_number:u32,port_value:OnOff) -> Self {
        Self {
            sequence_id,
            port_number,
            port_value,
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWaitDINResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}