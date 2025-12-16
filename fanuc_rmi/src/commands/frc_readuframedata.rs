use serde::{Deserialize, Serialize};
use crate::FrameData;

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadUFrameData {
    #[serde(rename = "FrameNumber")]
    pub frame_number: i8,
    #[serde(rename = "Group")]
    pub group: u8,
}

impl FrcReadUFrameData{
    #[allow(unused)]
    pub fn new(group: Option<u8>, frame_number:i8) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },
            frame_number,
        }
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadUFrameDataResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    /// Note: Manual says "UFrameNumber" but real robot sends "FrameNumber"
    /// Per documentation: byteValue (u8, 0-255)
    #[serde(rename = "FrameNumber")]
    pub frame_number: u8,
    /// Per documentation: byteValue2 (u8)
    #[serde(rename = "Group")]
    pub group: u8,
    #[serde(rename = "Frame")]
    pub frame: FrameData,
}