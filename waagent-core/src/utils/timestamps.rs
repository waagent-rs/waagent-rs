use chrono::Utc;

/// Get timestamp in Azure-specific format (YYYY-MM-DDTHH:MM:SS.fffZ)
pub fn get_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string()
}

/// Get RFC3339 timestamp
pub fn get_rfc3339_timestamp() -> String {
    Utc::now().to_rfc3339()
}

/// Get user agent string for Azure Guest Agent
pub fn get_user_agent(agent_name: &str, agent_version: &str) -> String {
    format!("{}/{}", agent_name, agent_version)
}
