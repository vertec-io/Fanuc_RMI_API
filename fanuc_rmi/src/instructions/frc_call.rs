use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcCall {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
    #[serde(rename = "ProgramName")]
    pub program_name: String,

}


impl FrcCall{
    #[allow(unused)]
    pub fn new(seq:u32, program:String) -> Self {
        Self {
            sequence_id: seq,
            program_name: program,
        }

    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcCallResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}