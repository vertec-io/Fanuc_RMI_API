use serde::{Deserialize, Serialize};
use crate::FrameData;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcWriteUFrameData {
    #[serde(rename = "FrameNumber")]
    frame_number: i8,    
    #[serde(rename = "Frame")]
    frame: FrameData,
    #[serde(rename = "Group")]
    group: u8,
}


impl FrcWriteUFrameData{
    #[allow(unused)]
    fn new(groupentered: Option<u8>, framenum:i8, framespecs:FrameData) -> Self {
        Self {
            group: match groupentered {
                Some(gm) => gm,
                None => 1
            },
            frame_number: framenum,
            frame: framespecs
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteUFrameDataResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "Group")]
    group: u8,
}