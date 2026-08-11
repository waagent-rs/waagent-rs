use super::types::Result;
use base64::prelude::*;
use crate::crypto::decryption::{decrypt_protected_settings_optimized, decrypt_protected_settings};
use crate::crypto::certificates::find_local_azure_certificates;
use std::process::Command;

/// Execute a RunCommand from Azure configuration
pub async fn execute_run_command<T>(
    config_json: &str, 
    _client: &reqwest::Client, 
    _goal_state: &T
) -> Result<String> 
where
    T: std::fmt::Debug,
{
    println!("Parsing RunCommand configuration...");
    
    // Parse the JSON configuration
    let config: serde_json::Value = serde_json::from_str(config_json)?;
    println!("Parsed RunCommand JSON structure");
    
    let mut command = String::new();
    let mut found_protected_settings = false;
    
    if let Some(runtime_settings) = config.get("runtimeSettings") {
        if let Some(settings_array) = runtime_settings.as_array() {
            if let Some(first_setting) = settings_array.first() {
                if let Some(handler_settings) = first_setting.get("handlerSettings") {
                    
                    // Check for protected settings first
                    if let Some(protected_settings) = handler_settings.get("protectedSettings") {
                        if let Some(cert_thumbprint) = handler_settings.get("protectedSettingsCertThumbprint") {
                            println!("Found protectedSettings with certificate thumbprint");
                            found_protected_settings = true;
                            
                            let thumbprint = cert_thumbprint.as_str().unwrap_or("");
                            let protected_data = protected_settings.as_str().unwrap_or("");
                            
                            println!("Attempting to decrypt protected settings...");
                            
                            // Try optimized decryption with direct certificate access first
                            match decrypt_protected_settings_optimized(protected_data, thumbprint).await {
                                Ok(decrypted_json) => {
                                    println!("Successfully decrypted protected settings!");
                                    println!("Decrypted protected settings payload successfully");
                                    
                                    // Try to extract command, but don't fail if protectedSettings is empty
                                    match extract_command_from_json(&decrypted_json) {
                                        Ok(cmd) => {
                                            command = cmd;
                                            println!("Found command in protectedSettings");
                                        }
                                        Err(_) => {
                                            println!("No command found in protectedSettings (empty or missing fields)");
                                            // Will check publicSettings next
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("Warning: Decryption failed: {}", e);
                                    println!("   Trying fallback with wireserver certificates...");
                                    
                                    // Fallback: try fetching certificates from local store
                                    match find_local_azure_certificates() {
                                        Ok(certificates) => {
                                            match decrypt_protected_settings(protected_data, thumbprint, &certificates).await {
                                                Ok(decrypted_json) => {
                                                    println!("Successfully decrypted with local certificates!");
                                                    match extract_command_from_json(&decrypted_json) {
                                                        Ok(cmd) => {
                                                            command = cmd;
                                                            println!("Found command in protectedSettings");
                                                        }
                                                        Err(_) => {
                                                            println!("No command found in protectedSettings (empty or missing fields)");
                                                        }
                                                    }
                                                }
                                                Err(fallback_e) => {
                                                    println!("Warning: Local fallback also failed: {}", fallback_e);
                                                    // Don't return error - will try publicSettings
                                                }
                                            }
                                        }
                                        Err(cert_e) => {
                                            println!("Warning: Certificate fetch fallback failed: {}", cert_e);
                                            // Don't return error - will try publicSettings
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Check public settings as fallback
                    if command.is_empty() {
                        if let Some(public_settings) = handler_settings.get("publicSettings") {
                            println!("Checking publicSettings for command");
                            
                            // publicSettings can be either a JSON object or a JSON string
                            let public_json: serde_json::Value = if public_settings.is_string() {
                                // Parse the JSON string
                                match serde_json::from_str(public_settings.as_str().unwrap_or("{}")) {
                                    Ok(parsed) => parsed,
                                    Err(e) => {
                                        println!("Failed to parse publicSettings JSON string: {}", e);
                                        serde_json::Value::Object(serde_json::Map::new())
                                    }
                                }
                            } else {
                                public_settings.clone()
                            };
                            
                            // Try direct commandToExecute
                            if let Some(cmd) = public_json.get("commandToExecute") {
                                command = cmd.as_str().unwrap_or("").to_string();
                                println!("Found command in publicSettings.commandToExecute: {}", command);
                            } 
                            // Try source.script (common for RunCommand)
                            else if let Some(source) = public_json.get("source") {
                                if let Some(script) = source.get("script") {
                                    if let Some(script_str) = script.as_str() {
                                        command = script_str.to_string();
                                        println!("Found command in publicSettings.source.script: {}", command);
                                    }
                                }
                            }
                            // Try script field directly
                            else if let Some(script) = public_json.get("script") {
                                if let Some(script_str) = script.as_str() {
                                    command = script_str.to_string();
                                    println!("Found command in publicSettings.script: {}", command);
                                }
                            }
                            else {
                                println!("No 'commandToExecute', 'source.script', or 'script' found in publicSettings");
                                println!("publicSettings content: {}",
                                        serde_json::to_string_pretty(&public_json).unwrap_or_default());
                            }
                        }
                    }
                }
            }
        }
    }

    if command.is_empty() {
        if found_protected_settings {
            return Err("Command is encrypted in protectedSettings - decryption attempted but failed".into());
        } else {
            return Err("No command found in RunCommand configuration".into());
        }
    }

    println!("Executing command: {}", command);

    // Execute the command using bash
    let output = Command::new("bash")
        .arg("-c")
        .arg(command)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    let result = if output.status.success() {
        format!("Command executed successfully.\nSTDOUT:\n{}\nSTDERR:\n{}", stdout, stderr)
    } else {
        format!("Command failed with exit code: {:?}\nSTDOUT:\n{}\nSTDERR:\n{}",
                output.status.code(), stdout, stderr)
    };

    println!("Command result: {}", result);
    Ok(result)
}

/// Extract command from decrypted JSON
fn extract_command_from_json(decrypted_json: &str) -> Result<String> {
    if let Ok(decrypted_config) = serde_json::from_str::<serde_json::Value>(decrypted_json) {
        // Try commandToExecute first (standard field)
        if let Some(cmd) = decrypted_config.get("commandToExecute") {
            let command = cmd.as_str().unwrap_or("").to_string();
            println!("Extracted command from decrypted settings: {}", command);
            return Ok(command);
        }
        // Try script field (base64-encoded command)
        else if let Some(script) = decrypted_config.get("script") {
            if let Some(script_str) = script.as_str() {
                println!("Found base64-encoded script: {}", script_str);
                // Decode base64
                match BASE64_STANDARD.decode(script_str) {
                    Ok(decoded_bytes) => {
                        match String::from_utf8(decoded_bytes) {
                            Ok(decoded_command) => {
                                let command = decoded_command.trim().to_string();
                                println!("Decoded script command: {}", command);
                                return Ok(command);
                            }
                            Err(e) => {
                                return Err(format!("Failed to decode base64 script as UTF-8: {}", e).into());
                            }
                        }
                    }
                    Err(e) => {
                        return Err(format!("Failed to decode base64 script: {}", e).into());
                    }
                }
            }
        }
        else {
            println!("No 'commandToExecute' or 'script' field found in decrypted JSON");
            if cfg!(debug_assertions) {
                println!("Available fields: {:?}", 
                    decrypted_config.as_object().map(|o| o.keys().collect::<Vec<_>>()));
            }
        }
    }
    
    Err("Failed to extract command from decrypted JSON".into())
}
