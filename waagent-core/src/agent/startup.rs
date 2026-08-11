use reqwest::Client;
use tokio::time::{sleep, Duration};

use crate::config::{AGENT_NAME, AGENT_VERSION, HOSTGAPLUGIN_API_VERSION};
use crate::crypto::download_and_save_certificates;
use crate::extensions::{process_extensions, extract_runcommand_config, execute_run_command};
use crate::wireserver::{GoalState, fetch_goal_state, send_health_report, send_status_report, send_telemetry_event, create_wa_start_telemetry, create_provision_telemetry};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Initialize the agent and perform startup tasks
/// 
/// This function:
/// - Fetches the initial goal state
/// - Sends health report
/// - Sends startup telemetry events (WAStart, Provision)
/// - Downloads certificates
/// - Processes initial extensions (if any)
/// - Sends initial status report
/// 
/// Returns the initial goal state for use by the heartbeat loop
pub async fn initialize_agent(client: &Client) -> Result<GoalState> {
    // Fetch goal state
    let goal_state = fetch_goal_state(client).await?;
    
    // Send health report
    send_health_report(client, &goal_state, AGENT_NAME, AGENT_VERSION).await?;
    
    // Send initial startup events
    println!("Sending initial agent startup events...");
    let wa_start_telemetry = create_wa_start_telemetry(&goal_state, AGENT_NAME, AGENT_VERSION);
    send_telemetry_event(client, &wa_start_telemetry, "WAStart", 0, AGENT_NAME, AGENT_VERSION).await?;
    sleep(Duration::from_secs(2)).await;
    let provision_telemetry = create_provision_telemetry(&goal_state, AGENT_NAME, AGENT_VERSION);
    send_telemetry_event(client, &provision_telemetry, "Provision", 0, AGENT_NAME, AGENT_VERSION).await?;
    
    // Download certificates before processing extensions
    if let Err(e) = download_and_save_certificates(client, &goal_state, AGENT_NAME).await {
        eprintln!("Failed to download certificates: {}", e);
    }
    
    // Process extensions if configured
    let extensions_config_url = &goal_state.container.role_instance_list.role_instance.configuration.extensions_config;
    if !extensions_config_url.is_empty() {
        println!("\n=== Processing Extensions ===");
        match process_extensions(client, extensions_config_url, &goal_state).await {
            Ok(handler_statuses) => {
                println!("Successfully processed {} extensions", handler_statuses.len());
                
                // Check if any extension is RunCommandLinux and execute it
                for handler in &handler_statuses {
                    if handler.handler_name.contains("RunCommand") {
                        println!("\n=== Found RunCommand Extension ===");
                        println!("Attempting to execute RunCommand...");
                        
                        // Fetch the extension configuration again to execute
                        match client
                            .get(extensions_config_url)
                            .header("x-ms-version", HOSTGAPLUGIN_API_VERSION)
                            .header("x-ms-agent-name", AGENT_NAME)
                            .timeout(Duration::from_secs(10))
                            .send()
                            .await
                        {
                            Ok(ext_response) => {
                                if let Ok(ext_xml) = ext_response.text().await {
                                    // Parse to find RunCommand settings
                                    if let Some(config_json) = extract_runcommand_config(&ext_xml) {
                                        match execute_run_command(&config_json, client, &goal_state).await {
                                            Ok(result) => {
                                                println!("RunCommand execution result:\n{}", result);
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to execute RunCommand: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to fetch extension config for RunCommand: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to process extensions: {}", e);
            }
        }
    }
    
    // Send status report to status service (this is what the portal reads!)
    send_status_report(client, &goal_state, Vec::new(), AGENT_NAME, AGENT_VERSION).await?;
    
    Ok(goal_state)
}
