use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcGetUFrameUTool {
    #[serde(rename = "Group")]
    group: u8,
}


impl FrcGetUFrameUTool{
    #[allow(unused)]
    fn new(groupentered: Option<u8>) -> Self {
        Self {
            group: match groupentered {
                Some(gm) => gm,
                None => 1
            },
        }

    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcGetUFrameUToolResponse { 
    #[serde(rename = "UFrameNumber")]
    uframe_number: u8,
    #[serde(rename = "UToolNumber")]
    utool_number: u8,
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "Group")]
    pub group: u16,

}