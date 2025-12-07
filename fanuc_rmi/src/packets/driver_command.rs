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
}
