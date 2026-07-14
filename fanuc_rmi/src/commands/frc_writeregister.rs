use serde::{Deserialize, Serialize};

/// Write a **numeric register** (`R[n]`) on the controller.
///
/// See [`super::FrcReadRegister`] for the coordinated-motion parameter-channel
/// rationale and the ⚠️ portability caveat: `FRC_WriteRegister` is **not** in the
/// base RMI command set (UNTESTED across controllers). Prefer group I/O or
/// position registers where portability matters.
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcWriteRegister {
    #[serde(rename = "RegisterNumber")]
    pub register_number: u16,
    /// Value to write. Carried as `f32` so both integer and real registers are
    /// representable.
    #[serde(rename = "RegisterValue")]
    pub register_value: f32,
}

impl FrcWriteRegister {
    #[allow(unused)]
    pub fn new(register_number: u16, register_value: f32) -> Self {
        Self {
            register_number,
            register_value,
        }
    }
}

/// Response for [`FrcWriteRegister`].
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcWriteRegisterResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
}
