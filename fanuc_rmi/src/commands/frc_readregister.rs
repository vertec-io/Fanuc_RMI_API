use serde::{Deserialize, Serialize};

/// Read a **numeric register** (`R[n]`) on the controller.
///
/// Numeric registers are the natural *parameter channel* for a controller-resident
/// coordinated-motion (TP/COORD) program: the host writes targets / flags into
/// registers, then starts the TP program (see [`crate::instructions::FrcCall`]),
/// which reads them.
///
/// # ⚠️ Portability note (UNTESTED)
///
/// `FRC_ReadRegister` / `FRC_WriteRegister` are **not** part of the base RMI
/// command set documented in FANUC B-84184EN and are not guaranteed to be
/// accepted by every controller/firmware. A controller that does not implement
/// them replies with an `Unknown` command response (handled by this crate). For
/// maximum portability of the numeric parameter channel, prefer group I/O
/// ([`super::FrcWriteGOUT`] / [`super::FrcReadGIN`]) or **position** registers
/// ([`super::FrcReadPositionRegister`]), which are documented. Verify against your
/// controller before relying on numeric-register commands.
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadRegister {
    #[serde(rename = "RegisterNumber")]
    pub register_number: u16,
}

impl FrcReadRegister {
    #[allow(unused)]
    pub fn new(register_number: u16) -> Self {
        Self { register_number }
    }
}

/// Response for [`FrcReadRegister`].
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadRegisterResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "RegisterNumber", default)]
    pub register_number: u16,
    /// Register value. FANUC numeric registers hold either an integer or a real;
    /// this is carried as `f32` so both are representable.
    #[serde(rename = "RegisterValue", default)]
    pub register_value: f32,
}
