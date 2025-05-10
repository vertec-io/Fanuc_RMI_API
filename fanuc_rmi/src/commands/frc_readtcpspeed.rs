use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadTCPSpeedResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "TimeTag")]
    pub time_tag: u32,
    #[serde(rename = "Speed")]
    pub speed: f32,
}