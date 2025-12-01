use serde::{Deserialize, Serialize};

/// Write Group Output (GOUT) command
/// Writes a value to a group output port on the robot controller.
/// Group I/O allows reading/writing multiple bits as a single value.
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteGOUT {
    #[serde(rename = "PortNumber")]
    pub port_number: u16,
    #[serde(rename = "PortValue")]
    pub port_value: u32,
}

impl FrcWriteGOUT {
    #[allow(unused)]
    pub fn new(port_number: u16, port_value: u32) -> Self {
        Self {
            port_number,
            port_value,
        }
    }
}

/// Response for FrcWriteGOUT command
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteGOUTResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
}

