use base64::prelude::*;
use chrono::Utc;
use os_info;
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;
use sys_info;
use tokio::time::sleep;

// Constants
const WIRESERVER_ENDPOINT: &str = "http://168.63.129.16";
const STATUS_SERVICE_PORT: u16 = 32526;
const AGENT_VERSION: &str = "waagent-rs/0.0.1";
const AGENT_NAME: &str = "waagent-rs";
const WIRESERVER_API_VERSION: &str = "2012-11-30";
const STATUS_API_VERSION: &str = "2015-09-01";
const HEARTBEAT_INTERVAL_SECS: u64 = 30;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// Helper functions
fn get_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string()
}

fn get_rfc3339_timestamp() -> String {
    Utc::now().to_rfc3339()
}

fn get_user_agent() -> String {
    format!("{}/{}", AGENT_NAME, AGENT_VERSION)
}

fn get_system_info() -> SystemInfo {
    let hostname = match sys_info::hostname() {
        Ok(name) => name,
        Err(e) => {
            eprintln!("Warning: Failed to get hostname: {}", e);
            "Undefined".to_string()
        }
    };
    
    SystemInfo {
        hostname,
        cpu_usage: get_cpu_usage_percent(),
        memory_usage: get_memory_usage_percent(),
        processor_time: get_processor_time(),
    }
}

fn get_cpu_usage_percent() -> String {
    // Get CPU load average as a simple approximation
    match sys_info::loadavg() {
        Ok(load) => format!("{:.1}", load.one * 100.0),
        Err(_) => "0".to_string(),
    }
}

fn get_memory_usage_percent() -> String {
    match (sys_info::mem_info(), sys_info::mem_info()) {
        (Ok(mem), _) => {
            let used = mem.total - mem.free;
            let usage_percent = (used as f64 / mem.total as f64) * 100.0;
            format!("{:.1}", usage_percent)
        }
        _ => "0".to_string(),
    }
}

fn get_processor_time() -> String {
    // Just a quick patch for building on non *nix systems
    #[cfg(not(unix))]
    {
        "0".to_string()
    }

    #[cfg(unix)]
    {
        // Simple approximation using boot time
        match sys_info::boottime() {
            Ok(boot_time) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let uptime = now.saturating_sub(boot_time.tv_sec as u64);
                format!("{}", uptime)
            }
            Err(_) => "0".to_string(),
        }
    }
}

fn get_os_version(_sys_info: &SystemInfo) -> String {
    let info = os_info::get();
    
    let version = info.version().to_string();
    // There could be two bugs in os_info 3.12.0, if confirmed we need to move this comment
    // to a doc in our repo, submit an issue and if possible submit a patch.
    // version for ubuntu should return "24.04.3" but instead returns "24.4.0"
    version
}

fn get_os_display_name() -> String {
    let info = os_info::get();

    let os_type = info.os_type().to_string().to_lowercase();
    os_type
}

