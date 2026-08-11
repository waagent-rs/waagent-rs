use serde::{Deserialize, Serialize};

// Re-export Result type for convenience
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Extension configuration from Azure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExtensionsConfig {
    #[serde(rename = "Plugins", default)]
    pub plugins: Plugins,
    #[serde(rename = "PluginSettings", default)]
    pub plugin_settings: PluginSettings,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]  
pub struct Plugins {
    #[serde(rename = "Plugin", default)]
    pub plugin: Vec<PluginDefinition>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]  
pub struct PluginSettings {
    #[serde(rename = "Plugin", default)]
    pub plugin: Vec<PluginRuntimeSettings>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PluginDefinition {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "@location", default)]
    pub location: String,
    #[serde(rename = "@state")]
    pub state: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRuntimeSettings {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "RuntimeSettings")]
    pub runtime_settings_element: RuntimeSettingsElement,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSettingsElement {
    #[serde(rename = "@seqNo")]
    pub seq_no: String,
    #[serde(rename = "$value")]
    pub content: String,
}

/// Handler status for reporting back to Azure
#[derive(Debug, Serialize)]
pub struct HandlerStatus {
    #[serde(rename = "handlerName")]
    pub handler_name: String,
    #[serde(rename = "handlerVersion")]
    pub handler_version: String,
    pub status: String,
    pub code: i32,
    #[serde(rename = "formattedMessage")]
    pub formatted_message: FormattedMessage,
}

#[derive(Debug, Serialize)]
pub struct FormattedMessage {
    pub lang: String,
    pub message: String,
}
