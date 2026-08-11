//! Azure agent lifecycle management
//! 
//! This module provides high-level functions for managing the Azure agent lifecycle:
//! - Agent initialization and startup
//! - Continuous heartbeat loop with goal state monitoring
//! - Extension processing and status reporting
//! - Telemetry event generation

mod heartbeat;
mod startup;

pub use heartbeat::run_heartbeat_loop;
pub use startup::initialize_agent;
