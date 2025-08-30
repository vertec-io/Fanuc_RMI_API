mod command;
mod communication;
mod instruction;
mod driver_command;

pub use command::*;
pub use communication::*;
pub use instruction::*;
#[cfg(feature = "DTO")]
pub use instruction::OnOffDto;

pub use driver_command::*;

use serde::{Deserialize, Serialize};







#[cfg_attr(feature = "DTO", crate::mirror_dto)]

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum SendPacket {
    Communication(Communication),
    Command(Command),
    Instruction(Instruction),
    DriverCommand(DriverCommand)
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum ResponsePacket {
    CommunicationResponse(CommunicationResponse),
    CommandResponse(CommandResponse),
    InstructionResponse(InstructionResponse),
}

pub trait Packet: Serialize + for<'de> Deserialize<'de> {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PacketPriority {
    Low,
    Standard,
    High,
    Immediate,
    Termination,
}



