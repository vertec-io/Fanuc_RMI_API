use serde::{Serialize, Deserialize};
use super::Packet;
use crate::commands::*;

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
    FrcSetOverride(FrcSetOverride),

    #[serde(rename = "FRC_GetStatus")]
    FrcGetStatus,

    #[serde(rename = "FRC_GetUFrameUTool")]
    FrcGetUFrameUTool(FrcGetUFrameUTool),

    #[serde(rename = "FRC_WriteUToolData")]
    FrcWriteUToolData(FrcWriteUToolData),

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

    #[serde(rename = "FRC_ReadCartesianPosition")]
    FrcReadCartesianPosition(FrcReadCartesianPosition),

    #[serde(rename = "FRC_ReadJointAngles")]
    FrcReadJointAngles(FrcReadJointAngles),

    #[serde(rename = "FRC_ReadTCPSpeed")]
    FrcReadTCPSpeed,
}







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

    #[serde(rename = "FRC_ReadCartesianPosition")]
    FrcReadCartesianPosition(FrcReadCartesianPositionResponse),

    #[serde(rename = "FRC_WriteDOUT")]
    FrcWriteDOUT(FrcWriteDOUTResponse),

    #[serde(rename = "FRC_ReadJointAngles")]
    FrcReadJointAngles(FrcReadJointAnglesResponse),
    
    #[serde(rename = "FRC_SetOverRide")]
    FrcSetOverride(FrcSetOverrideResponse),

    #[serde(rename = "FRC_ReadPositionRegister")]
    FrcReadPositionRegister(FrcReadPositionRegisterResponse),

    #[serde(rename = "FRC_WritePositionRegister")]
    FrcWritePositionRegister(FrcWritePositionRegisterResponse),

    #[serde(rename = "FRC_Reset")]
    FrcReset(FrcResetResponse),

    #[serde(rename = "FRC_ReadTCPSpeed")]
    FrcReadTCPSpeed(FrcReadTCPSpeedResponse),


}


impl Packet for Command{}
