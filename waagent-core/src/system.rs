use sysinfo::System;

#[derive(Debug)]
pub struct SystemInfo {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
}

#[derive(Debug)]
pub struct SystemStats {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub uptime_seconds: u64,
}

impl SystemInfo {
    pub fn current() -> Self {
        let hostname = get_hostname();
        let os_version = get_os_version();
        let os_name = get_os_display_name();

        SystemInfo {
            hostname,
            os_name,
            os_version,
        }
    }
}

impl SystemStats {
    pub fn current() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let cpu_usage = get_cpu_usage_percent();
        let memory_usage = get_memory_usage_percent();
        let uptime_seconds = get_uptime_seconds();

        SystemStats {
            cpu_usage,
            memory_usage,
            uptime_seconds,
        }
    }

    pub fn uptime_seconds_str(&self) -> String {
        format!("{}", self.uptime_seconds)
    }

    pub fn cpu_usage_str(&self) -> String {
        format!("{:.1}%", self.cpu_usage)
    }

    pub fn memory_usage_str(&self) -> String {
        format!("{:.1}%", self.memory_usage)
    }
}

fn get_hostname() -> String {
    let hostname = System::host_name().unwrap_or_else(|| "unknown".to_string());
    hostname.to_string()
}

fn get_cpu_usage_percent() -> f64 {
    System::load_average().one
}

// Get memory usage percentage
fn get_memory_usage_percent() -> f64 {
    let mut system = System::new_all();
    system.refresh_all();
    let used = system.used_memory();
    let total = system.total_memory();
    let usage_percent = (used as f64 / total as f64) * 100.0;
    usage_percent
}

// Get OS display name
fn get_os_display_name() -> String {
    let mut system = System::new_all();
    system.refresh_all();

    // Get OS name using the static method
    let os_name = System::name().unwrap_or_else(|| "Linux".to_string()).to_lowercase();

    // Try to match known distributions
    if os_name.contains("ubuntu") || os_name.contains("debian") || os_name.contains("centos") || os_name.contains("fedora") {
        os_name
    } else if os_name.contains("azure linux") {
        "Azurelinux".to_lowercase().to_string()
    } else if os_name.contains("rhel") || os_name.to_lowercase().contains("red hat") {
        "RHEL".to_lowercase().to_string()
    } else if os_name.contains("opensuse") || os_name.contains("suse") {
        "sles".to_string()
    } else {
        "Linux".to_lowercase().to_string()
    }
}

// Get OS version
fn get_os_version() -> String {
    // Try to get the actual OS version from sysinfo static method
    if let Some(os_version) = System::os_version() {
        os_version
    } else {
        // Fallback to Unknown
        format!("Unknown")
    }
}

// Get system uptime in seconds
fn get_uptime_seconds() -> u64 {
    #[cfg(all(not(unix), not(windows)))]
    {
        0
    }

    // Simple approximation using boot time
    #[cfg(unix)]    
    {
        let mut system = System::new_all();
        system.refresh_all();
        System::uptime()
    }

    #[cfg(windows)]
    {
        use winapi::um::sysinfoapi::GetTickCount64;
        
        unsafe {
            let uptime_ms = GetTickCount64();
            let uptime_seconds = uptime_ms / 1000;
            uptime_seconds
        }
    }
}

