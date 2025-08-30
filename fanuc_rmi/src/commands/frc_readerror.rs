use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadError {
    #[serde(rename = "Count")]
    pub count: u8,
}

impl FrcReadError{
    pub fn new(count: Option<u8>) -> Self {
        let count = match count {
            Some(gm) => gm,
            None => 1
        };
        Self {
            count
        }
    }
}

impl Default for FrcReadError {
    fn default() -> Self {
        FrcReadError::new(Some(1))
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadErrorResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u16,
    #[serde(rename = "Count")]
    pub count: u8,
    #[serde(rename = "ErrorData")]
    pub error_data: String
}