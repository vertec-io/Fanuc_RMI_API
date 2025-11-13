use super::Packet;
use crate::instructions::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "Instruction")]
pub enum Instruction {
    #[serde(rename = "FRC_WaitDIN")]
    FrcWaitDIN(FrcWaitDIN), // Wait for DIN Instruction

    #[serde(rename = "FRC_SetUFrame")]
    FrcSetUFrame(FrcSetUFrame), // Set User Frame Instruction

    #[serde(rename = "FRC_SetUTool")]
    FrcSetUTool(FrcSetUTool), // Set User Tool Instruction

    #[serde(rename = "FRC_WaitTime")]
    FrcWaitTime(FrcWaitTime), // Add Wait Time Instruction

    #[serde(rename = "FRC_SetPayLoad")]
    FrcSetPayLoad(FrcSetPayLoad), // Set Payload Instruction

    #[serde(rename = "FRC_Call")]
    FrcCall(FrcCall), // Call a Program

    #[serde(rename = "FRC_LinearMotion")]
    FrcLinearMotion(FrcLinearMotion), // Add Linear Motion Instruction

    #[serde(rename = "FRC_LinearRelative")]
    FrcLinearRelative(FrcLinearRelative), // Add Linear Incremental Motion Instruction

    #[serde(rename = "FRC_LinearRelativeJRep")]
    FrcLinearRelativeJRep(FrcLinearRelativeJRep), // Add Linear Relative Motion with Joint Representation

    #[serde(rename = "FRC_JointMotion")]
    FrcJointMotion(FrcJointMotion), // Add Joint Motion Instruction

    #[serde(rename = "FRC_JointRelative")]
    FrcJointRelative(FrcJointRelative), // Add Joint Incremental Motion Instruction

    #[serde(rename = "FRC_CircularMotion")]
    FrcCircularMotion(FrcCircularMotion), // Add Circular Motion Instruction

    #[serde(rename = "FRC_CircularRelative")]
    FrcCircularRelative(FrcCircularRelative), // Add Circular Incremental Motion Instruction

    #[serde(rename = "FRC_JointMotionJRep")]
    FrcJointMotionJRep(FrcJointMotionJRep), // Add Joint Motion with Joint Representation

    #[serde(rename = "FRC_JointRelativeJRep")]
    FrcJointRelativeJRep(FrcJointRelativeJRep), // Add Joint Incremental Motion with Joint Representation

    #[serde(rename = "FRC_LinearMotionJRep")]
    FrcLinearMotionJRep(FrcLinearMotionJRep), // Add Linear Motion with Joint Representation
}

impl Instruction {
    pub fn get_sequence_id(&self) -> u32 {
        match self {
            Instruction::FrcWaitDIN(resp) => resp.sequence_id,
            Instruction::FrcSetUFrame(resp) => resp.sequence_id,
            Instruction::FrcSetUTool(resp) => resp.sequence_id,
            Instruction::FrcWaitTime(resp) => resp.sequence_id,
            Instruction::FrcSetPayLoad(resp) => resp.sequence_id,
            Instruction::FrcCall(resp) => resp.sequence_id,
            Instruction::FrcLinearMotion(resp) => resp.sequence_id,
            Instruction::FrcLinearRelative(resp) => resp.sequence_id,
            Instruction::FrcLinearRelativeJRep(resp) => resp.sequence_id,
            Instruction::FrcJointMotion(resp) => resp.sequence_id,
            Instruction::FrcJointRelative(resp) => resp.sequence_id,
            Instruction::FrcCircularMotion(resp) => resp.sequence_id,
            Instruction::FrcCircularRelative(resp) => resp.sequence_id,
            Instruction::FrcJointMotionJRep(resp) => resp.sequence_id,
            Instruction::FrcJointRelativeJRep(resp) => resp.sequence_id,
            Instruction::FrcLinearMotionJRep(resp) => resp.sequence_id,
        }
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "Instruction")]
pub enum InstructionResponse {
    #[serde(rename = "FRC_WaitDIN")]
    FrcWaitDIN(FrcWaitDINResponse),
    #[serde(rename = "FRC_SetUFrame")]
    FrcSetUFrame(FrcSetUFrameResponse),
    #[serde(rename = "FRC_SetUTool")]
    FrcSetUTool(FrcSetUToolResponse),
    #[serde(rename = "FRC_WaitTime")]
    FrcWaitTime(FrcWaitTimeResponse),
    #[serde(rename = "FRC_SetPayLoad")]
    FrcSetPayLoad(FrcSetPayLoadResponse),
    #[serde(rename = "FRC_Call")]
    FrcCall(FrcCallResponse),
    #[serde(rename = "FRC_LinearMotion")]
    FrcLinearMotion(FrcLinearMotionResponse),
    #[serde(rename = "FRC_LinearRelative")]
    FrcLinearRelative(FrcLinearRelativeResponse),
    #[serde(rename = "FRC_LinearRelativeJRep")]
    FrcLinearRelativeJRep(FrcLinearRelativeJRepResponse),
    #[serde(rename = "FRC_JointMotion")]
    FrcJointMotion(FrcJointMotionResponse),
    #[serde(rename = "FRC_JointRelative")]
    FrcJointRelative(FrcJointRelativeResponse),
    #[serde(rename = "FRC_CircularMotion")]
    FrcCircularMotion(FrcCircularMotionResponse),
    #[serde(rename = "FRC_CircularRelative")]
    FrcCircularRelative(FrcCircularRelativeResponse),
    #[serde(rename = "FRC_JointMotionJRep")]
    FrcJointMotionJRep(FrcJointMotionJRepResponse),
    #[serde(rename = "FRC_JointRelativeJRep")]
    FrcJointRelativeJRep(FrcJointRelativeJRepResponse),
    #[serde(rename = "FRC_LinearMotionJRep")]
    FrcLinearMotionJRep(FrcLinearMotionJRepResponse),
}

