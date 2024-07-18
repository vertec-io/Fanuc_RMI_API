use serde::{Deserialize, Serialize};
use crate::packets::OnOff;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWaitDIN {
    #[serde(rename = "SequenceID")]
    sequence_id: i32,
    #[serde(rename = "PortNumber")]
    port_number: u32,
    #[serde(rename = "PortValue")]
    port_value: OnOff,
}

 
impl FrcWaitDIN{
    #[allow(unused)]
    fn new(seq:i32,port_num:u32,port_val:OnOff) -> Self {
        Self {
            sequence_id: seq,
            port_number: port_num,
            port_value: port_val,
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