use std::error::Error;
use std::fmt;
use int_enum::IntEnum;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FrcError{
    Serialization(String),
    UnrecognizedPacket,
    FanucErrorCode(FanucErrorCode),
    FailedToSend(String),
    FailedToRecieve(String),
    Disconnected(),
    Initialization(String),
}
impl Error for FrcError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
impl fmt::Display for FrcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FrcError::Serialization(ref msg) => write!(f, "Serialization error: {}", msg),
            FrcError::UnrecognizedPacket => write!(f, "Fanuc threw an unrecognized weeoe"),
            FrcError::FanucErrorCode(ref code) => write!(f, "fanuc returned  error#: {}", code.message()),
            FrcError::FailedToSend(ref msg) => write!(f, "SendError: {}", msg),
            FrcError::FailedToRecieve(ref msg) => write!(f, "RecieveError: {}", msg),
            FrcError::Disconnected() => write!(f, "Fanuc appears to be disconnected"),
            FrcError::Initialization(ref msg) => write!(f, "Could not initialize: {}", msg)
        }
    }
}

#[repr(u32)]
#[derive(Debug, Serialize, Deserialize, IntEnum, Clone)]
pub enum FanucErrorCode {
    InternalSystemError = 2556929,
    InvalidUToolNumber = 2556930,
    InvalidUFrameNumber = 2556931,
    InvalidPositionRegister = 2556932,
    InvalidSpeedOverride = 2556933,
    CannotExecuteTPProgram = 2556934,
    ControllerServoOff = 2556935,
    CannotExecuteTPProgramDuplicate = 2556936,
    RMINotRunning = 2556937,
    TPProgramNotPaused = 2556938,
    CannotResumeTPProgram = 2556939,
    CannotResetController = 2556940,
    InvalidRMICommand = 2556941,
    RMICommandFail = 2556942,
    InvalidControllerState = 2556943,
    PleaseCyclePower = 2556944,
    InvalidPayloadSchedule = 2556945,
    InvalidMotionOption = 2556946,
    InvalidVisionRegister = 2556947,
    InvalidRMIInstruction = 2556948,
    InvalidValue = 2556949,
    InvalidTextString = 2556950,
    InvalidPositionData = 2556951,
    RMIInHoldState = 2556952,
    RemoteDeviceDisconnected = 2556953,
    RobotAlreadyConnected = 2556954,
    WaitForCommandDone = 2556955,
    WaitForInstructionDone = 2556956,
    InvalidSequenceIDNumber = 2556957,
    InvalidSpeedType = 2556958,
    InvalidSpeedValue = 2556959,
    InvalidTermType = 2556960,
    InvalidTermValue = 2556961,
    InvalidLCBPortType = 2556962,
    InvalidACCValue = 2556963,
    InvalidDestinationPosition = 2556964,
    InvalidVIAPosition = 2556965,
    InvalidPortNumber = 2556966,
    InvalidGroupNumber = 2556967,
    InvalidGroupMask = 2556968,
    JointMotionWithCOORD = 2556969,
    IncrementalMotionWithCOORD = 2556970,
    RobotInSingleStepMode = 2556971,
    InvalidPositionDataType = 2556972,
    ReadyForASCIIPacket = 2556973,
    ASCIIConversionFailed = 2556974,
    InvalidASCIIInstruction = 2556975,
    InvalidNumberOfGroups = 2556976,
    InvalidInstructionPacket = 2556977,
    InvalidASCIIStringPacket = 2556978,
    InvalidASCIIStringSize = 2556979,
    InvalidApplicationTool = 2556980,
    InvalidCallProgramName = 2556981,
    UnrecognizedFrcError = 0,
}

