use serde::{Deserialize, Serialize};

/// Read Group Input (GIN) command
/// Reads the value of a group input port on the robot controller.
/// Group I/O allows reading/writing multiple bits as a single value.
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadGIN {
    #[serde(rename = "PortNumber")]
    pub port_number: u16,
}

impl FrcReadGIN {
    #[allow(unused)]
    pub fn new(port_number: u16) -> Self {
        Self { port_number }
    }
}

/// Response for FrcReadGIN command
/// Contains the group value (typically 0-255 for 8-bit groups, or larger for wider groups)
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadGINResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "PortNumber")]
    pub port_number: u16,
    #[serde(rename = "PortValue")]
    pub port_value: u32,
}

