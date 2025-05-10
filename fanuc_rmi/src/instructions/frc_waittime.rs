use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcWaitTime {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
    #[serde(rename = "Time")]
    pub time: f32,

}


impl FrcWaitTime{
    #[allow(unused)]
    pub fn new(sequence_id:u32, time:f32) -> Self {
        Self {
            sequence_id,
            time,
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWaitTimeResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}