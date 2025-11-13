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

// Packets - Top-level enums
pub use crate::packets::SendPacketDto as SendPacket;
pub use crate::packets::ResponsePacketDto as ResponsePacket;

// Packets - Response enums (re-exported with same names for client convenience)
pub use crate::packets::CommandDto as Command;
pub use crate::packets::CommandResponseDto as CommandResponse;
pub use crate::packets::InstructionDto as Instruction;
pub use crate::packets::InstructionResponseDto as InstructionResponse;
pub use crate::packets::CommunicationDto as Communication;
pub use crate::packets::CommunicationResponseDto as CommunicationResponse;

