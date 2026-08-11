use crate::wireserver::{TelemetryData, Provider, Event, EventData, Param};
use crate::wireserver::GoalState;
use crate::system::SystemStats;
use crate::utils::get_timestamp;

/// Build telemetry parameters for HeartBeat events
pub fn build_heartbeat_params(goal_state: &GoalState, agent_version: &str) -> Vec<Param> {
    let sys_info = SystemStats::current();
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
            value: sys_info.cpu_usage_str(),
        },
        Param {
            name: "Memory".to_string(),
            value: sys_info.memory_usage_str(),
        },
        Param {
            name: "ProcessorTime".to_string(),
            value: sys_info.uptime_seconds_str(),
        },
    ]
}

/// Build telemetry parameters for WAStart events
pub fn build_wastart_params(goal_state: &GoalState, agent_version: &str) -> Vec<Param> {
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
        Param {
            name: "GAState".to_string(),
            value: "Ready".to_string(),
        },
    ]
}

/// Build telemetry parameters for Provision events
pub fn build_provision_params(goal_state: &GoalState, agent_version: &str) -> Vec<Param> {
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
        Param {
            name: "IsVMProvisionedForLogs".to_string(),
            value: "true".to_string(),
        },
        Param {
            name: "ProvisioningState".to_string(),
            value: "Ready".to_string(),
        },
    ]
}

/// Build telemetry parameters for AgentStatus events
pub fn build_agent_status_params(goal_state: &GoalState, agent_version: &str) -> Vec<Param> {
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
            value: format!("Guest Agent is running (Version: {})", agent_version),
        },
    ]
}

/// Build a complete telemetry event for the specified event type
pub fn build_telemetry_event(
    goal_state: &GoalState,
    event_name: &str,
    event_id: &str,
    agent_name: &str,
    agent_version: &str,
) -> TelemetryData {
    let params = match event_name {
        "HeartBeat" => build_heartbeat_params(goal_state, agent_version),
        "WAStart" => build_wastart_params(goal_state, agent_version),
        "Provision" => build_provision_params(goal_state, agent_version),
        "AgentStatus" => build_agent_status_params(goal_state, agent_version),
        _ => {
            // Fallback to base params
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
    };

    TelemetryData {
        version: "1.0".to_string(),
        provider: Provider {
            id: agent_name.to_string(),
            event: Event {
                id: event_id.to_string(),
                event_data: EventData {
                    name: event_name.to_string(),
                    param: params,
                },
            },
        },
    }
}
