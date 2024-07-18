use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUFrame {
    #[serde(rename = "SequenceID")]
    sequence_id: i32,
    #[serde(rename = "FrameNumber")]
    frame_number: u8,

}

 
impl FrcSetUFrame{
    #[allow(unused)]
    fn new(seq:i32, frame_num:u8) -> Self {
        Self {
            sequence_id: seq,
            frame_number: frame_num,
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUFrameResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}