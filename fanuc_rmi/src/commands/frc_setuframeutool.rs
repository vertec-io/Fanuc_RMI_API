use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUFrameUTool {
    #[serde(rename = "Group")]
    group: u8,
    #[serde(rename = "UFrameNumber")]
    uframe_number: u8,
    #[serde(rename = "UToolNumber")]
    utool_number: u8,
}


impl FrcSetUFrameUTool{
    #[allow(unused)]
    fn new(groupentered: Option<u8>, tool_num: u8, frame_num: u8 ) -> Self {
        Self {
            group: match groupentered {
                Some(gm) => gm,
                None => 1
            },
            utool_number: tool_num,
            uframe_number: frame_num
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