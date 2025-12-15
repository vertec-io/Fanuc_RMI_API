mod robot_status;
mod position_display;
mod jog_controls;
mod error_log;
mod motion_log;
mod settings;
mod toast;
mod robot_creation_wizard;
mod hmi_popup;
pub mod layout;
pub mod hmi_widgets;

pub use robot_status::RobotStatus;
pub use position_display::PositionDisplay;
pub use jog_controls::JogControls;
pub use error_log::ErrorLog;
pub use toast::ToastContainer;
pub use layout::{DesktopLayout, FloatingJogControls, FloatingIOStatus};
#[allow(unused_imports)]
pub use robot_creation_wizard::RobotCreationWizard;

// Re-export for potential future use (currently used in layout components)
#[allow(unused_imports)]
pub use motion_log::MotionLog;
#[allow(unused_imports)]
pub use settings::Settings;

// HMI Widget re-exports (will be used in Phase 5: HMI Panel View)
#[allow(unused_imports)]
pub use hmi_widgets::{LedIndicator, IoButton, Gauge, Slider, Bar, Numeric, MultiState};

// HMI Popup for pop-out windows
pub use hmi_popup::HmiPopup;

