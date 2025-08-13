use waagent_core::system::{SystemInfo, SystemStats};

pub mod info_tests;
pub mod stats_tests;

#[test] 
fn test_multiple_calls_performance() {
    use std::time::Instant;
    
    let start = Instant::now();
    
    // Test that multiple calls complete in reasonable time
    for _ in 0..10 {
        let _info = SystemInfo::current();
        let _stats = SystemStats::current();
    }
    
    let duration = start.elapsed();
    
    // Should complete 20 calls in under 1 second (very generous)
    assert!(duration.as_secs() < 1, "Multiple system calls should be reasonably fast");
    
    println!("10 iterations took: {:?}", duration);
}