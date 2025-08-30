// Aggregated top-level DTO namespace for all mirrored types
// Users can `use fanuc_rmi::dto::*` for a clean import path.

pub use crate::FrameDataDto as FrameData;
pub use crate::ConfigurationDto as Configuration;
pub use crate::PositionDto as Position;
pub use crate::JointAnglesDto as JointAngles;

// Commands
pub use crate::commands::dto::*;
// Instructions
pub use crate::instructions::dto::*;

// Packets
pub use crate::packets::SendPacketDto as SendPacket;
pub use crate::packets::ResponsePacketDto as ResponsePacket;

