use serde::{Deserialize, Serialize};
use crate::FrameData;

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcWriteUFrameData {
    #[serde(rename = "FrameNumber")]
    pub frame_number: i8,
    #[serde(rename = "Frame")]
    pub frame: FrameData,
    #[serde(rename = "Group")]
    pub group: u8,
}

impl FrcWriteUFrameData{
    #[allow(unused)]
    pub fn new(group: Option<u8>, frame_number:i8, frame:FrameData) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },
            frame_number,
            frame
        }
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteUFrameDataResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    /// Using u16 because robot may return unexpected values in error states
    #[serde(rename = "Group")]
    pub group: u16,
}