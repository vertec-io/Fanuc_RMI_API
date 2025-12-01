use super::Packet;
use crate::commands::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "Command")]
pub enum Command {
    #[serde(rename = "FRC_Initialize")]
    FrcInitialize(FrcInitialize),

    #[serde(rename = "FRC_Abort")]
    FrcAbort,

    #[serde(rename = "FRC_Pause")]
    FrcPause,

    #[serde(rename = "FRC_ReadError")]
    FrcReadError(FrcReadError),

    #[serde(rename = "FRC_Continue")]
    FrcContinue,

    #[serde(rename = "FRC_SetUFrameUTool")]
    FrcSetUFrameUTool(FrcSetUFrameUTool),

    // //only requires that the remote device has made a connection to the robot controller.
    #[serde(rename = "FRC_ReadPositionRegister")]
    FrcReadPositionRegister(FrcReadPositionRegister),

    #[serde(rename = "FrcWritePositionRegister")]
    FrcWritePositionRegister(FrcWritePositionRegister),

    #[serde(rename = "FRC_SetOverRide")]
    FrcSetOverRide(FrcSetOverRide),

    #[serde(rename = "FRC_GetStatus")]
    FrcGetStatus,

    #[serde(rename = "FRC_GetUFrameUTool")]
    FrcGetUFrameUTool(FrcGetUFrameUTool),

    #[serde(rename = "FRC_WriteUToolData")]
    FrcWriteUToolData(FrcWriteUToolData),

    #[serde(rename = "FRC_ReadUToolData")]
    FrcReadUToolData(FrcReadUToolData),

    #[serde(rename = "FRC_ReadUFrameData")]
    FrcReadUFrameData(FrcReadUFrameData),

    #[serde(rename = "FRC_WriteUFrameData")]
    FrcWriteUFrameData(FrcWriteUFrameData),

    #[serde(rename = "FRC_Reset")]
    FrcReset,

    #[serde(rename = "FRC_ReadDIN")]
    FrcReadDIN(FrcReadDIN),

    #[serde(rename = "FRC_WriteDOUT")]
    FrcWriteDOUT(FrcWriteDOUT),

    #[serde(rename = "FRC_ReadAIN")]
    FrcReadAIN(FrcReadAIN),

    #[serde(rename = "FRC_WriteAOUT")]
    FrcWriteAOUT(FrcWriteAOUT),

    #[serde(rename = "FRC_ReadGIN")]
    FrcReadGIN(FrcReadGIN),

    #[serde(rename = "FRC_WriteGOUT")]
    FrcWriteGOUT(FrcWriteGOUT),

    #[serde(rename = "FRC_ReadCartesianPosition")]
    FrcReadCartesianPosition(FrcReadCartesianPosition),

    #[serde(rename = "FRC_ReadJointAngles")]
    FrcReadJointAngles(FrcReadJointAngles),

    #[serde(rename = "FRC_ReadTCPSpeed")]
    FrcReadTCPSpeed,
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "Command")]
pub enum CommandResponse {
    #[serde(rename = "FRC_Initialize")]
    FrcInitialize(FrcInitializeResponse),

    #[serde(rename = "FRC_Abort")]
    FrcAbort(FrcAbortResponse),

    #[serde(rename = "FRC_Pause")]
    FrcPause(FrcPauseResponse),

    #[serde(rename = "FRC_Continue")]
    FrcContinue(FrcContinueResponse),

    #[serde(rename = "FRC_ReadError")]
    FrcReadError(FrcReadErrorResponse),

    #[serde(rename = "FRC_SetUFrameUTool")]
    FrcSetUFrameUTool(FrcSetUFrameUToolResponse),

    #[serde(rename = "FRC_GetUFrameUTool")]
    FrcGetUFrameUTool(FrcGetUFrameUToolResponse),

    #[serde(rename = "FRC_GetStatus")]
    FrcGetStatus(FrcGetStatusResponse),

    #[serde(rename = "FRC_ReadUFrameData")]
    FrcReadUFrameData(FrcReadUFrameDataResponse),

    #[serde(rename = "FRC_WriteUFrameData")]
    FrcWriteUFrameData(FrcWriteUFrameDataResponse),

    #[serde(rename = "FRC_ReadUToolData")]
    FrcReadUToolData(FrcReadUToolDataResponse),

    #[serde(rename = "FRC_WriteUToolData")]
    FrcWriteUToolData(FrcWriteUToolData),

