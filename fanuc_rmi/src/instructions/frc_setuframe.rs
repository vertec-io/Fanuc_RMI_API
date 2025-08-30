use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUFrame {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
    #[serde(rename = "FrameNumber")]
    pub frame_number: u8,

}


impl FrcSetUFrame{
    #[allow(unused)]
    pub fn new(sequence_id:u32, frame_number:u8) -> Self {
        Self {
            sequence_id,
            frame_number,
        }

    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUFrameResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}