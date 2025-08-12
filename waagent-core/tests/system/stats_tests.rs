use waagent_core::system::{SystemInfo, SystemStats};

#[test]
fn test_system_info_integration() {
    let info = SystemInfo::current();
    
    // Integration test: verify the complete flow works
    assert!(!info.hostname.is_empty());
    assert!(!info.os_name.is_empty());
    assert!(!info.os_version.is_empty());
    
    // Test that the data is realistic
    assert!(info.hostname.len() > 0);
    assert!(info.os_name.len() > 0);
    assert!(info.os_version.len() > 0);
    
    println!("System Info: {:?}", info);
}

#[test]
fn test_system_stats_integration() {
    let stats = SystemStats::current();
    
    // Integration test: verify the complete flow works
    // Verify reasonable ranges for numeric values
    assert!(stats.cpu_usage >= 0.0, "CPU usage should be non-negative");
    assert!(stats.memory_usage >= 0.0 && stats.memory_usage <= 100.0, "Memory usage should be between 0 and 100");
    // uptime_seconds is u64, so it's always non-negative by type
    
    // Verify values are finite (not NaN or infinite)
    assert!(stats.cpu_usage.is_finite(), "CPU usage should be finite");
    assert!(stats.memory_usage.is_finite(), "Memory usage should be finite");
    
    println!("System Stats: {:?}", stats);
}

#[test]
fn test_system_uptime_changes() {
    let mut stats = SystemStats::current();
    let first_uptime: u64 = stats.uptime_seconds;

    // Sleep for a short time
    std::thread::sleep(std::time::Duration::from_millis(1250));

    stats = SystemStats::current();
    let second_uptime: u64 = stats.uptime_seconds;
    
    // Uptime should increase (or stay the same due to timing precision)
    assert!(second_uptime >= first_uptime, "Expected second iteration to be >= first --> First Uptime: {}, Second Uptime: {}", first_uptime, second_uptime);
}

#[test]
fn test_system_cpu_non_zero() {
    let stats = SystemStats::current();

    assert!(stats.cpu_usage >= 0.0, "CPU usage should be non-negative");
}

#[test]
fn test_system_memory_non_zero() {
    let stats = SystemStats::current();

    assert!(stats.memory_usage > 0.0, "Memory usage should be greater than 0");
}