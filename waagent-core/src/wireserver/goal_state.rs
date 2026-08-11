use serde::Deserialize;

/// Azure WireServer GoalState document
#[derive(Debug, Deserialize, Clone)]
pub struct GoalState {
    #[allow(dead_code)]
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Incarnation")]
    pub incarnation: u32,
    #[allow(dead_code)]
    #[serde(rename = "Machine")]
    pub machine: Machine,
    #[serde(rename = "Container")]
    pub container: Container,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Machine {
    #[allow(dead_code)]
    #[serde(rename = "ExpectedState")]
    pub expected_state: String,
    #[allow(dead_code)]
    #[serde(rename = "StopRolesDeadlineHint")]
    pub stop_roles_deadline_hint: u32,
    #[allow(dead_code)]
    #[serde(rename = "LBProbePorts")]
    pub lb_probe_ports: LBProbePorts,
    #[allow(dead_code)]
    #[serde(rename = "ExpectHealthReport")]
    pub expect_health_report: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LBProbePorts {
    #[allow(dead_code)]
    #[serde(rename = "Port")]
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Container {
    #[serde(rename = "ContainerId")]
    pub container_id: String,
    #[serde(rename = "RoleInstanceList")]
    pub role_instance_list: RoleInstanceList,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RoleInstanceList {
    #[serde(rename = "RoleInstance")]
    pub role_instance: RoleInstance,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RoleInstance {
    #[serde(rename = "InstanceId")]
    pub instance_id: String,
    #[allow(dead_code)]
    #[serde(rename = "State")]
    pub state: String,
    #[allow(dead_code)]
    #[serde(rename = "Configuration")]
    pub configuration: Configuration,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Configuration {
    #[allow(dead_code)]
    #[serde(rename = "HostingEnvironmentConfig")]
    pub hosting_environment_config: String,
    #[allow(dead_code)]
    #[serde(rename = "SharedConfig")]
    pub shared_config: String,
    #[allow(dead_code)]
    #[serde(rename = "ExtensionsConfig")]
    pub extensions_config: String,
    #[allow(dead_code)]
    #[serde(rename = "FullConfig")]
    pub full_config: String,
    #[allow(dead_code)]
    #[serde(rename = "Certificates")]
    pub certificates: String,
    #[allow(dead_code)]
    #[serde(rename = "ConfigName")]
    pub config_name: String,
}
