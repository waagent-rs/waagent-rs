use std::path::PathBuf;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Native Rust implementation of run-command extension functionality
pub async fn process_run_command_native(
    _client: &reqwest::Client,
    extension_name: &str,
    seq_no: u64,
    runtime_settings: &serde_json::Value,
    ext: &serde_json::Value,
) -> Result<serde_json::Value> {
    let version = ext.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0");
    
    println!("[RUNCOMMAND] Processing extension '{}' (seq {}) with native Rust handler", 
             extension_name, seq_no);
    
    // Create work directory for this extension and sequence
    let work_dir = PathBuf::from("/var/lib/waagent-rs/run-command")
        .join(extension_name)
        .join(seq_no.to_string());
    std::fs::create_dir_all(&work_dir)?;
    
    // Check if this sequence number was already processed
    let mrseq_file = PathBuf::from("/var/lib/waagent-rs/run-command")
        .join(extension_name)
        .join("mrseq");
    
    if let Ok(prev_seq) = std::fs::read_to_string(&mrseq_file) {
        if let Ok(prev_seq_num) = prev_seq.trim().parse::<u64>() {
            if seq_no <= prev_seq_num {
                println!("[RUNCOMMAND] Sequence {} already processed (mrseq={}), skipping", seq_no, prev_seq_num);
                // Return success status for already-processed sequence
                return Ok(serde_json::json!({
                    "handlerVersion": version,
                    "handlerName": extension_name,
                    "status": "success",
                    "code": 0,
                    "sequenceNumber": seq_no,
                    "formattedMessage": {
                        "lang": "en-US",
                        "message": "Already processed"
                    }
                }));
            }
        }
    }
    
    // Parse the extension settings
    let public_settings = runtime_settings.get("publicSettings")
        .and_then(|ps| {
            // publicSettings is a JSON string, parse it
            if let Some(ps_str) = ps.as_str() {
                serde_json::from_str::<serde_json::Value>(ps_str).ok()
            } else {
                Some(ps.clone())
            }
        });
    let protected_settings = runtime_settings.get("protectedSettings");
    
    // Get command to execute
    // Check for source.script first (new format), then commandToExecute (legacy format)
    let command = public_settings.as_ref()
        .and_then(|s| s.get("source"))
        .and_then(|src| src.get("script"))
        .and_then(|c| c.as_str())
        .or_else(|| {
            // Legacy format: commandToExecute at root level
            protected_settings
                .and_then(|s| s.get("commandToExecute"))
                .and_then(|c| c.as_str())
        })
        .or_else(|| {
            public_settings.as_ref()
                .and_then(|s| s.get("commandToExecute"))
                .and_then(|c| c.as_str())
        })
        .or_else(|| {
            // Also check for 'script' field at root
            protected_settings
                .and_then(|s| s.get("script"))
                .and_then(|c| c.as_str())
        })
        .or_else(|| {
            public_settings.as_ref()
                .and_then(|s| s.get("script"))
                .and_then(|c| c.as_str())
        })
        .ok_or("No commandToExecute or script specified")?;
    
    println!("[RUNCOMMAND] Executing command: {}", command);
    
    // Create stdout and stderr files
    let stdout_path = work_dir.join("stdout");
    let stderr_path = work_dir.join("stderr");
    let stdout_file = std::fs::File::create(&stdout_path)?;
    let stderr_file = std::fs::File::create(&stderr_path)?;
    
    // Execute the command using /bin/sh
    let output = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .current_dir(&work_dir)
        .stdout(stdout_file)
        .stderr(stderr_file)
        .output();
    
    let (success, exit_code, message) = match output {
        Ok(output) => {
            let code = output.status.code().unwrap_or(1);
            let success = output.status.success();
            
            // Read stdout and stderr for status message
            let stdout = std::fs::read_to_string(&stdout_path)
                .unwrap_or_default();
            let stderr = std::fs::read_to_string(&stderr_path)
                .unwrap_or_default();
            
            // Truncate output to 4KB for status message
            let stdout_trunc = if stdout.len() > 4096 {
                format!("{}... (truncated)", &stdout[..4096])
            } else {
                stdout
            };
            
            let stderr_trunc = if stderr.len() > 4096 {
                format!("{}... (truncated)", &stderr[..4096])
            } else {
                stderr
            };
            
            let message = format!("\n[stdout]\n{}\n[stderr]\n{}", stdout_trunc, stderr_trunc);
            
            println!("[RUNCOMMAND] Command completed with exit code: {}", code);
            (success, code, message)
        }
        Err(e) => {
            eprintln!("[RUNCOMMAND] Failed to execute command: {}", e);
            (false, 1, format!("Failed to execute command: {}", e))
        }
    };
    
    // Save the sequence number to mrseq
    std::fs::create_dir_all(mrseq_file.parent().unwrap())?;
    std::fs::write(&mrseq_file, seq_no.to_string())?;
    println!("[RUNCOMMAND] Saved mrseq: {}", seq_no);
    
    // Build status response
    let ext_status = serde_json::json!({
        "handlerVersion": version,
        "handlerName": extension_name,
        "status": if success { "success" } else { "error" },
        "code": exit_code,
        "sequenceNumber": seq_no,
        "formattedMessage": {
            "lang": "en-US",
            "message": message
        }
    });
    
    Ok(ext_status)
}