async fn add_wireserver_iptables_rule() -> Result<()> {
    println!("Adding iptables rule for wireserver access...");
    
    // First, check if the rule already exists in the security table OUTPUT chain
    let check_existing = Command::new("sudo")
        .args(&[
            "iptables", 
            "-t", "security",
            "-C", "OUTPUT", 
            "-d", "168.63.129.16/32",
            "-p", "tcp",
            "-m", "owner",
            "--uid-owner", "999",
            "-j", "ACCEPT"
        ])
        .output();
        
    match check_existing {
        Ok(result) => {
            if result.status.success() {
                println!("Iptables rule for wireserver already exists in security table OUTPUT chain, skipping");
                return Ok(());
            }
        }
        Err(_) => {
            // Rule doesn't exist or check failed, continue to add it
        }
    }
    
    println!("Inserting iptables rule at position 2 in security table OUTPUT chain");
    
    let output = Command::new("sudo")
        .args(&[
            "iptables",
            "-t", "security",
            "-I", "OUTPUT", "2",
            "-d", "168.63.129.16/32",
            "-p", "tcp",
            "-m", "owner",
            "--uid-owner", "999",
            "-j", "ACCEPT"
        ])
        .output();
        
    match output {
        Ok(result) => {
            if result.status.success() {
                println!("Successfully added iptables rule for wireserver to security table OUTPUT chain at position 2");
                
                // Show the current security table OUTPUT rules for debugging
                if cfg!(debug_assertions) {
                    let show_rules = Command::new("sudo")
                        .args(&["iptables", "-t", "security", "-L", "OUTPUT", "-n", "--line-numbers"])
                        .output();
                    if let Ok(rules_result) = show_rules {
                        let rules_output = String::from_utf8_lossy(&rules_result.stdout);
                        println!("Current security table OUTPUT chain rules:\n{}", rules_output);
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                eprintln!("Failed to add iptables rule: {}", stderr);
            }
        }
        Err(e) => {
            eprintln!("Error executing iptables command: {}", e);
        }
    }
    
    Ok(())
}


#[derive(Debug)]
struct SystemInfo {
    hostname: String,
    cpu_usage: String,
    memory_usage: String,
    processor_time: String,
}

fn create_base_params(goal_state: &GoalState) -> Vec<Param> {
    vec![
        Param {
            name: "Version".to_string(),
            value: AGENT_VERSION.to_string(),
        },
        Param {
            name: "Timestamp".to_string(),
            value: get_timestamp(),
        },
        Param {
            name: "Container".to_string(),
            value: goal_state.container.container_id.clone(),
        },
        Param {
            name: "RoleInstance".to_string(),
            value: goal_state.container.role_instance_list.role_instance.instance_id.clone(),
        },
    ]
}

#[derive(Debug, Deserialize)]
struct GoalState {
    #[allow(dead_code)]
    #[serde(rename = "Version")]
    version: String,
    #[serde(rename = "Incarnation")]
    incarnation: u32,
    #[allow(dead_code)]
    #[serde(rename = "Machine")]
    machine: Machine,
    #[serde(rename = "Container")]
    container: Container,
}

#[derive(Debug, Deserialize)]
struct Machine {
    #[allow(dead_code)]
    #[serde(rename = "ExpectedState")]
    expected_state: String,
    #[allow(dead_code)]
    #[serde(rename = "StopRolesDeadlineHint")]
    stop_roles_deadline_hint: u32,
    #[allow(dead_code)]
    #[serde(rename = "LBProbePorts")]
    lb_probe_ports: LBProbePorts,
    #[allow(dead_code)]
    #[serde(rename = "ExpectHealthReport")]
    expect_health_report: String,
}

#[derive(Debug, Deserialize)]
struct LBProbePorts {
    #[allow(dead_code)]
    #[serde(rename = "Port")]
    port: u16,
}

#[derive(Debug, Deserialize)]
struct Container {
    #[serde(rename = "ContainerId")]
    container_id: String,
    #[serde(rename = "RoleInstanceList")]
    role_instance_list: RoleInstanceList,
}

#[derive(Debug, Deserialize)]
struct RoleInstanceList {
    #[serde(rename = "RoleInstance")]
    role_instance: RoleInstance,
}

#[derive(Debug, Deserialize)]
struct RoleInstance {
    #[serde(rename = "InstanceId")]
    instance_id: String,
    #[allow(dead_code)]
    #[serde(rename = "State")]
    state: String,
    #[allow(dead_code)]
    #[serde(rename = "Configuration")]
    configuration: Configuration,
}

#[derive(Debug, Deserialize)]
struct Configuration {
    #[allow(dead_code)]
    #[serde(rename = "HostingEnvironmentConfig")]
    hosting_environment_config: String,
    #[allow(dead_code)]
    #[serde(rename = "SharedConfig")]
    shared_config: String,
    #[allow(dead_code)]
    #[serde(rename = "ExtensionsConfig")]
    extensions_config: String,
    #[allow(dead_code)]
    #[serde(rename = "FullConfig")]
    full_config: String,
    #[allow(dead_code)]
    #[serde(rename = "Certificates")]
    certificates: String,
    #[allow(dead_code)]
    #[serde(rename = "ConfigName")]
    config_name: String,
}

// Health report structures for XML generation
#[derive(Debug, Serialize)]
struct Health {
    #[serde(rename = "GoalStateIncarnation")]
    goal_state_incarnation: u32,
    #[serde(rename = "Container")]
    container: HealthContainer,
}

#[derive(Debug, Serialize)]
struct HealthContainer {
    #[serde(rename = "ContainerId")]
    container_id: String,
    #[serde(rename = "RoleInstanceList")]
    role_instance_list: HealthRoleInstanceList,
}

#[derive(Debug, Serialize)]
struct HealthRoleInstanceList {
    #[serde(rename = "Role")]
    role: HealthRole,
}

#[derive(Debug, Serialize)]
struct HealthRole {
    #[serde(rename = "InstanceId")]
    instance_id: String,
    #[serde(rename = "Health")]
    health: HealthState,
}

#[derive(Debug, Serialize)]
struct HealthState {
    #[serde(rename = "State")]
    state: String,
}

// Agent status structures for XML generation
#[derive(Debug, Serialize)]
struct TelemetryData {
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "Provider")]
    provider: Provider,
}

