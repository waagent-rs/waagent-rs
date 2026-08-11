use reqwest::Client;
use quick_xml::de::from_str;
use quick_xml::se::to_string;
use std::time::Duration;
use chrono::Utc;
use crate::wireserver::{GoalState, Health, HealthContainer, HealthRoleInstanceList, HealthRole, HealthState};
use crate::wireserver::{TelemetryData, Param};
use crate::utils::{get_timestamp, get_rfc3339_timestamp, get_user_agent};
use crate::system::SystemInfo;
use base64::prelude::*;

#[cfg(unix)]
use crate::network::add_wireserver_iptables_rule;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// Constants - these should ideally be passed in or configured
const WIRESERVER_ENDPOINT: &str = "http://168.63.129.16";
const HOSTGAPLUGIN_PORT: u16 = 32526;
const WIRESERVER_API_VERSION: &str = "2012-11-30";
const HOSTGAPLUGIN_API_VERSION: &str = "2015-09-01";

/// Fetch the current goal state from WireServer
pub async fn fetch_goal_state(client: &Client) -> Result<GoalState> {
    let response_result = client
        .get(format!("{}/machine?comp=goalstate", WIRESERVER_ENDPOINT))
        .header("x-ms-version", WIRESERVER_API_VERSION)
        .timeout(Duration::from_secs(10))
        .send()
        .await;
    
    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            #[cfg(unix)]
            if e.is_timeout() || e.is_connect() {
                eprintln!("Timeout or connection error reaching wireserver: {}", e);
                eprintln!("Attempting to add iptables rule for wireserver access...");
                add_wireserver_iptables_rule().await?;
                
                // Retry the request after adding the iptables rule
                println!("Retrying wireserver connection...");
                client
                    .get(format!("{}/machine?comp=goalstate", WIRESERVER_ENDPOINT))
                    .header("x-ms-version", WIRESERVER_API_VERSION)
                    .timeout(Duration::from_secs(10))
                    .send()
                    .await?
            } else {
                return Err(e.into());
            }
            
            #[cfg(not(unix))]
            return Err(e.into());
        }
    };
    
    let xml = response.text().await?;
    let goal_state = from_str::<GoalState>(&xml)?;
    
    if cfg!(debug_assertions) {
        println!("Received GoalState: {:#?}", goal_state);
    }
    
    Ok(goal_state)
}

/// Fetch vmSettings from HostGAPlugin
pub async fn fetch_vmsettings(client: &Client, goal_state: &GoalState, agent_name: &str) -> Result<serde_json::Value> {
    let url = format!("{}:{}/vmSettings", WIRESERVER_ENDPOINT, HOSTGAPLUGIN_PORT);
    
    let response = client
        .get(&url)
        .header("x-ms-version", HOSTGAPLUGIN_API_VERSION)
        .header("x-ms-agent-name", agent_name)
        .header("x-ms-containerid", &goal_state.container.container_id)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to fetch vmSettings: HTTP {}", response.status()).into());
    }
    
    let json_text = response.text().await?;
    let vm_settings: serde_json::Value = serde_json::from_str(&json_text)?;
    
    if cfg!(debug_assertions) {
        println!("vmSettings response: {}", serde_json::to_string_pretty(&vm_settings).unwrap_or_default());
    }
    
    Ok(vm_settings)
}

/// Send health report to WireServer
pub async fn send_health_report(client: &Client, goal_state: &GoalState, agent_name: &str, agent_version: &str) -> Result<()> {
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
        .post(format!("{}/machine?comp=health", WIRESERVER_ENDPOINT))
        .header("x-ms-version", HOSTGAPLUGIN_API_VERSION)
        .header("x-ms-agent-name", agent_name)
        .header("User-Agent", &get_user_agent(agent_name, agent_version))
        .header("Content-Type", "text/xml;charset=utf-8")
        .timeout(Duration::from_secs(10))
        .body(health_xml.clone())
        .send()
        .await;
        
    let health_response = match health_response_result {
        Ok(resp) => resp,
        Err(e) => {
            #[cfg(unix)]
            if e.is_timeout() || e.is_connect() {
                eprintln!("Timeout sending health report: {}", e);
                add_wireserver_iptables_rule().await?;
                
                // Retry the request
                client
                    .post(format!("{}/machine?comp=health", WIRESERVER_ENDPOINT))
                    .header("x-ms-version", HOSTGAPLUGIN_API_VERSION)
                    .header("x-ms-agent-name", agent_name)
                    .header("User-Agent", &get_user_agent(agent_name, agent_version))
                    .header("Content-Type", "text/xml;charset=utf-8")
                    .timeout(Duration::from_secs(10))
                    .body(health_xml)
                    .send()
                    .await?
            } else {
                return Err(e.into());
            }
            
            #[cfg(not(unix))]
            return Err(e.into());
        }
    };
        
    println!("Health report status: {}", health_response.status());
    
    if cfg!(debug_assertions) {
        let health_response_text = health_response.text().await?;
        println!("Health report response: {}", health_response_text);
    }
    
    Ok(())
}

