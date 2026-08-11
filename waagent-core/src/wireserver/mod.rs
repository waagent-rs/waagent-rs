//! Azure WireServer protocol types and structures
//!
//! This module contains all the data structures used to communicate with
//! the Azure WireServer and HostGAPlugin services.

pub mod goal_state;
pub mod health;
pub mod telemetry;
pub mod protocol;
pub mod telemetry_builder;

// Re-export commonly used types
pub use goal_state::*;
pub use health::*;
pub use telemetry::*;
pub use protocol::*;
pub use telemetry_builder::*;
