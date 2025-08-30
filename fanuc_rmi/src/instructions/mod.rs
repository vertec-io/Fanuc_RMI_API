mod frc_waitdin;
mod frc_setuframe;
mod frc_setutool;
mod frc_waittime;
mod frc_setpayload;
mod frc_call;
mod frc_linearmotion;
mod frc_linearrelative;
mod frc_linearrelativejrep;
mod frc_jointmotion;
mod frc_jointrelative;
mod frc_circularmotion;
mod frc_circularrelative;
mod frc_jointmotionjrep;
mod frc_jointrelativejrep;
mod frc_linearmotionjrep;

pub use frc_waitdin::*;
pub use frc_setuframe::*;
pub use frc_setutool::*;
pub use frc_waittime::*;
pub use frc_setpayload::*;
pub use frc_call::*;
pub use frc_linearmotion::*;
pub use frc_linearrelative::*;
pub use frc_linearrelativejrep::*;
pub use frc_jointmotion::*;
pub use frc_jointrelative::*;
pub use frc_circularmotion::*;
pub use frc_circularrelative::*;
pub use frc_jointmotionjrep::*;
pub use frc_jointrelativejrep::*;
pub use frc_linearmotionjrep::*;


#[cfg(feature = "DTO")]
pub mod dto {
    pub use super::frc_waitdin::FrcWaitDINDto as FrcWaitDIN;
    pub use super::frc_setuframe::FrcSetUFrameDto as FrcSetUFrame;
    pub use super::frc_setutool::FrcSetUToolDto as FrcSetUTool;
    pub use super::frc_waittime::FrcWaitTimeDto as FrcWaitTime;
    pub use super::frc_setpayload::FrcSetPayLoadDto as FrcSetPayLoad;
    pub use super::frc_call::FrcCallDto as FrcCall;
    pub use super::frc_linearmotion::FrcLinearMotionDto as FrcLinearMotion;
    pub use super::frc_linearrelative::FrcLinearRelativeDto as FrcLinearRelative;
    pub use super::frc_linearrelativejrep::FrcLinearRelativeJRepDto as FrcLinearRelativeJRep;
    pub use super::frc_jointmotion::FrcJointMotionDto as FrcJointMotion;
    pub use super::frc_jointrelative::FrcJointRelativeDto as FrcJointRelative;
    pub use super::frc_circularmotion::FrcCircularMotionDto as FrcCircularMotion;
    pub use super::frc_circularrelative::FrcCircularRelativeDto as FrcCircularRelative;
    pub use super::frc_jointmotionjrep::FrcJointMotionJRepDto as FrcJointMotionJRep;
    pub use super::frc_jointrelativejrep::FrcJointRelativeJRepDto as FrcJointRelativeJRep;
    pub use super::frc_linearmotionjrep::FrcLinearMotionJRepDto as FrcLinearMotionJRep;
        pub use super::frc_waitdin::FrcWaitDINResponseDto as FrcWaitDINResponse;
        pub use super::frc_setuframe::FrcSetUFrameResponseDto as FrcSetUFrameResponse;
        pub use super::frc_setutool::FrcSetUToolResponseDto as FrcSetUToolResponse;
        pub use super::frc_waittime::FrcWaitTimeResponseDto as FrcWaitTimeResponse;
        pub use super::frc_setpayload::FrcSetPayLoadResponseDto as FrcSetPayLoadResponse;
        pub use super::frc_call::FrcCallResponseDto as FrcCallResponse;
        pub use super::frc_linearmotion::FrcLinearMotionResponseDto as FrcLinearMotionResponse;
        pub use super::frc_linearrelative::FrcLinearRelativeResponseDto as FrcLinearRelativeResponse;
        pub use super::frc_linearrelativejrep::FrcLinearRelativeJRepResponseDto as FrcLinearRelativeJRepResponse;
        pub use super::frc_jointmotion::FrcJointMotionResponseDto as FrcJointMotionResponse;
        pub use super::frc_jointrelative::FrcJointRelativeResponseDto as FrcJointRelativeResponse;
        pub use super::frc_circularmotion::FrcCircularMotionResponseDto as FrcCircularMotionResponse;
        pub use super::frc_circularrelative::FrcCircularRelativeResponseDto as FrcCircularRelativeResponse;
        pub use super::frc_jointmotionjrep::FrcJointMotionJRepResponseDto as FrcJointMotionJRepResponse;
        pub use super::frc_jointrelativejrep::FrcJointRelativeJRepResponseDto as FrcJointRelativeJRepResponse;
        pub use super::frc_linearmotionjrep::FrcLinearMotionJRepResponseDto as FrcLinearMotionJRepResponse;

}
