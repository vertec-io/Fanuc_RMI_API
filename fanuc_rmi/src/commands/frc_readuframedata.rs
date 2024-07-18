use serde::{Deserialize, Serialize};
use crate::FrameData;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadUFrameData {
    #[serde(rename = "FrameNumber")]
    frame_number: i8,    
    #[serde(rename = "Group")]
    group: u8,
}


impl FrcReadUFrameData{
    #[allow(unused)]
    fn new(groupentered: Option<u8>, frame:i8) -> Self {
        Self {
            group: match groupentered {
                Some(gm) => gm,
                None => 1
            },
            frame_number: frame,
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadUFrameDataResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "UFrameNumber")]
    pub uframe_number: i8,
    #[serde(rename = "Group")]
    group: u8,
    #[serde(rename = "Frame")]
    frame: FrameData,


}