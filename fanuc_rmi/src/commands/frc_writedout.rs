use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteDOUT{
    #[serde(rename = "PortNumber")]
    pub port_number: u16,
    #[serde(rename = "PortValue")]
    pub port_value: u8,
}

impl FrcWriteDOUT{
    #[allow(unused)]
    fn new(port_num: u16,port_val: u8) -> Self {
        Self {
            port_number: port_num,
            port_value: port_val
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteDOUTResponse {    
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
}