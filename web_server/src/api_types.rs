//! WebSocket API types for program management and robot control.
//!
//! These types are used for client-server communication over WebSocket.
//! They are separate from the fanuc_rmi DTO types which handle robot protocol.
//!
//! All types are re-exported from the `web_common` crate which is shared
//! between `web_server` and `web_app`.

// Re-export all types from web_common
pub use web_common::*;