#[derive(Debug, Serialize)]
struct Provider {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "Event")]
    event: Event,
}

#[derive(Debug, Serialize)]
struct Event {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "EventData")]
    event_data: EventData,
}

#[derive(Debug, Serialize)]
struct EventData {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "Param")]
    param: Vec<Param>,
}

#[derive(Debug, Serialize)]
struct Param {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@value")]
    value: String,
}

fn create_wa_start_telemetry(goal_state: &GoalState) -> TelemetryData {
    TelemetryData {
        version: "1.0".to_string(),
        provider: Provider {
            id: AGENT_NAME.to_string(),
            event: Event {
                id: "3".to_string(),
                event_data: EventData {
                    name: "WAStart".to_string(),
                    param: vec![
                        Param {
                            name: "Version".to_string(),
                            value: AGENT_VERSION.to_string(),
                        },
                        Param {
                            name: "GAState".to_string(),
                            value: "Ready".to_string(),
                        },
                        Param {
                            name: "Container".to_string(),
                            value: goal_state.container.container_id.clone(),
                        },
                        Param {
                            name: "RoleInstance".to_string(),
                            value: goal_state.container.role_instance_list.role_instance.instance_id.clone(),
                        },
                        Param {
                            name: "Timestamp".to_string(),
                            value: get_timestamp(),
                        },
                    ],
                },
            },
        },
    }
}

fn create_provision_telemetry(goal_state: &GoalState) -> TelemetryData {
    TelemetryData {
        version: "1.0".to_string(),
        provider: Provider {
            id: AGENT_NAME.to_string(),
            event: Event {
                id: "4".to_string(),
                event_data: EventData {
                    name: "Provision".to_string(),
                    param: vec![
                        Param {
                            name: "Version".to_string(),
                            value: AGENT_VERSION.to_string(),
                        },
                        Param {
                            name: "IsVMProvisionedForLogs".to_string(),
                            value: "true".to_string(),
                        },
                        Param {
                            name: "ProvisioningState".to_string(),
                            value: "Ready".to_string(),
                        },
                        Param {
                            name: "Container".to_string(),
                            value: goal_state.container.container_id.clone(),
                        },
                        Param {
                            name: "RoleInstance".to_string(),
                            value: goal_state.container.role_instance_list.role_instance.instance_id.clone(),
                        },
                        Param {
                            name: "Timestamp".to_string(),
                            value: get_timestamp(),
                        },
                    ],
                },
            },
        },
    }
}

