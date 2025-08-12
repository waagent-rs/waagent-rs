// src/firewall/windows.rs
use super::*;
use std::process::Command;

pub struct WindowsFirewallManager;

impl WindowsFirewallManager {
    const RULE_PREFIX: &'static str = "MicrosoftAzure_";
    
    pub fn new() -> Self {
        Self
    }
}

impl FirewallManager for WindowsFirewallManager {
    fn add_rule(&self, rule: &FirewallRule) -> Result<(), Box<dyn Error>> {
        let args = self.build_netsh_args(rule, "add")?;
        self.execute_netsh_command(args)
    }
    
    fn remove_rule(&self, rule: &FirewallRule) -> Result<(), Box<dyn Error>> {
        let filter = self.build_rule_filter(rule)?;
        let args = vec![
            "advfirewall".to_string(),
            "firewall".to_string(),
            "delete".to_string(),
            "rule".to_string(),
            filter,
        ];
        self.execute_netsh_command(args)
    }
    
    fn rule_exists(&self, rule: &FirewallRule) -> Result<bool, Box<dyn Error>> {
        let filter = self.build_rule_filter(rule)?;
        let args = vec![
            "advfirewall".to_string(),
            "firewall".to_string(),
            "show".to_string(),
            "rule".to_string(),
            filter,
        ];
        
        match self.execute_netsh_command(args) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    fn list_rules(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let output = Command::new("netsh")
            .args(&["advfirewall", "firewall", "show", "rule", "name=all"])
            .output()?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to list rules: {}", stderr).into());
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().map(|s| s.to_string()).collect())
    }
}

impl WindowsFirewallManager {
    fn build_netsh_args(&self, rule: &FirewallRule, operation: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let mut args = vec![
            "advfirewall".to_string(),
            "firewall".to_string(),
            operation.to_string(),
            "rule".to_string(),
            format!("name={}{}", Self::RULE_PREFIX, rule.name),
        ];
        
        // Direction
        let dir = match rule.direction {
            Direction::Inbound => "in",
            Direction::Outbound => "out",
        };
        args.push(format!("dir={}", dir));
        
        // Action
        let action = match rule.action {
            Action::Allow => "allow",
            Action::Block => "block",
        };
        args.push(format!("action={}", action));
        
        // Protocol
        let protocol = match rule.protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp", 
            Protocol::Any => "any",
        };
        args.push(format!("protocol={}", protocol));
        
        // Remote IP
        args.push(format!("remoteip={}", rule.destination));
        
        // Port if specified
        if let Some(port) = rule.port {
            args.push(format!("remoteport={}", port));
        }
        
        // Program path if specified (Windows-specific)
        if let Some(path) = &rule.program_path {
            args.push(format!("program={}", path));
        }
        
        Ok(args)
    }
    
    fn build_rule_filter(&self, rule: &FirewallRule) -> Result<String, Box<dyn Error>> {
        Ok(format!("name={}{}", Self::RULE_PREFIX, rule.name))
    }
    
    fn execute_netsh_command(&self, args: Vec<String>) -> Result<(), Box<dyn Error>> {
        let output = Command::new("netsh")
            .args(&args)
            .output()?;
            
        if !output.status.success() {
            let mut stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

            // if stderr is empty, then output stdout
            if stderr.is_empty() {
                stderr = String::from_utf8_lossy(&output.stdout).trim().to_string();
            }

            return Err(format!("netsh command failed: {}", stderr).into());
        }
        
        Ok(())
    }
    
    // Bonus: Add a method to clean up all auto-created rules
    pub fn cleanup_auto_rules(&self) -> Result<(), Box<dyn Error>> {
        let args = vec![
            "advfirewall".to_string(),
            "firewall".to_string(),
            "delete".to_string(),
            "rule".to_string(),
            format!("name={}*", Self::RULE_PREFIX),
        ];
        self.execute_netsh_command(args)
    }
}