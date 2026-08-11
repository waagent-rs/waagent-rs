// Azure Agent Configuration Constants

/// Agent version string
pub const AGENT_VERSION: &str = "waagent-rs/0.0.1";

/// Agent name
pub const AGENT_NAME: &str = "waagent-rs";

/// API version for legacy WireServer protocol (goal state, certificates)
pub const WIRESERVER_API_VERSION: &str = "2012-11-30";

/// API version for modern HostGAPlugin API (vmSettings, status)
pub const HOSTGAPLUGIN_API_VERSION: &str = "2015-09-01";

/// Interval between heartbeats in seconds
pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Check extensions every N heartbeats (to reduce overhead)
pub const EXTENSIONS_CHECK_INTERVAL: u32 = 2;
