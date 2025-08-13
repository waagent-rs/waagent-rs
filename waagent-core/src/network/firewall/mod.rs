// src/firewall.rs
use std::error::Error;

// Platform-specific modules
pub mod windows;
pub mod unix;

// Platform-specific exports
pub use windows::WindowsFirewallManager;
pub use unix::UnixFirewallManager;

#[derive(Debug, Clone)]
pub struct FirewallRule {
    pub name: String, // Unique identifier for the rule
    pub direction: Direction,
    pub action: Action,
    pub protocol: Protocol,
    pub destination: String,
    pub port: Option<u16>,
    pub uid_owner: Option<String>, // Unix-specific
    pub program_path: Option<String>, // Windows-specific
}

#[derive(Debug, Clone)]
pub enum Direction {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone)]
pub enum Action {
    Allow,
    Block,
}

#[derive(Debug, Clone)]
pub enum Protocol {
    Tcp,
    Udp,
    Any,
}

pub trait FirewallManager {
    fn add_rule(&self, rule: &FirewallRule) -> Result<(), Box<dyn Error>>;
    fn remove_rule(&self, rule: &FirewallRule) -> Result<(), Box<dyn Error>>;
    fn rule_exists(&self, rule: &FirewallRule) -> Result<bool, Box<dyn Error>>;
    fn list_rules(&self) -> Result<Vec<String>, Box<dyn Error>>;
}

// Factory function
pub fn create_firewall_manager() -> Box<dyn FirewallManager> {
    #[cfg(windows)]
    return Box::new(WindowsFirewallManager::new());
    
    #[cfg(unix)]
    return Box::new(unix::UnixFirewallManager::new());
}