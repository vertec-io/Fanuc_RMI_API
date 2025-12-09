use serde::{Deserialize, Serialize};

/// Response for unknown/unrecognized commands
/// 
/// When the robot receives a command it doesn't recognize or support,
/// it responds with {"Command": "Unknown", "ErrorID": <error_code>}
/// 
/// Common error codes:
/// - 2556950 (InvalidTextString): Command name not recognized
/// - 2556941 (InvalidRMICommand): Command not supported in current RMI version
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcUnknownResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
}

