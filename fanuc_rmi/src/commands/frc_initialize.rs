use serde::{Deserialize, Serialize};
use crate::GroupMask;

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcInitialize {
    /// Motion-group selection bitmask reserved by this RMI session.
    ///
    /// Defaults to [`GroupMask::GROUP_1`]. Pass a multi-group mask
    /// (e.g. `GroupMask::GROUP_1 | GroupMask::GROUP_2`) to reserve a coordinated
    /// positioner as well — but see the [`GroupMask`] docs: reserving Group 2 does
    /// **not** make RMI motion packets drive it; coordinated motion still requires
    /// a controller-resident TP/COORD program.
    #[serde(rename = "GroupMask")]
    pub group_mask: GroupMask,
}

impl FrcInitialize{
    /// Create an initialize command. `None` selects the default (Group 1 only).
    pub fn new(group_mask: Option<GroupMask>) -> Self {
        Self {
            group_mask: group_mask.unwrap_or_default(),
        }
    }

    /// Convenience constructor from a raw bitmask byte (bit N-1 selects group N).
    pub fn from_bits(group_mask: u8) -> Self {
        Self {
            group_mask: GroupMask::from_bits(group_mask),
        }
    }
}

impl Default for FrcInitialize {
    fn default() -> Self {
        FrcInitialize::new(None)
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcInitializeResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "GroupMask", default)]
    pub group_mask: u16,
}