async fn run_heartbeat_loop(client: &Client, goal_state: &GoalState) -> Result<()> {
    let mut heartbeat_count = 1;
    
    loop {
        sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECS)).await;
        
        // Cycle through different event types
        let (event_name, event_id) = match heartbeat_count % 4 {
            0 => ("AgentStatus", "2"),
            1 => ("HeartBeat", "1"),
            2 => ("WAStart", "3"),
            _ => ("Provision", "4"),
        };
        
        let mut params = create_base_params(goal_state);
        
        match event_name {
            "HeartBeat" => {
                let sys_info = get_system_info();
                params.extend(vec![
                    Param {
                        name: "IsVersionFromRSM".to_string(),
                        value: "true".to_string(),
                    },
                    Param {
                        name: "GAState".to_string(),
                        value: "Ready".to_string(),
                    },
                    Param {
                        name: "Role".to_string(),
                        value: goal_state.container.role_instance_list.role_instance.instance_id.clone(),
                    },
                    Param {
                        name: "CPU".to_string(),
                        value: sys_info.cpu_usage,
                    },
                    Param {
                        name: "Memory".to_string(),
                        value: sys_info.memory_usage,
                    },
                    Param {
                        name: "ProcessorTime".to_string(),
                        value: sys_info.processor_time,
                    },
                ]);
            },
            "WAStart" => {
                params.push(Param {
                    name: "GAState".to_string(),
                    value: "Ready".to_string(),
                });
            },
            "Provision" => {
                params.extend(vec![
                    Param {
                        name: "IsVMProvisionedForLogs".to_string(),
                        value: "true".to_string(),
                    },
                    Param {
                        name: "ProvisioningState".to_string(),
                        value: "Ready".to_string(),
                    },
                ]);
            },
            "AgentStatus" => {
                params.extend(vec![
                    Param {
                        name: "Status".to_string(),
                        value: "Ready".to_string(),
                    },
                    Param {
                        name: "Message".to_string(),
                        value: "Guest Agent is running".to_string(),
                    },
                    Param {
                        name: "FormattedMessage".to_string(),
                        value: format!("Guest Agent is running (Version: {})", AGENT_VERSION),
                    },
                ]);
            },
            _ => {}
        }
        
        let current_telemetry = TelemetryData {
            version: "1.0".to_string(),
            provider: Provider {
                id: AGENT_NAME.to_string(),
                event: Event {
                    id: event_id.to_string(),
                    event_data: EventData {
                        name: event_name.to_string(),
                        param: params,
                    },
                },
            },
        };
        
        send_telemetry_event(client, &current_telemetry, event_name, heartbeat_count).await?;
        heartbeat_count += 1;
    }
}

async fn fetch_goal_state(client: &Client) -> Result<GoalState> {
    let response_result = client
        .get(&format!("{}/machine?comp=goalstate", WIRESERVER_ENDPOINT))
        .header("x-ms-version", WIRESERVER_API_VERSION)
        .timeout(Duration::from_secs(10))
        .send()
        .await;
    
    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            if e.is_timeout() || e.is_connect() {
                eprintln!("Timeout or connection error reaching wireserver: {}", e);
                eprintln!("Attempting to add iptables rule for wireserver access...");
                add_wireserver_iptables_rule().await?;
                
                // Retry the request after adding the iptables rule
                println!("Retrying wireserver connection...");
                client
                    .get(&format!("{}/machine?comp=goalstate", WIRESERVER_ENDPOINT))
                    .header("x-ms-version", WIRESERVER_API_VERSION)
                    .timeout(Duration::from_secs(10))
                    .send()
                    .await?
            } else {
                return Err(e.into());
            }
        }
    };
    
    let xml = response.text().await?;
    let goal_state = from_str::<GoalState>(&xml)?;
    
    if cfg!(debug_assertions) {
        println!("Received GoalState: {:#?}", goal_state);
    }
    
    Ok(goal_state)
}

