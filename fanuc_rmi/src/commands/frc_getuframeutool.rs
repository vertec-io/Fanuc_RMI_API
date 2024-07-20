use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcGetUFrameUTool {
    #[serde(rename = "Group")]
    group: u8,
}


impl FrcGetUFrameUTool{
    #[allow(unused)]
    fn new(group: Option<u8>) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },
        }

    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcGetUFrameUToolResponse { 
    #[serde(rename = "UFrameNumber")]
    u_frame_number: u8,
    #[serde(rename = "UToolNumber")]
    u_tool_number: u8,
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "Group")]
    pub group: u16,

}