impl FanucErrorCode {
    fn message(&self) -> &str {
        match self {
            FanucErrorCode::InternalSystemError => "Internal System Error.",
            FanucErrorCode::InvalidUToolNumber => "Invalid UTool Number.",
            FanucErrorCode::InvalidUFrameNumber => "Invalid UFrame Number.",
            FanucErrorCode::InvalidPositionRegister => "Invalid Position Register.",
            FanucErrorCode::InvalidSpeedOverride => "Invalid Speed Override.",
            FanucErrorCode::CannotExecuteTPProgram => "Cannot Execute TP program.",
            FanucErrorCode::ControllerServoOff => "Controller Servo is Off.",
            FanucErrorCode::CannotExecuteTPProgramDuplicate => "Cannot Execute TP program.",
            FanucErrorCode::RMINotRunning => "RMI is Not Running.",
            FanucErrorCode::TPProgramNotPaused => "TP Program is Not Paused.",
            FanucErrorCode::CannotResumeTPProgram => "Cannot Resume TP Program.",
            FanucErrorCode::CannotResetController => "Cannot Reset Controller.",
            FanucErrorCode::InvalidRMICommand => "Invalid RMI Command.",
            FanucErrorCode::RMICommandFail => "RMI Command Fail.",
            FanucErrorCode::InvalidControllerState => "Invalid Controller State.",
            FanucErrorCode::PleaseCyclePower => "Please Cycle Power.",
            FanucErrorCode::InvalidPayloadSchedule => "Invalid Payload Schedule.",
            FanucErrorCode::InvalidMotionOption => "Invalid Motion Option.",
            FanucErrorCode::InvalidVisionRegister => "Invalid Vision Register.",
            FanucErrorCode::InvalidRMIInstruction => "Invalid RMI Instruction.",
            FanucErrorCode::InvalidValue => "Invalid Value.",
            FanucErrorCode::InvalidTextString => "Invalid Text String.",
            FanucErrorCode::InvalidPositionData => "Invalid Position Data.",
            FanucErrorCode::RMIInHoldState => "RMI is In HOLD State.",
            FanucErrorCode::RemoteDeviceDisconnected => "Remote Device Disconnected.",
            FanucErrorCode::RobotAlreadyConnected => "Robot is Already Connected.",
            FanucErrorCode::WaitForCommandDone => "Wait for Command Done.",
            FanucErrorCode::WaitForInstructionDone => "Wait for Instruction Done.",
            FanucErrorCode::InvalidSequenceIDNumber => "Invalid sequence ID number.",
            FanucErrorCode::InvalidSpeedType => "Invalid Speed Type.",
            FanucErrorCode::InvalidSpeedValue => "Invalid Speed Value.",
            FanucErrorCode::InvalidTermType => "Invalid Term Type.",
            FanucErrorCode::InvalidTermValue => "Invalid Term Value.",
            FanucErrorCode::InvalidLCBPortType => "Invalid LCB Port Type.",
            FanucErrorCode::InvalidACCValue => "Invalid ACC Value.",
            FanucErrorCode::InvalidDestinationPosition => "Invalid Destination Position.",
            FanucErrorCode::InvalidVIAPosition => "Invalid VIA Position.",
            FanucErrorCode::InvalidPortNumber => "Invalid Port Number.",
            FanucErrorCode::InvalidGroupNumber => "Invalid Group Number.",
            FanucErrorCode::InvalidGroupMask => "Invalid Group Mask.",
            FanucErrorCode::JointMotionWithCOORD => "Joint motion with COORD.",
            FanucErrorCode::IncrementalMotionWithCOORD => "Incremental motn with COORD.",
            FanucErrorCode::RobotInSingleStepMode => "Robot in Single Step Mode.",
            FanucErrorCode::InvalidPositionDataType => "Invalid Position Data Type.",
            FanucErrorCode::ReadyForASCIIPacket => "Ready for ASCII Packet.",
            FanucErrorCode::ASCIIConversionFailed => "ASCII Conversion Failed.",
            FanucErrorCode::InvalidASCIIInstruction => "Invalid ASCII Instruction.",
            FanucErrorCode::InvalidNumberOfGroups => "Invalid Number of Groups.",
            FanucErrorCode::InvalidInstructionPacket => "Invalid Instruction packet.",
            FanucErrorCode::InvalidASCIIStringPacket => "Invalid ASCII String packet.",
            FanucErrorCode::InvalidASCIIStringSize => "Invalid ASCII string size.",
            FanucErrorCode::InvalidApplicationTool => "Invalid Application Tool.",
            FanucErrorCode::InvalidCallProgramName => "Invalid Call Program Name.",
            FanucErrorCode::UnrecognizedFrcError => "Unrecognized FANUC Error ID",
        }
    }
}