use super::certificates::{find_additional_private_keys, find_local_azure_certificates, get_azure_certificate_by_thumbprint};
use crate::types::{Certificate, Result};
use base64::prelude::*;
use std::io::Write;
use std::process::{Command, Stdio};

/// Optimized decrypt function that tries direct certificate access first
pub async fn decrypt_protected_settings_optimized(
    protected_settings_b64: &str, 
    cert_thumbprint: &str,
) -> Result<String> {
    if cfg!(debug_assertions) {
        println!("Attempting to decrypt protected settings...");
        println!("Looking for certificate with thumbprint: {}", cert_thumbprint);
    }

    // Try direct certificate access first
    if let Ok(cert_data) = get_azure_certificate_by_thumbprint(cert_thumbprint) {
        if cfg!(debug_assertions) {
            println!("Found matching certificate: {}", cert_data.name);
        }
        return decrypt_with_certificate(protected_settings_b64, &cert_data).await;
    }

    // Fall back to fetching all certificates and searching
    if cfg!(debug_assertions) {
        println!("Direct lookup failed, falling back to certificate list search...");
    }
    match find_local_azure_certificates() {
        Ok(certificates) => {
            if let Some(cert_data) = certificates.iter().find(|c| c.thumbprint.eq_ignore_ascii_case(cert_thumbprint)) {
                if cfg!(debug_assertions) {
                    println!("Found matching certificate in fallback search: {}", cert_data.name);
                }
                return decrypt_with_certificate(protected_settings_b64, cert_data).await;
            }
        }
        Err(e) => {
            if cfg!(debug_assertions) {
                println!("Failed to load local certificates: {}", e);
            }
        }
    }

    Err(format!("Certificate with thumbprint {} not found", cert_thumbprint).into())
}

/// Helper function to perform the actual decryption with a certificate
pub async fn decrypt_with_certificate(
    protected_settings_b64: &str,
    cert_data: &Certificate,
) -> Result<String> {
    if cfg!(debug_assertions) {
        println!("Certificate lookup attempted: {} (not used by working decryption)", cert_data.name);
    }

    // Decode the protected settings from base64
    let encrypted_data = BASE64_STANDARD.decode(protected_settings_b64)
        .map_err(|e| format!("Failed to decode protected settings: {}", e))?;
    
    if cfg!(debug_assertions) {
        println!("Encrypted data length: {} bytes", encrypted_data.len());
        println!("Trying OpenSSL CMS decrypt on encrypted data...");
    }

    // Use the working OpenSSL CMS approach directly
    if let Some(cms_result) = try_openssl_cms_on_original_data(&encrypted_data) {
        if cfg!(debug_assertions) {
            println!("OpenSSL CMS decrypt successful!");
        }
        return Ok(cms_result);
    }

    Err("All decryption methods failed".into())
}

/// Legacy decrypt function (for backward compatibility)
pub async fn decrypt_protected_settings(
    protected_settings_b64: &str, 
    cert_thumbprint: &str,
    certificates: &[Certificate]
) -> Result<String> {
    if cfg!(debug_assertions) {
        println!("Attempting to decrypt protected settings (legacy function)...");
        println!("Looking for certificate with thumbprint: {}", cert_thumbprint);
    }

    // Find the certificate with matching thumbprint
    let cert_data = certificates
        .iter()
        .find(|c| c.thumbprint.eq_ignore_ascii_case(cert_thumbprint))
        .ok_or_else(|| format!("Certificate with thumbprint {} not found", cert_thumbprint))?;
    
    if cfg!(debug_assertions) {
        println!("Found matching certificate: {}", cert_data.name);
    }

    // Decode the protected settings from base64
    let encrypted_data = BASE64_STANDARD.decode(protected_settings_b64)
        .map_err(|e| format!("Failed to decode protected settings: {}", e))?;
    
    if cfg!(debug_assertions) {
        println!("Encrypted data length: {} bytes", encrypted_data.len());
    }

    // Use the working OpenSSL CMS approach
    if let Some(cms_result) = try_openssl_cms_on_original_data(&encrypted_data) {
        if cfg!(debug_assertions) {
            println!("OpenSSL CMS decrypt successful!");
        }
        return Ok(cms_result);
    }

    Err("Decryption failed - no working method succeeded".into())
}

