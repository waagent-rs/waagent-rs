use quick_xml::de::from_str;
use crate::extensions::types::ExtensionsConfig;

/// Extract RunCommand configuration from extensions XML
pub fn extract_runcommand_config(extensions_xml: &str) -> Option<String> {
    // Try to parse the extensions config
    match from_str::<ExtensionsConfig>(extensions_xml) {
        Ok(config) => {
            // Look for RunCommand in plugin settings
            for plugin_setting in &config.plugin_settings.plugin {
                if plugin_setting.name.contains("RunCommand") {
                    return Some(plugin_setting.runtime_settings_element.content.clone());
                }
            }
            None
        }
        Err(e) => {
            eprintln!("Failed to parse extensions XML for RunCommand: {}", e);
            None
        }
    }
}
