use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcCall {
    #[serde(rename = "SequenceID")]
    sequence_id: i32,
    #[serde(rename = "ProgramName")]
    program_name: String,

}

 
impl FrcCall{
    #[allow(unused)]
    fn new(seq:i32, program:String) -> Self {
        Self {
            sequence_id: seq,
            program_name: program,
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcCallResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}