// Unit tests for the system module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_creation() {
        let info = SystemInfo::current();
        
        // Verify that all fields are populated (not empty)
        assert!(!info.hostname.is_empty(), "Hostname should not be empty");
        assert!(!info.os_name.is_empty(), "OS name should not be empty");
        assert!(!info.os_version.is_empty(), "OS version should not be empty");
        
        // Verify that fields contain reasonable values
        assert_ne!(info.hostname, "Undefined", "Hostname should not be 'Undefined' in normal circumstances");
    }

    #[test]
    fn test_system_stats_creation() {
        let stats = SystemStats::current();
        
        // Verify reasonable ranges for numeric values
        assert!(stats.cpu_usage >= 0.0, "CPU usage should be non-negative");
        assert!(stats.memory_usage >= 0.0 && stats.memory_usage <= 100.0, "Memory usage should be between 0 and 100");
        // uptime_seconds is u64, so it's always non-negative by type
        
        // Verify that we can format the values
        let cpu_str = stats.cpu_usage_str();
        let memory_str = stats.memory_usage_str();
        let uptime_str = stats.uptime_seconds_str();
        
        assert!(!cpu_str.is_empty(), "CPU usage string should not be empty");
        assert!(!memory_str.is_empty(), "Memory usage string should not be empty");
        assert!(!uptime_str.is_empty(), "Uptime string should not be empty");
    }

    #[test]
    fn test_get_hostname() {
        let hostname = get_hostname();
        
        // Should return a non-empty string
        assert!(!hostname.is_empty(), "Hostname should not be empty");
        
        // In case of error, it should return "Undefined"
        // This is hard to test directly since we can't force sys_info::hostname() to fail
        // but we verify the fallback logic exists
    }

    #[test]
    fn test_get_cpu_usage_percent() {
        let cpu_usage = get_cpu_usage_percent();
        
        // Should return a valid float
        assert!(cpu_usage >= 0.0, "CPU usage should be non-negative");
        
        // Test that it's a reasonable value (not NaN or infinite)
        assert!(cpu_usage.is_finite(), "CPU usage should be a finite number");
    }

    #[test]
    fn test_get_memory_usage_percent() {
        let memory_usage = get_memory_usage_percent();
        
        // Should return a valid float
        assert!(memory_usage >= 0.0 && memory_usage <= 100.0, "Memory usage should be between 0 and 100");
        
        // Test that it's a reasonable value (not NaN or infinite)
        assert!(memory_usage.is_finite(), "Memory usage should be a finite number");
    }

    #[test]
    fn test_get_os_version() {
        let version = get_os_version();
        
        // Should return a non-empty string
        assert!(!version.is_empty(), "OS version should not be empty");
        
        // Should not contain "Unknown" (os_info should provide some version info)
        assert_ne!(version.to_lowercase(), "unknown", "OS version should not be 'unknown'");
    }

    #[test]
    fn test_get_os_display_name() {
        let os_name = get_os_display_name();
        
        // Should return a non-empty string
        assert!(!os_name.is_empty(), "OS display name should not be empty");
        
        // Should be lowercase (per the implementation)
        assert_eq!(os_name, os_name.to_lowercase(), "OS display name should be lowercase");
        
        // Should not contain "Unknown" (os_info should provide some OS info)
        assert_ne!(os_name, "unknown", "OS display name should not be 'unknown'");
    }

    #[test]
    fn test_get_uptime_seconds() {
        let uptime = get_uptime_seconds();
        
        // Should return a valid u64 (non-negative by type)
        // In most cases, uptime should be greater than 0 (system has been running for some time)
        // But we don't assert this as it could be 0 in edge cases or during testing
        
        // Test that the function runs without panicking and returns a reasonable value
        // u64 is always non-negative by type, so we just verify it executed successfully
        let _ = uptime;
    }

    #[test]
    fn test_system_info_debug() {
        let info = SystemInfo::current();
        let debug_str = format!("{:?}", info);
        
        // Verify Debug trait works and includes expected fields
        assert!(debug_str.contains("SystemInfo"));
        assert!(debug_str.contains("hostname"));
        assert!(debug_str.contains("os_name"));
        assert!(debug_str.contains("os_version"));
    }

    #[test]
    fn test_system_stats_debug() {
        let stats = SystemStats::current();
        let debug_str = format!("{:?}", stats);
        
        // Verify Debug trait works and includes expected fields
        assert!(debug_str.contains("SystemStats"));
        assert!(debug_str.contains("cpu_usage"));
        assert!(debug_str.contains("memory_usage"));
        assert!(debug_str.contains("uptime_seconds"));
    }

    // Test for consistency - multiple calls should return similar results
    #[test]
    fn test_system_info_consistency() {
        let info1 = SystemInfo::current();
        let info2 = SystemInfo::current();
        
        // Hostname and OS info should be consistent across calls
        assert_eq!(info1.hostname, info2.hostname, "Hostname should be consistent");
        assert_eq!(info1.os_name, info2.os_name, "OS name should be consistent");
        assert_eq!(info1.os_version, info2.os_version, "OS version should be consistent");
    }

    #[test]
    fn test_system_stats_format() {
        let stats = SystemStats::current();
        
        // Verify that the formatted strings are as expected
        let cpu_str = stats.cpu_usage_str();
        let memory_str = stats.memory_usage_str();
        let uptime_str = stats.uptime_seconds_str();
        
        // Verify that formatted strings contain expected patterns
        assert!(cpu_str.contains('%'), "CPU usage string should contain '%'");
        assert!(memory_str.contains('%'), "Memory usage string should contain '%'");
        assert!(uptime_str.chars().all(|c| c.is_ascii_digit()), 
               "Uptime string should only contain digits");
        
        // Verify that the numeric values are reasonable
        assert!(stats.cpu_usage >= 0.0, "CPU usage should be non-negative");
        assert!(stats.memory_usage >= 0.0 && stats.memory_usage <= 100.0, 
               "Memory usage should be between 0 and 100");
    }
}
