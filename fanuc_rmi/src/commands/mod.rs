mod frc_initialize;
mod frc_readerror;
mod frc_abort;
mod frc_pause;
mod frc_continue;
mod frc_setuframeutool;
mod frc_getstatus;
mod frc_readuframedata;
mod frc_writeuframedata;
mod frc_readutooldata;
mod frc_writeutooldata;
mod frc_readdin;
mod frc_writedout;
mod frc_readcartesianposition;
mod frc_readjointangles;
mod frc_setoveride;
mod frc_getuframeutool;
mod frc_readpositionregister;
mod frc_writepositionregister;
mod frc_reset;
mod frc_readtcpspeed;

pub use frc_initialize::*;
pub use frc_readerror::*;
pub use frc_abort::*;
pub use frc_pause::*;
pub use frc_continue::*;
pub use frc_setuframeutool::*;
pub use frc_getstatus::*;
pub use frc_readuframedata::*;
pub use frc_writeuframedata::*;
pub use frc_readutooldata::*;
pub use frc_writeutooldata::*;
pub use frc_readdin::*;
pub use frc_writedout::*;
pub use frc_readcartesianposition::*;
pub use frc_readjointangles::*;
pub use frc_setoveride::*;
pub use frc_getuframeutool::*;
pub use frc_readpositionregister::*;
pub use frc_writepositionregister::*;
pub use frc_reset::*;
pub use frc_readtcpspeed::*;

#[cfg(feature = "DTO")]
pub mod dto {
    pub use super::frc_initialize::FrcInitializeDto as FrcInitialize;
    pub use super::frc_readerror::FrcReadErrorDto as FrcReadError;
    pub use super::frc_abort::FrcAbortResponseDto as FrcAbortResponse;
    pub use super::frc_pause::FrcPauseResponseDto as FrcPauseResponse;
    pub use super::frc_continue::FrcContinueResponseDto as FrcContinueResponse;
    pub use super::frc_setuframeutool::FrcSetUFrameUToolDto as FrcSetUFrameUTool;
    pub use super::frc_getstatus::FrcGetStatusResponseDto as FrcGetStatusResponse;
    pub use super::frc_readuframedata::FrcReadUFrameDataDto as FrcReadUFrameData;
    pub use super::frc_writeuframedata::FrcWriteUFrameDataDto as FrcWriteUFrameData;
    pub use super::frc_readutooldata::FrcReadUToolDataDto as FrcReadUToolData;
    pub use super::frc_readutooldata::FrcReadUToolDataResponseDto as FrcReadUToolDataResponse;
    pub use super::frc_writeutooldata::FrcWriteUToolDataDto as FrcWriteUToolData;
    pub use super::frc_readdin::FrcReadDINDto as FrcReadDIN;
    pub use super::frc_writedout::FrcWriteDOUTDto as FrcWriteDOUT;
    pub use super::frc_readcartesianposition::FrcReadCartesianPositionDto as FrcReadCartesianPosition;
    pub use super::frc_readcartesianposition::FrcReadCartesianPositionResponseDto as FrcReadCartesianPositionResponse;
    pub use super::frc_readjointangles::FrcReadJointAnglesDto as FrcReadJointAngles;
    pub use super::frc_setoveride::FrcSetOverRideDto as FrcSetOverRide;
    pub use super::frc_getuframeutool::FrcGetUFrameUToolDto as FrcGetUFrameUTool;
    pub use super::frc_readpositionregister::FrcReadPositionRegisterDto as FrcReadPositionRegister;
    pub use super::frc_writepositionregister::FrcWritePositionRegisterDto as FrcWritePositionRegister;
    pub use super::frc_reset::FrcResetResponseDto as FrcResetResponse;
    pub use super::frc_readtcpspeed::FrcReadTCPSpeedResponseDto as FrcReadTCPSpeedResponse;
        pub use super::frc_initialize::FrcInitializeResponseDto as FrcInitializeResponse;
        pub use super::frc_readerror::FrcReadErrorResponseDto as FrcReadErrorResponse;
        pub use super::frc_setuframeutool::FrcSetUFrameUToolResponseDto as FrcSetUFrameUToolResponse;
        pub use super::frc_getuframeutool::FrcGetUFrameUToolResponseDto as FrcGetUFrameUToolResponse;
        pub use super::frc_readuframedata::FrcReadUFrameDataResponseDto as FrcReadUFrameDataResponse;
        pub use super::frc_writeuframedata::FrcWriteUFrameDataResponseDto as FrcWriteUFrameDataResponse;
        pub use super::frc_readdin::FrcReadDINResponseDto as FrcReadDINResponse;
        pub use super::frc_writedout::FrcWriteDOUTResponseDto as FrcWriteDOUTResponse;
        pub use super::frc_readjointangles::FrcReadJointAnglesResponseDto as FrcReadJointAnglesResponse;
        pub use super::frc_setoveride::FrcSetOverRideResponseDto as FrcSetOverRideResponse;
        pub use super::frc_readpositionregister::FrcReadPositionRegisterResponseDto as FrcReadPositionRegisterResponse;
        pub use super::frc_writepositionregister::FrcWritePositionRegisterResponseDto as FrcWritePositionRegisterResponse;

}



