use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DriverCommand {
    Pause,
    Unpause,
}
