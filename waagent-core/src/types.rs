use serde::{Deserialize, Serialize};

// Re-export Result type for convenience
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Certificate information structure
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Certificate {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@certificateDataFormat")]
    pub certificate_data_format: String,
    #[serde(rename = "@thumbprint")]
    pub thumbprint: String,
    #[serde(rename = "$value")]
    pub data: String,
    #[serde(rename = "@format")]
    pub format: String,
    #[serde(rename = "@store")]
    pub store: String,
}

/// System information structure
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub hostname: String,
    pub os_version_major: u32,
    pub os_version_minor: u32,
    pub os_version_patch: u32,
    pub cpu_count: u32,
    pub memory_total_mb: u32,
    pub uptime_secs: u64,
}
