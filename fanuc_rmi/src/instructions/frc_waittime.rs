use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcWaitTime {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
    #[serde(rename = "Time")]
    time: f32,

}

 
impl FrcWaitTime{
    #[allow(unused)]
    fn new(seq:u32, time:f32) -> Self {
        Self {
            sequence_id: seq,
            time: time,
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