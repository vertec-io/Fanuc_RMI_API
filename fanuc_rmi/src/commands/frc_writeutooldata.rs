use serde::{Deserialize, Serialize};
use crate::FrameData;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcWriteUToolData {
    #[serde(rename = "ToolNumber")]
    tool_number: i8,    
    #[serde(rename = "Frame")]
    frame: FrameData,
    #[serde(rename = "Group")]
    group: u8,
}


impl FrcWriteUToolData{
    #[allow(unused)]
    fn new(groupentered: Option<u8>, toolnum:i8, framespecs:FrameData) -> Self {
        Self {
            group: match groupentered {
                Some(gm) => gm,
                None => 1
            },
            tool_number: toolnum,
            frame: framespecs
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteUToolDataResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "Group")]
    group: u8,
}