/// Send status report to HostGAPlugin
pub async fn send_status_report(
    client: &Client, 
    goal_state: &GoalState, 
    handler_statuses: Vec<serde_json::Value>,
    agent_name: &str,
    agent_version: &str
) -> Result<()> {
    let sys_info = SystemInfo::current();
    let status_content = serde_json::json!({
        "version": "1.1",
        "timestampUTC": get_rfc3339_timestamp(),
        "aggregateStatus": {
            "guestAgentStatus": {
                "version": agent_version,
                "status": "Ready",
                "formattedMessage": {
                    "lang": "en-US",
                    "message": "Guest Agent is running"
                },
                "updateStatus": {
                    "expectedVersion": agent_version,
                    "status": "Success",
                    "code": 0,
                    "formattedMessage": {
                        "lang": "en-US",
                        "message": ""
                    }
                }
            },
            "handlerAggregateStatus": handler_statuses,
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
            "osName": sys_info.os_name,
            "osVersion": sys_info.os_version,
            "version": agent_version
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
    println!("[DEBUG] Handler statuses count: {}", handler_statuses.len());
    if !handler_statuses.is_empty() {
        println!("[DEBUG] First handler status: {}", serde_json::to_string(&handler_statuses[0]).unwrap_or_default());
    }
    
    let status_response = client
        .put(format!("{}:{}/status", WIRESERVER_ENDPOINT, HOSTGAPLUGIN_PORT))
        .header("x-ms-version", HOSTGAPLUGIN_API_VERSION)
        .header("x-ms-agent-name", agent_name)
        .header("User-Agent", &get_user_agent(agent_name, agent_version))
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

/// Send telemetry event to WireServer
pub async fn send_telemetry_event(
    client: &Client, 
    telemetry_data: &TelemetryData, 
    event_name: &str, 
    count: u32,
    agent_name: &str,
    agent_version: &str
) -> Result<()> {
    let telemetry_xml = to_string(telemetry_data)?;
    
    println!("Sending {} #{} at {}", event_name, count, Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
    
    let response_result = client
        .post(format!("{}/machine?comp=telemetrydata", WIRESERVER_ENDPOINT))
        .header("x-ms-version", HOSTGAPLUGIN_API_VERSION)
        .header("x-ms-agent-name", agent_name)
        .header("User-Agent", &get_user_agent(agent_name, agent_version))
        .header("Content-Type", "text/xml;charset=utf-8")
        .timeout(Duration::from_secs(10))
        .body(telemetry_xml.clone())
        .send()
        .await;
        
    let response = match response_result {
        Ok(resp) => resp,
        Err(e) => {
            #[cfg(unix)]
            if e.is_timeout() || e.is_connect() {
                eprintln!("Timeout sending telemetry event {}: {}", event_name, e);
                add_wireserver_iptables_rule().await?;
                
                // Retry the request
                client
                    .post(format!("{}/machine?comp=telemetrydata", WIRESERVER_ENDPOINT))
                    .header("x-ms-version", HOSTGAPLUGIN_API_VERSION)
                    .header("x-ms-agent-name", agent_name)
                    .header("User-Agent", &get_user_agent(agent_name, agent_version))
                    .header("Content-Type", "text/xml;charset=utf-8")
                    .timeout(Duration::from_secs(10))
                    .body(telemetry_xml)
                    .send()
                    .await?
            } else {
                return Err(e.into());
            }
            
            #[cfg(not(unix))]
            return Err(e.into());
        }
    };
        
    println!("{} #{} status: {}", event_name, count, response.status());
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        eprintln!("Telemetry error: {}", error_text);
    }
    
    Ok(())
}

/// Create base parameters for telemetry events
pub fn create_base_params(goal_state: &GoalState, agent_version: &str) -> Vec<Param> {
    vec![
        Param {
            name: "Version".to_string(),
            value: agent_version.to_string(),
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

/// Create WAStart telemetry event
pub fn create_wa_start_telemetry(goal_state: &GoalState, agent_name: &str, agent_version: &str) -> TelemetryData {
    TelemetryData {
        version: "1.0".to_string(),
        provider: crate::wireserver::Provider {
            id: agent_name.to_string(),
            event: crate::wireserver::Event {
                id: "3".to_string(),
                event_data: crate::wireserver::EventData {
                    name: "WAStart".to_string(),
                    param: vec![
                        Param {
                            name: "Version".to_string(),
                            value: agent_version.to_string(),
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

/// Create Provision telemetry event
pub fn create_provision_telemetry(goal_state: &GoalState, agent_name: &str, agent_version: &str) -> TelemetryData {
    TelemetryData {
        version: "1.0".to_string(),
        provider: crate::wireserver::Provider {
            id: agent_name.to_string(),
            event: crate::wireserver::Event {
                id: "4".to_string(),
                event_data: crate::wireserver::EventData {
                    name: "Provision".to_string(),
                    param: vec![
                        Param {
                            name: "Version".to_string(),
                            value: agent_version.to_string(),
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
