use reqwest::Client;
use crate::extensions::process_run_command_native;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Process extensions from vmSettings endpoint
/// Returns updated extension statuses
pub async fn process_vmextensions_from_vmsettings(
    client: &Client,
    vm_settings: &serde_json::Value,
) -> Result<Vec<serde_json::Value>> {
    let mut all_extension_statuses: Vec<serde_json::Value> = Vec::new();
    
    if let Some(obj) = vm_settings.as_object() {
        println!("vmSettings keys: {:?}", obj.keys().collect::<Vec<_>>());
    }
    
    // Get statusUploadBlob URL for reporting
    let status_upload_blob = vm_settings.get("statusUploadBlob")
        .and_then(|v| v.get("value"))  // Azure uses "value" field
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    if !status_upload_blob.is_empty() {
        println!("[STATUS_BLOB_URL] {}", status_upload_blob);
    }
    
    if let Some(extensions) = vm_settings.get("extensionGoalStates").and_then(|v| v.as_array()) {
        println!("[EXTENSIONS] Received {} extension(s) from Azure", extensions.len());
        
        for ext in extensions {
            if let Some(ext_name) = ext.get("name").and_then(|v| v.as_str()) {
                println!("[EXTENSION] Detected: {} (state: {})", 
                    ext_name,
                    ext.get("state").and_then(|v| v.as_str()).unwrap_or("unknown"));
                
                // Debug: Print extension keys to understand structure
                if let Some(obj) = ext.as_object() {
                    println!("[DEBUG] Extension '{}' keys: {:?}", ext_name, obj.keys().collect::<Vec<_>>());
                }
                
                // Check for RunCommand extension
                if ext_name.contains("RunCommand") && ext.get("state").and_then(|v| v.as_str()) == Some("enabled") {
                    println!("[RUNCOMMAND] Processing RunCommand extension: {}", ext_name);
                    
                    // Check if this is a multi-config extension
                    let is_multi_config = ext.get("isMultiConfig")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    
                    if is_multi_config {
                        println!("[RUNCOMMAND] Multi-config extension detected - will process each run-command separately");
                    }
                    
                    // vmSettings format has settings as an array
                    if let Some(settings_array) = ext.get("settings").and_then(|v| v.as_array()) {
                        // For multi-config, process ALL settings; for single-config, just the first
                        let settings_to_process: Vec<&serde_json::Value> = if is_multi_config {
                            settings_array.iter().collect()
                        } else {
                            settings_array.first().map(|s| vec![s]).unwrap_or_default()
                        };
                        
                        for setting in settings_to_process {
                            // Get seqNo from the setting
                            let seq_no = setting.get("seqNo")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                            
                            // For multi-config, use extensionName from the setting; otherwise use parent extension name
                            let extension_name = if is_multi_config {
                                setting.get("extensionName")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(ext_name)
                            } else {
                                ext_name
                            };
                            
                            // Check if there's a per-extension statusUploadBlob
                            let ext_status_blob = setting.get("statusUploadBlob")
                                .and_then(|v| v.as_str())
                                .or_else(|| ext.get("statusUploadBlob").and_then(|v| v.as_str()))
                                .unwrap_or(status_upload_blob);
                            
                            if ext_status_blob != status_upload_blob {
                                println!("[DEBUG] Extension '{}' has its own status blob (different from global)", extension_name);
                            }
                            
                            // Check extension state for multi-config
                            if is_multi_config {
                                let extension_state = setting.get("extensionState")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("enabled");
                                
                                if extension_state != "enabled" {
                                    println!("[RUNCOMMAND] Skipping '{}' - state is '{}'", extension_name, extension_state);
                                    continue;
                                }
                            }
                            
                            println!("[RUNCOMMAND] Processing '{}' with sequence number {}", extension_name, seq_no);
                            println!("[RUNCOMMAND] Executing extension '{}' seqNo {} with native Rust handler", extension_name, seq_no);
                            
                            let status_file = format!("/var/lib/waagent-rs/extension_status_{}_{}.json", 
                                                     extension_name, seq_no);
                                
                            // Use native Rust execution instead of external Go handler
                            let execution_result = match process_run_command_native(
                                client,
                                extension_name,
                                seq_no,
                                setting,
                                ext
                            ).await {
                                Ok(ext_status) => {
                                    println!("[RUNCOMMAND] Native handler execution completed successfully for '{}'", extension_name);
                                    
                                    // Remove status file on success (no need to persist)
                                    if std::path::Path::new(&status_file).exists() {
                                        let _ = std::fs::remove_file(&status_file);
                                        println!("[STATUS] Removed status file for successful execution: {}", status_file);
                                    }
                                    
                                    all_extension_statuses.push(ext_status);
                                    println!("[STATUS] Collected status for '{}' (just executed)", extension_name);
                                    continue;
                                },
                                Err(e) => {
                                    eprintln!("[HANDLER] Handler execution failed for '{}': {}", extension_name, e);
                                    ("error", format!("Handler execution failed: {}", e))
                                }
                            };
                                
                            // Only reach here if handler execution failed - create error status
                            let truncated_output: String = execution_result.1.chars().take(2000).collect();
                            let ext_status = serde_json::json!({
                                "handlerVersion": "1.0",
                                "handlerName": extension_name,
                                "status": execution_result.0,
                                "code": 1,
                                "sequenceNumber": seq_no,
                                "formattedMessage": {
                                    "lang": "en-US",
                                    "message": truncated_output
                                }
                            });
                            
                            // Keep error status file for debugging
                            if let Err(e) = std::fs::write(&status_file, serde_json::to_string_pretty(&ext_status).unwrap_or_default()) {
                                eprintln!("[ERROR] Failed to write local status file: {}", e);
                            } else {
                                println!("[STATUS] Saved error status locally: {}", status_file);
                            }
                            
                            all_extension_statuses.push(ext_status);
                            println!("[STATUS] Collected error status for '{}'", extension_name);
                        }
                    }
                }
            }
        }
        
        // Extension statuses will be included in the status report
        if !all_extension_statuses.is_empty() {
            println!("[STATUS] Collected {} extension status(es) for status report", all_extension_statuses.len());
        }
    } else {
        println!("[EXTENSIONS] No extensions found in vmSettings");
    }
    
    Ok(all_extension_statuses)
}
