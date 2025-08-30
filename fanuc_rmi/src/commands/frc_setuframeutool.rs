use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUFrameUTool {
    #[serde(rename = "Group")]
    pub group: u8,
    #[serde(rename = "UFrameNumber")]
    pub u_frame_number: u8,
    #[serde(rename = "UToolNumber")]
    pub u_tool_number: u8,
}

impl FrcSetUFrameUTool{
    #[allow(unused)]
    pub fn new(group: Option<u8>, u_tool_number: u8, u_frame_number: u8 ) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },
            u_tool_number,
            u_frame_number
        }
    }
}
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUFrameUToolResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "Group")]
    pub group: u16,
}