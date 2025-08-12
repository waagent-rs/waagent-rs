use super::*;
use std::process::Command;
use tracing::{debug, warn};

pub struct UnixFirewallManager {
    use_sudo: bool,
}

impl UnixFirewallManager {
    pub fn new() -> Self {
        Self { use_sudo: true }
    }
    
    pub fn new_no_sudo() -> Self {
        Self { use_sudo: false }
    }
}

impl FirewallManager for UnixFirewallManager {
    fn add_rule(&self, rule: &FirewallRule) -> Result<(), Box<dyn Error>> {
        // Check if rule already exists before adding
        if self.rule_exists(rule)? {
            warn!("Rule already exists, skipping: {:?}", rule);
            return Ok(());
        }
        
        let args = self.build_iptables_args(rule, "INSERT")?;
        self.execute_command(args)
    }
    
    fn remove_rule(&self, rule: &FirewallRule) -> Result<(), Box<dyn Error>> {
        let args = self.build_iptables_args(rule, "DELETE")?;
        self.execute_command(args)
    }
    
    fn rule_exists(&self, rule: &FirewallRule) -> Result<bool, Box<dyn Error>> {
        let args = self.build_iptables_args(rule, "CHECK")?;
        match self.execute_command(args) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    fn list_rules(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.arg("iptables");
            c
        } else {
            Command::new("iptables")
        };
        
        let output = cmd
            .args(&["-t", "security", "-L", "OUTPUT", "-n", "--line-numbers"])
            .output()?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to list rules: {}", stderr).into());
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().map(|s| s.to_string()).collect())
    }
}

impl UnixFirewallManager {
    fn build_iptables_args(&self, rule: &FirewallRule, operation: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let mut args = vec![
            "iptables".to_string(),
            "-t".to_string(), "security".to_string(),
        ];
        
        match operation {
            // Use APPEND instead of INSERT to avoid position conflicts
            "INSERT" => args.extend(["-A".to_string(), "OUTPUT".to_string()]),
            "DELETE" => args.extend(["-D".to_string(), "OUTPUT".to_string()]),
            "CHECK" => args.extend(["-C".to_string(), "OUTPUT".to_string()]),
            _ => return Err("Invalid operation".into()),
        }
        
        // Add destination
        args.extend(["-d".to_string(), rule.destination.clone()]);
        
        // Add protocol
        match rule.protocol {
            Protocol::Tcp => args.extend(["-p".to_string(), "tcp".to_string()]),
            Protocol::Udp => args.extend(["-p".to_string(), "udp".to_string()]),
            Protocol::Any => {},
        }
        
        // Add port if specified
        if let Some(port) = rule.port {
            args.extend(["--dport".to_string(), port.to_string()]);
        }
        
        // Add uid owner if specified (Unix-specific)
        if let Some(uid) = &rule.uid_owner {
            args.extend([
                "-m".to_string(), "owner".to_string(),
                "--uid-owner".to_string(), uid.clone(),
            ]);
        }
        
        // Add action
        match rule.action {
            Action::Allow => args.extend(["-j".to_string(), "ACCEPT".to_string()]),
            Action::Block => args.extend(["-j".to_string(), "DROP".to_string()]),
        }
        
        Ok(args)
    }
    
    fn execute_command(&self, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
        debug!("Command: {}", args.join(" "));

        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.args(&args);
            c
        } else {
            let program = args.remove(0);
            let mut c = Command::new(program);
            c.args(&args);
            c
        };
        
        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Command failed: {}", stderr).into());
        }
        
        Ok(())
    }
}