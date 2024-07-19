use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetUTool {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
    #[serde(rename = "ToolNumber")]
    tool_number: u8,

}

 
impl FrcSetUTool{
    #[allow(unused)]
    fn new(seq:u32, tool_num:u8) -> Self {
        Self {
            sequence_id: seq,
            tool_number: tool_num,
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