//! Shared API types for FANUC RMI web application.
//!
//! This crate provides the types used for WebSocket communication between
//! the web server and web client. All types are WASM-compatible.
//!
//! # Architecture
//!
//! - `web_common` - Web API types (ClientRequest, ServerResponse, DTOs)
//! - `fanuc_rmi` - Robot protocol types (Position, Configuration, Command, etc.)
//!
//! Both `web_server` and `web_app` depend on both crates directly.
//!
//! # Reused Types from fanuc_rmi
//!
//! This crate re-exports the following types from `fanuc_rmi::dto` to avoid duplication:
//! - `FrameData` - 6-DOF coordinate data (x, y, z, w, p, r)
//! - `Configuration` - Robot arm configuration (frame/tool numbers, arm config bits)
//! - `Position` - Full Cartesian position with orientation and external axes
//!
//! # Usage
//!
//! ```rust
//! use web_common::{ClientRequest, ServerResponse, ProgramInfo, FrameData, Configuration};
//! ```

mod requests;
mod responses;
mod programs;
mod robots;
mod settings;
mod models;

pub use requests::*;
pub use responses::*;
pub use programs::*;
pub use robots::*;
pub use settings::*;
pub use models::*;

// Re-export fanuc_rmi DTO types that are used in the API
pub use fanuc_rmi::dto::{FrameData, Configuration, Position};

