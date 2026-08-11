use super::types::*;
use super::installer::download_and_install_extension;
use quick_xml::de::from_str;
use reqwest::Client;
use std::time::Duration;

// Constants
const WIRESERVER_API_VERSION: &str = "2012-11-30";  // For WireServer extensionsConfig (legacy)
const AGENT_NAME: &str = "waagent-rs";

/// Get user agent string
fn get_user_agent() -> String {
    format!("{} (Linux)", "waagent-rs/0.0.1")
}

/// Process extensions from Azure configuration
pub async fn process_extensions<T>(
    client: &Client, 
    extensions_config_url: &str,
    _goal_state: &T,
) -> Result<Vec<HandlerStatus>> 
where
    T: std::fmt::Debug,
{
    let mut handler_statuses = Vec::new();

    println!("=== Processing Extensions ===");
    println!("Extensions config URL: {}", extensions_config_url);

    if extensions_config_url.is_empty() {
        println!("No extensions configuration URL provided, skipping extension processing");
        return Ok(handler_statuses);
    }

    // Fetch extensions configuration
    println!("Fetching extensions configuration from: {}", extensions_config_url);
    
    let extensions_response = client
        .get(extensions_config_url)
        .header("x-ms-version", WIRESERVER_API_VERSION)
        .header("x-ms-agent-name", AGENT_NAME)
        .header("User-Agent", &get_user_agent())
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    println!("Extensions config response status: {}", extensions_response.status());
    if !extensions_response.status().is_success() {
        eprintln!("Failed to fetch extensions config: HTTP {}", extensions_response.status());
        return Ok(handler_statuses);
    }

    let extensions_xml = extensions_response.text().await?;
    println!("Extensions config XML length: {} bytes", extensions_xml.len());
    
    // Always print first 500 chars to debug parsing issues
    if !extensions_xml.is_empty() {
        let preview = if extensions_xml.len() > 500 { &extensions_xml[..500] } else { &extensions_xml };
        println!("Extensions config XML preview:\n{}", preview);
    }
    
    if cfg!(debug_assertions) {
        println!("Extensions config XML content:\n{}", extensions_xml);
    }

    // If XML is empty or just whitespace, there are no extensions
    if extensions_xml.trim().is_empty() {
        println!("Extensions configuration is empty - no extensions to process");
        return Ok(handler_statuses);
    }

    // Parse extensions configuration
    println!("Parsing extensions configuration...");
    match from_str::<ExtensionsConfig>(&extensions_xml) {
        Ok(config) => {
            println!("Successfully parsed extensions configuration");
            if cfg!(debug_assertions) {
                println!("Parsed config structure: {:#?}", config);
            }
            println!("Number of plugin definitions found: {}", config.plugins.plugin.len());
            println!("Number of plugin settings found: {}", config.plugin_settings.plugin.len());
            
            // Debug: print all plugin names found
            if !config.plugins.plugin.is_empty() {
                println!("Plugin names:");
                for p in &config.plugins.plugin {
                    println!("  - {} v{} (state: {})", p.name, p.version, p.state);
                }
            }
            if !config.plugin_settings.plugin.is_empty() {
                println!("Plugin settings names:");
                for p in &config.plugin_settings.plugin {
                    println!("  - {} v{}", p.name, p.version);
                }
            }
            
            if config.plugins.plugin.is_empty() {
                println!("No plugin definitions configured");
                return Ok(handler_statuses);
            }
            
            for (index, plugin_def) in config.plugins.plugin.iter().enumerate() {
                println!("Plugin Definition {}: {} v{} (state: {})", 
                    index + 1, plugin_def.name, plugin_def.version, plugin_def.state);
                
                // Look for corresponding runtime settings
                let runtime_config = config.plugin_settings.plugin.iter()
                    .find(|s| s.name == plugin_def.name && s.version == plugin_def.version)
                    .map(|s| s.runtime_settings_element.content.as_str())
                    .unwrap_or("");

                // Parse and display configuration if available
                if !runtime_config.is_empty() {
                    println!("  Runtime Configuration: {}", runtime_config);
                    
                    // Try to parse the config as JSON for better display
                    if let Ok(parsed_config) = serde_json::from_str::<serde_json::Value>(runtime_config) {
                        if let Ok(pretty_config) = serde_json::to_string_pretty(&parsed_config) {
                            println!("  Parsed Configuration:\n{}", pretty_config.lines()
                                .map(|line| format!("    {}", line))
                                .collect::<Vec<_>>()
                                .join("\n"));
                        }
                    }
                } else {
                    println!("  Runtime Configuration: None");
                }

                // Download and install extension if needed
                if plugin_def.state == "enabled" {
                    println!("Installing enabled extension: {}", plugin_def.name);
                    match download_and_install_extension(client, plugin_def, runtime_config).await {
                        Ok(_) => {
                            handler_statuses.push(HandlerStatus {
                                handler_name: plugin_def.name.clone(),
                                handler_version: plugin_def.version.clone(),
                                status: "Ready".to_string(),
                                code: 0,
                                formatted_message: FormattedMessage {
                                    lang: "en-US".to_string(),
                                    message: format!("Extension {} is ready", plugin_def.name),
                                },
                            });
                        }
                        Err(e) => {
                            let error_msg = e.to_string();
                            if error_msg.contains("Permission denied") {
                                println!("Extension installation failed: {}", error_msg);
                                println!("   Make sure sudo is available for privileged operations");
                            } else {
                                eprintln!("Failed to install extension {}: {}", plugin_def.name, e);
                            }
                            
                            handler_statuses.push(HandlerStatus {
                                handler_name: plugin_def.name.clone(),
                                handler_version: plugin_def.version.clone(),
                                status: "NotReady".to_string(),
                                code: 1,
                                formatted_message: FormattedMessage {
                                    lang: "en-US".to_string(),
                                    message: format!("Failed to install extension {}: {}", plugin_def.name, e),
                                },
                            });
                        }
                    }
                } else {
                    // Extension is disabled
                    handler_statuses.push(HandlerStatus {
                        handler_name: plugin_def.name.clone(),
                        handler_version: plugin_def.version.clone(),
                        status: "NotReady".to_string(),
                        code: 0,
                        formatted_message: FormattedMessage {
                            lang: "en-US".to_string(),
                            message: format!("Extension {} is disabled", plugin_def.name),
                        },
                    });
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to parse extensions configuration: {}", e);
            eprintln!("XML content that failed to parse:\n{}", extensions_xml);
        }
    }

    println!("=== Extension processing complete, {} handlers processed ===", handler_statuses.len());
    Ok(handler_statuses)
}

/// Extract extension ZIP URL from manifest XML
pub fn extract_extension_zip_url(manifest_xml: &str) -> Result<String> {
    // Simple regex-based extraction since the manifest format is straightforward
    let uri_pattern = regex::Regex::new(r#"<Uri>([^<]+)</Uri>"#)?;
    
    if let Some(captures) = uri_pattern.captures(manifest_xml) {
        if let Some(uri_match) = captures.get(1) {
            return Ok(uri_match.as_str().to_string());
        }
    }
    
    Err("Failed to extract extension ZIP URL from manifest".into())
}
