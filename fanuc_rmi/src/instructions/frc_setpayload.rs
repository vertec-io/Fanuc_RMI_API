use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetPayLoad {
    #[serde(rename = "SequenceID")]
    sequence_id: i32,
    #[serde(rename = "ScheduleNumber")]
    schedule_number: u8,

}

 
impl FrcSetPayLoad{
    #[allow(unused)]

    fn new(seq:i32, schedule_num:u8) -> Self {
        Self {
            sequence_id: seq,
            schedule_number: schedule_num,
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetPayLoadResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}