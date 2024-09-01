mod driver;
#[cfg(feature="driver")]
pub use driver::*;

#[cfg(not(feature="driver"))]
pub use driver::{FanucDriverConfig,FanucErrorCode};