impl InstructionResponse {
    pub fn get_sequence_id(&self) -> u32 {
        match self {
            InstructionResponse::FrcWaitDIN(resp) => resp.sequence_id,
            InstructionResponse::FrcSetUFrame(resp) => resp.sequence_id,
            InstructionResponse::FrcSetUTool(resp) => resp.sequence_id,
            InstructionResponse::FrcWaitTime(resp) => resp.sequence_id,
            InstructionResponse::FrcSetPayLoad(resp) => resp.sequence_id,
            InstructionResponse::FrcCall(resp) => resp.sequence_id,
            InstructionResponse::FrcLinearMotion(resp) => resp.sequence_id,
            InstructionResponse::FrcLinearRelative(resp) => resp.sequence_id,
            InstructionResponse::FrcLinearRelativeJRep(resp) => resp.sequence_id,
            InstructionResponse::FrcJointMotion(resp) => resp.sequence_id,
            InstructionResponse::FrcJointRelative(resp) => resp.sequence_id,
            InstructionResponse::FrcCircularMotion(resp) => resp.sequence_id,
            InstructionResponse::FrcCircularRelative(resp) => resp.sequence_id,
            InstructionResponse::FrcJointMotionJRep(resp) => resp.sequence_id,
            InstructionResponse::FrcJointRelativeJRep(resp) => resp.sequence_id,
            InstructionResponse::FrcLinearMotionJRep(resp) => resp.sequence_id,
        }
    }
}
impl InstructionResponse {
    pub fn get_error_id(&self) -> u32 {
        match self {
            InstructionResponse::FrcWaitDIN(resp) => resp.error_id,
            InstructionResponse::FrcSetUFrame(resp) => resp.error_id,
            InstructionResponse::FrcSetUTool(resp) => resp.error_id,
            InstructionResponse::FrcWaitTime(resp) => resp.error_id,
            InstructionResponse::FrcSetPayLoad(resp) => resp.error_id,
            InstructionResponse::FrcCall(resp) => resp.error_id,
            InstructionResponse::FrcLinearMotion(resp) => resp.error_id,
            InstructionResponse::FrcLinearRelative(resp) => resp.error_id,
            InstructionResponse::FrcLinearRelativeJRep(resp) => resp.error_id,
            InstructionResponse::FrcJointMotion(resp) => resp.error_id,
            InstructionResponse::FrcJointRelative(resp) => resp.error_id,
            InstructionResponse::FrcCircularMotion(resp) => resp.error_id,
            InstructionResponse::FrcCircularRelative(resp) => resp.error_id,
            InstructionResponse::FrcJointMotionJRep(resp) => resp.error_id,
            InstructionResponse::FrcJointRelativeJRep(resp) => resp.error_id,
            InstructionResponse::FrcLinearMotionJRep(resp) => resp.error_id,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletedPacketReturnInfo {
    pub sequence_id: u32,
    pub error_id: u32,
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum OnOff {
    ON,
    OFF,
}

impl Packet for Instruction {}

// ExtractInner trait implementations for InstructionResponse
impl_extract_inner!(InstructionResponse, FrcWaitDIN, FrcWaitDINResponse);
impl_extract_inner!(InstructionResponse, FrcSetUFrame, FrcSetUFrameResponse);
impl_extract_inner!(InstructionResponse, FrcSetUTool, FrcSetUToolResponse);
impl_extract_inner!(InstructionResponse, FrcWaitTime, FrcWaitTimeResponse);
impl_extract_inner!(InstructionResponse, FrcSetPayLoad, FrcSetPayLoadResponse);
impl_extract_inner!(InstructionResponse, FrcCall, FrcCallResponse);
impl_extract_inner!(InstructionResponse, FrcLinearMotion, FrcLinearMotionResponse);
impl_extract_inner!(InstructionResponse, FrcLinearRelative, FrcLinearRelativeResponse);
impl_extract_inner!(InstructionResponse, FrcLinearRelativeJRep, FrcLinearRelativeJRepResponse);
impl_extract_inner!(InstructionResponse, FrcJointMotion, FrcJointMotionResponse);
impl_extract_inner!(InstructionResponse, FrcJointRelative, FrcJointRelativeResponse);
impl_extract_inner!(InstructionResponse, FrcCircularMotion, FrcCircularMotionResponse);
impl_extract_inner!(InstructionResponse, FrcCircularRelative, FrcCircularRelativeResponse);
impl_extract_inner!(InstructionResponse, FrcJointMotionJRep, FrcJointMotionJRepResponse);
impl_extract_inner!(InstructionResponse, FrcJointRelativeJRep, FrcJointRelativeJRepResponse);
impl_extract_inner!(InstructionResponse, FrcLinearMotionJRep, FrcLinearMotionJRepResponse);