    #[serde(rename = "FRC_ReadDIN")]
    FrcReadDIN(FrcReadDINResponse),

    #[serde(rename = "FRC_WriteDOUT")]
    FrcWriteDOUT(FrcWriteDOUTResponse),

    #[serde(rename = "FRC_ReadAIN")]
    FrcReadAIN(FrcReadAINResponse),

    #[serde(rename = "FRC_WriteAOUT")]
    FrcWriteAOUT(FrcWriteAOUTResponse),

    #[serde(rename = "FRC_ReadGIN")]
    FrcReadGIN(FrcReadGINResponse),

    #[serde(rename = "FRC_WriteGOUT")]
    FrcWriteGOUT(FrcWriteGOUTResponse),

    #[serde(rename = "FRC_ReadCartesianPosition")]
    FrcReadCartesianPosition(FrcReadCartesianPositionResponse),

    #[serde(rename = "FRC_ReadJointAngles")]
    FrcReadJointAngles(FrcReadJointAnglesResponse),

    #[serde(rename = "FRC_SetOverRide")]
    FrcSetOverRide(FrcSetOverRideResponse),

    #[serde(rename = "FRC_ReadPositionRegister")]
    FrcReadPositionRegister(FrcReadPositionRegisterResponse),

    #[serde(rename = "FRC_WritePositionRegister")]
    FrcWritePositionRegister(FrcWritePositionRegisterResponse),

    #[serde(rename = "FRC_Reset")]
    FrcReset(FrcResetResponse),

    #[serde(rename = "FRC_ReadTCPSpeed")]
    FrcReadTCPSpeed(FrcReadTCPSpeedResponse),
}

impl Packet for Command {}

// ExtractInner trait implementations for CommandResponse
impl_extract_inner!(CommandResponse, FrcInitialize, FrcInitializeResponse);
impl_extract_inner!(CommandResponse, FrcAbort, FrcAbortResponse);
impl_extract_inner!(CommandResponse, FrcPause, FrcPauseResponse);
impl_extract_inner!(CommandResponse, FrcContinue, FrcContinueResponse);
impl_extract_inner!(CommandResponse, FrcReadError, FrcReadErrorResponse);
impl_extract_inner!(CommandResponse, FrcSetUFrameUTool, FrcSetUFrameUToolResponse);
impl_extract_inner!(CommandResponse, FrcGetUFrameUTool, FrcGetUFrameUToolResponse);
impl_extract_inner!(CommandResponse, FrcGetStatus, FrcGetStatusResponse);
impl_extract_inner!(CommandResponse, FrcReadUFrameData, FrcReadUFrameDataResponse);
impl_extract_inner!(CommandResponse, FrcWriteUFrameData, FrcWriteUFrameDataResponse);
impl_extract_inner!(CommandResponse, FrcReadUToolData, FrcReadUToolDataResponse);
impl_extract_inner!(CommandResponse, FrcWriteUToolData, FrcWriteUToolData);
impl_extract_inner!(CommandResponse, FrcReadDIN, FrcReadDINResponse);
impl_extract_inner!(CommandResponse, FrcWriteDOUT, FrcWriteDOUTResponse);
impl_extract_inner!(CommandResponse, FrcReadAIN, FrcReadAINResponse);
impl_extract_inner!(CommandResponse, FrcWriteAOUT, FrcWriteAOUTResponse);
impl_extract_inner!(CommandResponse, FrcReadGIN, FrcReadGINResponse);
impl_extract_inner!(CommandResponse, FrcWriteGOUT, FrcWriteGOUTResponse);
impl_extract_inner!(CommandResponse, FrcReadCartesianPosition, FrcReadCartesianPositionResponse);
impl_extract_inner!(CommandResponse, FrcReadJointAngles, FrcReadJointAnglesResponse);
impl_extract_inner!(CommandResponse, FrcSetOverRide, FrcSetOverRideResponse);
impl_extract_inner!(CommandResponse, FrcReadPositionRegister, FrcReadPositionRegisterResponse);
impl_extract_inner!(CommandResponse, FrcWritePositionRegister, FrcWritePositionRegisterResponse);
impl_extract_inner!(CommandResponse, FrcReset, FrcResetResponse);
impl_extract_inner!(CommandResponse, FrcReadTCPSpeed, FrcReadTCPSpeedResponse);
