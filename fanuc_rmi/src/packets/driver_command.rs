use serde::{Serialize, Deserialize};



#[cfg_attr(feature = "DTO", crate::mirror_dto)]

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DriverCommand {
    Pause,
    Unpause,
}