/// Try OpenSSL CMS decryption on encrypted data
pub fn try_openssl_cms_on_original_data(encrypted_data: &[u8]) -> Option<String> {
    println!("Attempting OpenSSL CMS decrypt on {} bytes of encrypted data", encrypted_data.len());
    
    // Try multiple key/certificate combinations
    let mut key_cert_combinations = vec![
        // Primary transport key/cert (waagent-rs independent paths)
        ("/var/lib/waagent-rs/TransportPrivate.pem".to_string(), "/var/lib/waagent-rs/TransportCert.pem".to_string()),
        // Try without explicit recipient certificate
        ("/var/lib/waagent-rs/TransportPrivate.pem".to_string(), "".to_string()),
    ];
    
    // Look for all .prv files (private keys) in /var/lib/waagent-rs/
    // These are saved with thumbprint as filename
    println!("Scanning /var/lib/waagent-rs/ for .prv files...");
    if let Ok(entries) = std::fs::read_dir("/var/lib/waagent-rs/") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(extension) = path.extension() {
                if extension == "prv" {
                    // Found a private key file, try it with its matching .crt
                    if let Some(stem) = path.file_stem() {
                        let thumbprint = stem.to_string_lossy();
                        let key_path = format!("/var/lib/waagent-rs/{}.prv", thumbprint);
                        let cert_path = format!("/var/lib/waagent-rs/{}.crt", thumbprint);
                        
                        println!("Found private key for thumbprint: {}", thumbprint);
                        
                        key_cert_combinations.push((key_path.clone(), cert_path.clone()));
                        key_cert_combinations.push((key_path.clone(), "".to_string()));
                        println!("  Added key: {} with cert: {}", key_path, cert_path);
                    }
                }
            }
        }
    } else {
        println!("Failed to read /var/lib/waagent-rs/ directory");
    }
    
    // Look for additional private keys from legacy method
    let additional_keys = find_additional_private_keys();
    for key_path in additional_keys {
        key_cert_combinations.push((key_path.clone(), "".to_string()));
    }
    
    println!("Total key/cert combinations to try: {}", key_cert_combinations.len());
    
    for (private_key_path, cert_path) in &key_cert_combinations {
        println!("Trying key: {}, cert: {}", private_key_path, cert_path);
        
        if !std::path::Path::new(private_key_path).exists() {
            println!("  Private key not found at {}", private_key_path);
            continue;
        }
        
        if !cert_path.is_empty() && !std::path::Path::new(cert_path).exists() {
            println!("  Certificate not found at {}", cert_path);
            continue;
        }
        
        println!("  Files exist, trying CMS decrypt with key: {} and cert: {}", private_key_path,
                if cert_path.is_empty() { "none" } else { cert_path });
        
        // Build OpenSSL CMS command
        let mut args = vec!["cms", "-decrypt", "-inform", "DER", "-inkey", private_key_path];
        
        if !cert_path.is_empty() {
            args.extend_from_slice(&["-recip", cert_path]);
        }
        
        let cmd = Command::new("openssl")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        
        match cmd {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.take() {
                    let mut stdin = stdin;
                    if let Err(e) = stdin.write_all(encrypted_data) {
                        if cfg!(debug_assertions) {
                            println!("Failed to write encrypted data to OpenSSL stdin: {}", e);
                        }
                        continue;
                    }
                    drop(stdin);
                }
                
                match child.wait_with_output() {
                    Ok(output) => {
                        if output.status.success() {
                            if cfg!(debug_assertions) {
                                println!("OpenSSL CMS decrypt successful with key: {} and cert: {}!",
                                        private_key_path, if cert_path.is_empty() { "none" } else { cert_path });
                            }
                            
                            let cms_output = output.stdout;
                            if cfg!(debug_assertions) {
                                println!("CMS output length: {} bytes", cms_output.len());
                                
                                if !cms_output.is_empty() {
                                    let preview = &cms_output[..std::cmp::min(32, cms_output.len())];
                                    println!("CMS output preview (hex): {}",
                                            preview.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
                                }
                            }
                            
                            // Try UTF-8 FIRST (most common for JSON)
                            if let Ok(utf8_str) = String::from_utf8(cms_output.clone()) {
                                let cleaned = utf8_str.trim_matches('\0').trim();
                                if !cleaned.is_empty() && cleaned.len() > 5 {
                                    if cfg!(debug_assertions) {
                                        println!("CMS output as UTF-8: {:?}", cleaned);
                                    }
                                    if cleaned.starts_with('{') || cleaned.contains("commandToExecute") || cleaned.contains("script") {
                                        if cfg!(debug_assertions) {
                                            println!("FOUND JSON IN CMS UTF-8 OUTPUT!");
                                        }
                                        return Some(cleaned.to_string());
                                    }
                                    // Return any meaningful UTF-8 content
                                    if cleaned.len() > 10 {
                                        return Some(cleaned.to_string());
                                    }
                                }
                            }
                            
                            // Fallback: Try finding JSON in binary output
                            let output_str = String::from_utf8_lossy(&cms_output);
                            if output_str.contains('{') || output_str.contains("commandToExecute") {
                                if cfg!(debug_assertions) {
                                    println!("Found JSON markers in binary output");
                                }
                                return Some(output_str.to_string());
                            }
                        } else if cfg!(debug_assertions) {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            println!("OpenSSL CMS decrypt failed: {}", stderr);
                        }
                    }
                    Err(e) => {
                        if cfg!(debug_assertions) {
                            println!("Failed to wait for OpenSSL process: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                if cfg!(debug_assertions) {
                    println!("Failed to spawn OpenSSL process: {}", e);
                }
            }
        }
    }
    
    None
}

/// Helper function to write files with sudo if needed
pub async fn write_file_with_sudo(file_path: &str, content: &[u8]) -> Result<()> {
    // Try to write normally first
    if std::fs::write(file_path, content).is_ok() {
        return Ok(());
    }
    
    // If that fails, use sudo to write the file
    if cfg!(debug_assertions) {
        println!("Writing file with elevated privileges: {}", file_path);
    }
    
    // Create a temporary file with the content
    let temp_file = format!("/tmp/waagent_temp_{}", std::process::id());
    std::fs::write(&temp_file, content)?;
    
    // Use sudo to copy the temp file to the target location
    let output = Command::new("sudo")
        .args(["cp", &temp_file, file_path])
        .output()?;
    
    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to write file {}: {}", file_path, stderr).into());
    }
    
    Ok(())
}
