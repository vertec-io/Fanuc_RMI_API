use serde::{Deserialize, Serialize};

/// Read Analog Input (AIN) command
/// Reads the value of an analog input port on the robot controller.
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadAIN {
    #[serde(rename = "PortNumber")]
    pub port_number: u16,
}

impl FrcReadAIN {
    #[allow(unused)]
    pub fn new(port_number: u16) -> Self {
        Self { port_number }
    }
}

/// Response for FrcReadAIN command
/// Contains the analog value (typically 0-4095 for 12-bit ADC, or scaled value)
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadAINResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "PortNumber")]
    pub port_number: u16,
    #[serde(rename = "PortValue")]
    pub port_value: f64,
}

