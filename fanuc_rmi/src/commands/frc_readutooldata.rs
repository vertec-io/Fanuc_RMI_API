use serde::{Deserialize, Serialize};
use crate::FrameData;

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadUToolData {
    #[serde(rename = "ToolNumber")]
    pub tool_number: i8,
    #[serde(rename = "Group")]
    pub group: u8,
}

impl FrcReadUToolData{
    #[allow(unused)]
    pub fn new(group: Option<u8>, tool_number: i8) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },
            tool_number
        }
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadUToolDataResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    /// Note: Manual says "UToolNumber" but real robot sends "ToolNumber"
    /// Using i16 because robot may return values > 127 in error states
    #[serde(rename = "ToolNumber")]
    pub tool_number: i16,
    #[serde(rename = "Frame")]
    pub frame: FrameData,
    /// Using u16 because robot may return unexpected values in error states
    #[serde(rename = "Group")]
    pub group: u16,
}