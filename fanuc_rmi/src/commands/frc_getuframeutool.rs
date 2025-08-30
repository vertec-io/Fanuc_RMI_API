use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcGetUFrameUTool {
    #[serde(rename = "Group")]
    pub group: u8,
}

impl FrcGetUFrameUTool{
    #[allow(unused)]
    pub fn new(group: Option<u8>) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },
        }

    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcGetUFrameUToolResponse {
    #[serde(rename = "UFrameNumber")]
    pub u_frame_number: u8,
    #[serde(rename = "UToolNumber")]
    pub u_tool_number: u8,
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "Group")]
    pub group: u16,
}