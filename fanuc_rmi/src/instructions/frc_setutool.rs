use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUTool {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
    #[serde(rename = "ToolNumber")]
    pub tool_number: u8,

}


impl FrcSetUTool{
    #[allow(unused)]
    pub fn new(sequence_id:u32, tool_number:u8) -> Self {
        Self {
            sequence_id,
            tool_number,
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUToolResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}