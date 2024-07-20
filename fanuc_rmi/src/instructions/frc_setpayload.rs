use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSetPayLoad {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
    #[serde(rename = "ScheduleNumber")]
    schedule_number: u8,

}

 
impl FrcSetPayLoad{
    #[allow(unused)]

    fn new(sequence_id:u32, schedule_number:u8) -> Self {
        Self {
            sequence_id,
            schedule_number,
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