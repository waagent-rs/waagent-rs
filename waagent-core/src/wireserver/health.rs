use serde::Serialize;

/// Health report structures for XML generation and submission to WireServer
#[derive(Debug, Serialize)]
pub struct Health {
    #[serde(rename = "GoalStateIncarnation")]
    pub goal_state_incarnation: u32,
    #[serde(rename = "Container")]
    pub container: HealthContainer,
}

#[derive(Debug, Serialize)]
pub struct HealthContainer {
    #[serde(rename = "ContainerId")]
    pub container_id: String,
    #[serde(rename = "RoleInstanceList")]
    pub role_instance_list: HealthRoleInstanceList,
}

#[derive(Debug, Serialize)]
pub struct HealthRoleInstanceList {
    #[serde(rename = "Role")]
    pub role: HealthRole,
}

#[derive(Debug, Serialize)]
pub struct HealthRole {
    #[serde(rename = "InstanceId")]
    pub instance_id: String,
    #[serde(rename = "Health")]
    pub health: HealthState,
}

#[derive(Debug, Serialize)]
pub struct HealthState {
    #[serde(rename = "State")]
    pub state: String,
}
