use super::types::*;
use super::processor::extract_extension_zip_url;
use crate::crypto::decryption::write_file_with_sudo;
use reqwest::Client;
use std::process::Command;
use std::time::Duration;

/// Download and install an extension
pub async fn download_and_install_extension(
    client: &Client, 
    plugin_def: &PluginDefinition, 
    plugin_config: &str
) -> Result<()> {
    // Create extension directory using sudo if needed
    let extension_dir = format!("/var/lib/waagent/{}", plugin_def.name);
    println!("Creating extension directory: {}", extension_dir);
    
    // Try to create directory, use sudo if permission denied
    let mkdir_result = std::fs::create_dir_all(&extension_dir);
    if mkdir_result.is_err() {
        println!("Creating directory with elevated privileges...");
        let output = Command::new("sudo")
            .args(["mkdir", "-p", &extension_dir])
            .output()?;
            
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to create extension directory {}: {}", extension_dir, stderr).into());
        }
        println!("Extension directory created successfully with sudo");
    } else {
        println!("Extension directory created successfully");
    }

    // Save extension configuration
    if !plugin_config.is_empty() {
        let config_path = format!("{}/config.json", extension_dir);
        write_file_with_sudo(&config_path, plugin_config.as_bytes()).await?;
        println!("Saved extension configuration to: {}", config_path);
    }

    // Download extension package if location is provided
    if !plugin_def.location.is_empty() {
        println!("Downloading extension manifest from: {}", plugin_def.location);
        
        // First, download the manifest XML
        let manifest_response = client
            .get(&plugin_def.location)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        let manifest_xml = manifest_response.text().await?;
        println!("Downloaded manifest XML ({} bytes)", manifest_xml.len());
        
        // Parse the manifest to get the actual extension ZIP URL
        let zip_url = extract_extension_zip_url(&manifest_xml)?;
        println!("Found extension ZIP URL: {}", zip_url);
        
        // Now download the actual extension ZIP
        let zip_response = client
            .get(&zip_url)
            .timeout(Duration::from_secs(60))
            .send()
            .await?;

        let extension_zip = zip_response.bytes().await?;
        println!("Downloaded extension ZIP ({} bytes)", extension_zip.len());
        let zip_path = format!("{}/extension.zip", extension_dir);
        write_file_with_sudo(&zip_path, &extension_zip).await?;

        // Extract extension using sudo if needed
        println!("Extracting extension to: {}", extension_dir);
        let unzip_output = Command::new("unzip")
            .args(["-o", &zip_path, "-d", &extension_dir])
            .output();

        let extract_result = match unzip_output {
            Ok(output) if output.status.success() => Ok(output),
            _ => {
                // Try with sudo
                println!("Extracting with elevated privileges...");
                Command::new("sudo")
                    .args(["unzip", "-o", &zip_path, "-d", &extension_dir])
                    .output()
            }
        };

        if let Ok(output) = extract_result {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to extract extension: {}", stderr).into());
            }
        } else {
            return Err("Failed to extract extension even with sudo".into());
        }

        // Make extension executable
        let extension_script = format!("{}/enable.py", extension_dir);
        if std::path::Path::new(&extension_script).exists() {
            let chmod_result = Command::new("chmod")
                .args(["+x", &extension_script])
                .output();
                
            if chmod_result.is_err() || !chmod_result.as_ref().unwrap().status.success() {
                // Try with sudo
                println!("Setting executable permissions with sudo...");
                Command::new("sudo")
                    .args(["chmod", "+x", &extension_script])
                    .output()?;
            }
        }
    } else {
        println!("No download location provided for extension {}", plugin_def.name);
    }

    println!("Extension {} processed successfully", plugin_def.name);
    Ok(())
}