async fn send_health_report(client: &Client, goal_state: &GoalState) -> Result<()> {
    let health_report = Health {
        goal_state_incarnation: goal_state.incarnation,
        container: HealthContainer {
            container_id: goal_state.container.container_id.clone(),
            role_instance_list: HealthRoleInstanceList {
                role: HealthRole {
                    instance_id: goal_state.container.role_instance_list.role_instance.instance_id.clone(),
                    health: HealthState {
                        state: "Ready".to_string(),
                    },
                },
            },
        },
    };

    let health_xml = to_string(&health_report)?;
    if cfg!(debug_assertions) {
        println!("Generated health report XML: {}", health_xml);
    }

    let health_response_result = client
        .post(&format!("{}/machine?comp=health", WIRESERVER_ENDPOINT))
        .header("x-ms-version", WIRESERVER_API_VERSION)
        .header("x-ms-agent-name", AGENT_NAME)
        .header("User-Agent", &get_user_agent())
        .header("Content-Type", "text/xml;charset=utf-8")
        .timeout(Duration::from_secs(10))
        .body(health_xml.clone())
        .send()
        .await;
        
    let health_response = match health_response_result {
        Ok(resp) => resp,
        Err(e) => {
            if e.is_timeout() || e.is_connect() {
                eprintln!("Timeout sending health report: {}", e);
                add_wireserver_iptables_rule().await?;
                
                // Retry the request
                client
                    .post(&format!("{}/machine?comp=health", WIRESERVER_ENDPOINT))
                    .header("x-ms-version", WIRESERVER_API_VERSION)
                    .header("x-ms-agent-name", AGENT_NAME)
                    .header("User-Agent", &get_user_agent())
                    .header("Content-Type", "text/xml;charset=utf-8")
                    .timeout(Duration::from_secs(10))
                    .body(health_xml)
                    .send()
                    .await?
            } else {
                return Err(e.into());
            }
        }
    };
        
    println!("Health report status: {}", health_response.status());
    
    if cfg!(debug_assertions) {
        let health_response_text = health_response.text().await?;
        println!("Health report response: {}", health_response_text);
    }
    
    Ok(())
}

async fn send_status_report(client: &Client, goal_state: &GoalState) -> Result<()> {
    let sys_info = get_system_info();
    
    let status_content = serde_json::json!({
        "version": "1.1",
        "timestampUTC": get_rfc3339_timestamp(),
        "aggregateStatus": {
            "guestAgentStatus": {
                "version": AGENT_VERSION,
                "status": "Ready",
                "formattedMessage": {
                    "lang": "en-US",
                    "message": "Guest Agent is running"
                },
                "updateStatus": {
                    "expectedVersion": AGENT_VERSION,
                    "status": "Success",
                    "code": 0,
                    "formattedMessage": {
                        "lang": "en-US",
                        "message": ""
                    }
                }
            },
            "handlerAggregateStatus": [],
            "vmArtifactsAggregateStatus": {
                "goalStateAggregateStatus": {
                    "formattedMessage": {
                        "lang": "en-US",
                        "message": "GoalState executed successfully"
                    },
                    "timestampUTC": get_rfc3339_timestamp(),
                    "inSvdSeqNo": goal_state.incarnation.to_string(),
                    "status": "Success",
                    "code": 0
                }
            }
        },
        "guestOSInfo": {
            "computerName": sys_info.hostname,
            "osName": get_os_display_name(),
            "osVersion": get_os_version(&sys_info),
            "version": AGENT_VERSION
        },
        "supportedFeatures": [
            {"Key": "MultipleExtensionsPerHandler", "Value": "1.0"},
            {"Key": "VersioningGovernance", "Value": "1.0"},
            {"Key": "FastTrack", "Value": "1.0"}
        ]
    });
    
    let status_content_str = serde_json::to_string(&status_content)?;
    let status_content_b64 = BASE64_STANDARD.encode(status_content_str.as_bytes());
    
    let status_payload = serde_json::json!({
        "content": status_content_b64,
        "headers": [
            {"headerName": "Content-Length", "headerValue": "1024"},
            {"headerName": "x-ms-date", "headerValue": get_timestamp()},
            {"headerName": "x-ms-range", "headerValue": "bytes=0-1023"},
            {"headerName": "x-ms-page-write", "headerValue": "update"},
            {"headerName": "x-ms-version", "headerValue": "2014-02-14"}
        ],
        "requestUri": format!("https://md-hdd-placeholder.z27.blob.storage.azure.net/$system/gpg.{}.status", 
            goal_state.container.container_id)
    });
    
    println!("Sending status report to status service...");
    
    let status_response = client
        .put(&format!("{}:{}/status", WIRESERVER_ENDPOINT, STATUS_SERVICE_PORT))
        .header("x-ms-version", STATUS_API_VERSION)
        .header("x-ms-agent-name", AGENT_NAME)
        .header("User-Agent", &get_user_agent())
        .header("Content-Type", "application/json")
        .header("x-ms-containerid", &goal_state.container.container_id)
        .header("x-ms-host-config-name", format!("{}.0.{}.0._gpg.1.xml", 
            goal_state.container.role_instance_list.role_instance.instance_id,
            goal_state.container.role_instance_list.role_instance.instance_id))
        .json(&status_payload)
        .send()
        .await?;
        
    println!("Status service response: {}", status_response.status());
    if cfg!(debug_assertions) {
        let status_response_text = status_response.text().await?;
        println!("Status service response body: {}", status_response_text);
    }
    
    Ok(())
}

