use reqwest::Client;
use tokio::time::{sleep, Duration};

use crate::config::{AGENT_NAME, AGENT_VERSION, HEARTBEAT_INTERVAL_SECS, EXTENSIONS_CHECK_INTERVAL};
use crate::crypto::download_and_save_certificates;
use crate::extensions::process_vmextensions_from_vmsettings;
use crate::wireserver::{GoalState, fetch_goal_state, fetch_vmsettings, send_status_report, send_telemetry_event, build_telemetry_event};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Run the continuous heartbeat loop
/// 
/// This function continuously:
/// - Refreshes the goal state
/// - Detects incarnation changes
/// - Periodically checks for and processes extensions
/// - Sends health and status reports
/// - Sends telemetry events
pub async fn run_heartbeat_loop(client: &Client, goal_state: &GoalState) -> Result<()> {
    let mut heartbeat_count = 1;
    let mut last_incarnation = goal_state.incarnation;
    let mut last_extensions_check = 0;
    
    // Persist extension statuses across heartbeats
    let mut all_extension_statuses: Vec<serde_json::Value> = Vec::new();
    
    loop {
        sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS)).await;

        // Re-fetch the goal state before each heartbeat/telemetry event
        let latest_goal_state = match fetch_goal_state(client).await {
            Ok(gs) => {
                if cfg!(debug_assertions) || heartbeat_count % 10 == 0 {
                    println!("Goal state incarnation: {}", gs.incarnation);
                }
                gs
            },
            Err(e) => {
                eprintln!("Failed to refresh goal state: {e}");
                // Use previous goal_state as fallback
                goal_state.clone()
            }
        };
        
        // Check for extensions on incarnation change OR periodically
        let incarnation_changed = latest_goal_state.incarnation != last_incarnation;
        let time_to_check_extensions = (heartbeat_count - last_extensions_check) >= EXTENSIONS_CHECK_INTERVAL;
        
        if incarnation_changed {
            println!("\n=== Goal State Changed: incarnation {} -> {} ===", 
                     last_incarnation, latest_goal_state.incarnation);
            last_incarnation = latest_goal_state.incarnation;
        }
        
        if incarnation_changed || time_to_check_extensions {
            if time_to_check_extensions && !incarnation_changed {
                println!("Periodic extensions check (heartbeat #{})", heartbeat_count);
            }
            last_extensions_check = heartbeat_count;
            
            // Clear old statuses when we're about to collect new ones
            all_extension_statuses.clear();
            
            // Download certificates before processing extensions
            if let Err(e) = download_and_save_certificates(client, &latest_goal_state, AGENT_NAME).await {
                eprintln!("Failed to download certificates: {}", e);
            }
            
            // Fetch extensions from modern vmSettings endpoint (HostGAPlugin)
            match fetch_vmsettings(client, &latest_goal_state, AGENT_NAME).await {
                Ok(vm_settings) => {
                    // Process extensions from vmSettings
                    match process_vmextensions_from_vmsettings(client, &vm_settings).await {
                        Ok(statuses) => {
                            all_extension_statuses = statuses;
                        }
                        Err(e) => {
                            eprintln!("Failed to process vmSettings extensions: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to fetch vmSettings: {}", e);
                }
            }
        }

        // Send status report with extension statuses included
        println!("[DEBUG] About to send status report with {} handler statuses", all_extension_statuses.len());
        if let Err(e) = send_status_report(client, &latest_goal_state, all_extension_statuses.clone(), AGENT_NAME, AGENT_VERSION).await {
            eprintln!("Failed to send status report: {e}");
        }

        // Cycle through different event types
        let (event_name, event_id) = match heartbeat_count % 4 {
            0 => ("AgentStatus", "2"),
            1 => ("HeartBeat", "1"),
            2 => ("WAStart", "3"),
            _ => ("Provision", "4"),
        };

        let current_telemetry = build_telemetry_event(&latest_goal_state, event_name, event_id, AGENT_NAME, AGENT_VERSION);

        send_telemetry_event(client, &current_telemetry, event_name, heartbeat_count, AGENT_NAME, AGENT_VERSION).await?;
        heartbeat_count += 1;
    }
}
