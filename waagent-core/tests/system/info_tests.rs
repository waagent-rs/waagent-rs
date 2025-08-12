use waagent_core::system::SystemInfo;

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
fn test_data_consistency_over_time() {
    let info1 = SystemInfo::current();
    
    // Sleep for a short time
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    let info2 = SystemInfo::current();
    
    // Static system information should remain consistent
    assert_eq!(info1.hostname, info2.hostname, "Hostname should not change");
    assert_eq!(info1.os_name, info2.os_name, "OS name should not change");
    assert_eq!(info1.os_version, info2.os_version, "OS version should not change");
}
