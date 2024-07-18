use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcInitialize {
    #[serde(rename = "GroupMask")]
    group_mask: u8,
}


impl FrcInitialize{
    fn new(groupmask: Option<u8>) -> Self {
        let groupmask = match groupmask {
            Some(gm) => gm,
            None => 1
        };

        Self {
            group_mask: groupmask
        }

    }
}

impl Default for FrcInitialize {
    fn default() -> Self {
        FrcInitialize::new(Some(1))
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcInitializeResponse { 
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "GroupMask")]
    pub group_mask: u16,

}