async fn send_telemetry_event(client: &Client, telemetry_data: &TelemetryData, event_name: &str, count: u32) -> Result<()> {
    let telemetry_xml = to_string(telemetry_data)?;
    
    println!("Sending {} #{} at {}", event_name, count, Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
    
    let response_result = client
        .post(&format!("{}/machine?comp=telemetrydata", WIRESERVER_ENDPOINT))
        .header("x-ms-version", WIRESERVER_API_VERSION)
        .header("x-ms-agent-name", AGENT_NAME)
        .header("User-Agent", &get_user_agent())
        .header("Content-Type", "text/xml;charset=utf-8")
        .timeout(Duration::from_secs(10))
        .body(telemetry_xml.clone())
        .send()
        .await;
        
    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            if e.is_timeout() || e.is_connect() {
                eprintln!("Timeout sending telemetry event {}: {}", event_name, e);
                add_wireserver_iptables_rule().await?;
                
                // Retry the request
                client
                    .post(&format!("{}/machine?comp=telemetrydata", WIRESERVER_ENDPOINT))
                    .header("x-ms-version", WIRESERVER_API_VERSION)
                    .header("x-ms-agent-name", AGENT_NAME)
                    .header("User-Agent", &get_user_agent())
                    .header("Content-Type", "text/xml;charset=utf-8")
                    .timeout(Duration::from_secs(10))
                    .body(telemetry_xml)
                    .send()
                    .await?
            } else {
                return Err(e.into());
            }
        }
    };
        
    println!("{} #{} status: {}", event_name, count, response.status());
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        eprintln!("Telemetry error: {}", error_text);
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::new();
    
    // Fetch goal state
    let goal_state = fetch_goal_state(&client).await?;
    
    // Send health report
    send_health_report(&client, &goal_state).await?;
    
    // Send initial startup events
    println!("Sending initial agent startup events...");
    
    let wa_start_telemetry = create_wa_start_telemetry(&goal_state);
    send_telemetry_event(&client, &wa_start_telemetry, "WAStart", 0).await?;
    
    sleep(Duration::from_secs(2)).await;
    
    let provision_telemetry = create_provision_telemetry(&goal_state);
    send_telemetry_event(&client, &provision_telemetry, "Provision", 0).await?;
    
    // Send status report to status service (this is what the portal reads!)
    send_status_report(&client, &goal_state).await?;
    
    println!("Starting continuous heartbeat loop (send SIGINT/Ctrl+C to stop)...");
    
    // Continuous heartbeat loop
    run_heartbeat_loop(&client, &goal_state).await?;
    
    Ok(())
}
