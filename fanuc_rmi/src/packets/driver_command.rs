use serde::{Serialize, Deserialize};



#[cfg_attr(feature = "DTO", crate::mirror_dto)]

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DriverCommand {
    Pause,
    Unpause,
    /// Clears the in-flight instruction counter.
    ///
    /// This should be sent after an abort command to reset the driver's tracking
    /// of in-flight packets, since the robot clears its motion queue on abort
    /// but doesn't send responses for aborted instructions.
    ClearInFlight,
    /// Program pause: Aborts the RMI program but preserves in-flight instructions for replay.
    /// Unlike Pause, this allows the robot to be jogged while the program is paused.
    ProgramPause,
    /// Program resume: Re-initializes the RMI program and replays preserved instructions.
    /// Sent after ProgramPause to resume normal operation.
    ProgramResume {
        /// Instructions to replay after re-initialization
        instructions_to_replay: Vec<crate::packets::Instruction>,
    },
}
