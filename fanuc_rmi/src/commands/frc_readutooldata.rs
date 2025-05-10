use serde::{Deserialize, Serialize};
use crate::FrameData;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadUToolData {
    #[serde(rename = "FrameNumber")]
    frame_number: i8,    
    #[serde(rename = "Group")]
    group: u8,
}


impl FrcReadUToolData{
    #[allow(unused)]
    fn new(group: Option<u8>, frame_number:i8) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },
            frame_number
            }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadUToolDataResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "UToolNumber")]
    utool_number: u8,
    #[serde(rename = "Frame")]
    frame: FrameData,
    #[serde(rename = "Group")]
    group: u8,
}