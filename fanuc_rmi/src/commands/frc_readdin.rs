use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadDIN{
    #[serde(rename = "PortNumber")]
    port_num: u16,
}


impl FrcReadDIN{
    #[allow(unused)]
    fn new(port: u16) -> Self {
        Self {
            port_num: port
        }

    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadDINResponse {    
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "PortNumber")]
    pub port_number: u16,
    #[serde(rename = "PortValue")]
    pub port_value: u8,

}