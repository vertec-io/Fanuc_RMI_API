mod command;
mod communication;
mod instruction;
mod driver_command;

pub use command::*;
pub use communication::*;
pub use instruction::*;
pub use driver_command::*;

use serde::{Deserialize, Serialize};

use crate::drivers::DriverPacket;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum SendPacket {
    Communication(Communication),
    Command(Command),
    Instruction(Instruction),
    DriverCommand(DriverCommand)
}

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



