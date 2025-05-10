use serde::{Deserialize, Serialize};
use crate::FrameData;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcWriteUToolData {
    #[serde(rename = "ToolNumber")]
    pub tool_number: i8,
    #[serde(rename = "Frame")]
    pub frame: FrameData,
    #[serde(rename = "Group")]
    pub group: u8,
}

impl FrcWriteUToolData{
    #[allow(unused)]
    pub fn new(group: Option<u8>, tool_number:i8, frame:FrameData) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },
            tool_number,
            frame
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteUToolDataResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "Group")]
    pub group: u8,
}