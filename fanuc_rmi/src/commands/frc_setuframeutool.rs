use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUFrameUTool {
    #[serde(rename = "Group")]
    group: u8,
    #[serde(rename = "UFrameNumber")]
    u_frame_number: u8,
    #[serde(rename = "UToolNumber")]
    u_tool_number: u8,
}


impl FrcSetUFrameUTool{
    #[allow(unused)]
    fn new(group: Option<u8>, u_tool_number: u8, u_frame_number: u8 ) -> Self {
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


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUFrameUToolResponse { 

    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "Group")]
    pub group: u16,

}