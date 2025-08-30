use serde::{Deserialize, Serialize};
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteDOUT{
    #[serde(rename = "PortNumber")]
    pub port_number: u16,
    #[serde(rename = "PortValue")]
    pub port_value: u8,
}

impl FrcWriteDOUT{
    #[allow(unused)]
    pub fn new(port_num: u16,port_val: u8) -> Self {
        Self {
            port_number: port_num,
            port_value: port_val
        }
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteDOUTResponse {    
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
}