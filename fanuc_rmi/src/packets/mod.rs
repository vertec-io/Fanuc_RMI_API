mod command;
mod communication;
mod instruction;

pub use command::*;
pub use communication::*;
pub use instruction::*;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum SendPacket {
    Communication(Communication),
    Command(Command),
    Instruction(Instruction)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum ResponsePacket {
    CommunicationResponse(CommunicationResponse),
    CommandResponse(CommandResponse),
    InstructionResponse(InstructionResponse)
}

pub trait Packet: Serialize + for<'de> Deserialize<'de> {}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PacketPriority{
    Low,
    Standard,
    High,
    Immediate,
    Termination,
}

