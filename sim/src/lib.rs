// Library exports for the FANUC CRX simulator

pub mod robot_config;
pub mod kinematics;

pub use robot_config::{RobotConfig, RobotModel};
pub use kinematics::CRXKinematics;

