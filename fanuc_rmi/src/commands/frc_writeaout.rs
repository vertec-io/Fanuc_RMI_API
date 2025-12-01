use serde::{Deserialize, Serialize};

/// Write Analog Output (AOUT) command
/// Writes a value to an analog output port on the robot controller.
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcWriteAOUT {
    #[serde(rename = "PortNumber")]
    pub port_number: u16,
    #[serde(rename = "PortValue")]
    pub port_value: f64,
}

impl FrcWriteAOUT {
    #[allow(unused)]
    pub fn new(port_number: u16, port_value: f64) -> Self {
        Self {
            port_number,
            port_value,
        }
    }
}

/// Response for FrcWriteAOUT command
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteAOUTResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
}

