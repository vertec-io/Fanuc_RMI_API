use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcInitialize {
    #[serde(rename = "GroupMask")]
    pub group_mask: u8,
}

impl FrcInitialize{
    pub fn new(group_mask: Option<u8>) -> Self {
        let group_mask = match group_mask {
            Some(gm) => gm,
            None => 1
        };

        Self {
            group_mask
        }
    }
}

impl Default for FrcInitialize {
    fn default() -> Self {
        FrcInitialize::new(Some(1))
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcInitializeResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "GroupMask")]
    pub group_mask: u16,
}
