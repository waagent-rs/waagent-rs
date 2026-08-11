use crate::types::{Certificate, Result};
use base64::prelude::*;
use openssl::pkcs12::Pkcs12;
use openssl::hash::MessageDigest;
use std::fs;

/// Find all local Azure certificates in common locations
pub fn find_local_azure_certificates() -> Result<Vec<Certificate>> {
    let mut certificates = Vec::new();
    
    if cfg!(debug_assertions) {
        println!("Searching for certificates in local Azure certificate store...");
    }
    
    // Common Azure certificate locations
    let cert_locations = [
        "/var/lib/waagent-rs/",
        "/var/lib/waagent-rs/certs/",
        "/etc/ssl/certs/",
        "/usr/local/share/ca-certificates/",
    ];
    
    for location in &cert_locations {
        if cfg!(debug_assertions) {
            println!("Scanning directory: {}", location);
        }
        
        if let Ok(entries) = fs::read_dir(location) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy();
                    
                    // Look for PKCS#12 certificate files only
                    if filename_str.ends_with(".pfx") || filename_str.ends_with(".p12") {
                        if cfg!(debug_assertions) {
                            println!("Found PKCS#12 certificate file: {}", path.display());
                        }
                        
                        if let Ok(cert_data) = fs::read(&path) {
                            match extract_pfx_certificates(&cert_data) {
                                Ok(pfx_certs) => {
                                    if !pfx_certs.is_empty() {
                                        if cfg!(debug_assertions) {
                                            println!("Parsed PKCS#12 certificate: {}, found {} certs", filename_str, pfx_certs.len());
                                        }
                                        certificates.extend(pfx_certs);
                                    }
                                }
                                Err(e) => {
                                    if cfg!(debug_assertions) {
                                        println!("Failed to parse PKCS#12 certificate {}: {}", filename_str, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    if cfg!(debug_assertions) {
        println!("Local certificate scan complete: found {} certificates", certificates.len());
    }
    Ok(certificates)
}

/// Get Azure certificate by thumbprint
pub fn get_azure_certificate_by_thumbprint(thumbprint: &str) -> Result<Certificate> {
    println!("Looking for certificate with thumbprint: {}", thumbprint);
    
    // Check for .crt/.prv file pairs first (new format from waagent-rs)
    let cert_path = format!("/var/lib/waagent-rs/{}.crt", thumbprint);
    let key_path = format!("/var/lib/waagent-rs/{}.prv", thumbprint);
    
    if std::path::Path::new(&cert_path).exists() && std::path::Path::new(&key_path).exists() {
        println!("Found certificate files: {} and {}", cert_path, key_path);
        // Return a minimal Certificate object - the actual decryption uses the file paths directly
        return Ok(Certificate {
            name: format!("{}.crt", thumbprint),
            certificate_data_format: "Pfx".to_string(),
            thumbprint: thumbprint.to_uppercase(),
            data: String::new(), // Not used for .crt/.prv based decryption
            format: "Pkcs12".to_string(),
            store: "My".to_string(),
        });
    }
    
    println!("Certificate files not found at {} or {}", cert_path, key_path);
    
    // Fall back to old PKCS#12 approach (not needed for waagent-rs but kept for compatibility)
    Err(format!("Certificate with thumbprint {} not found in /var/lib/waagent-rs/", thumbprint).into())
}

/// Extract certificates from PFX/PKCS#12 data
pub fn extract_pfx_certificates(data: &[u8]) -> Result<Vec<Certificate>> {
    let mut certificates = Vec::new();
    
    // Try to parse as PKCS#12 without password first
    for password in &["", "password", "azure"] {
        match Pkcs12::from_der(data) {
            Ok(pkcs12) => {
                match pkcs12.parse2(password) {
                    Ok(parsed) => {
                        if cfg!(debug_assertions) {
                            println!("Successfully parsed PKCS#12 with password: '{}'", password);
                        }
                        
                        // Extract the main certificate
                        if let Some(cert) = &parsed.cert {
                            match extract_certificate_info(cert, 0) {
                                Ok(cert_info) => {
                                    if cfg!(debug_assertions) {
                                        println!("Extracted main certificate: thumbprint={}", cert_info.thumbprint);
                                    }
                                    certificates.push(cert_info);
                                }
                                Err(e) => {
                                    if cfg!(debug_assertions) {
                                        println!("Failed to process main certificate: {}", e);
                                    }
                                }
                            }
                        }
                        
                        // Extract additional certificates from the chain
                        if let Some(chain) = &parsed.ca {
                            for (i, cert) in chain.iter().enumerate() {
                                match extract_certificate_info_ref(cert, i + 1) {
                                    Ok(cert_info) => {
                                        if cfg!(debug_assertions) {
                                            println!("Extracted chain certificate {}: thumbprint={}", i + 1, cert_info.thumbprint);
                                        }
                                        certificates.push(cert_info);
                                    }
                                    Err(e) => {
                                        if cfg!(debug_assertions) {
                                            println!("Failed to process chain certificate {}: {}", i + 1, e);
                                        }
                                    }
                                }
                            }
                        }
                        
                        break; // Successfully parsed, no need to try other passwords
                    }
                    Err(_) => {
                        // Try next password
                        continue;
                    }
                }
            }
            Err(_) => {
                // Not PKCS#12 format
                break;
            }
        }
    }
    
    Ok(certificates)
}

/// Extract certificate information from an X509 certificate reference
fn extract_certificate_info_from_x509_ref(x509_cert: &openssl::x509::X509Ref, index: usize) -> Result<Certificate> {
    // Calculate SHA-1 thumbprint
    let thumbprint = x509_cert.digest(MessageDigest::sha1())
        .map_err(|e| format!("Failed to calculate thumbprint: {}", e))?;
    
    let thumbprint_hex = thumbprint.iter()
        .fold(String::new(), |mut acc, b| {
            use std::fmt::Write;
            write!(&mut acc, "{:02X}", b).unwrap();
            acc
        });
    
    // Get certificate in DER format
    let der_data = x509_cert.to_der()
        .map_err(|e| format!("Failed to convert certificate to DER: {}", e))?;
    
    // Encode as base64
    let cert_data = BASE64_STANDARD.encode(&der_data);
    
    // Extract subject name for certificate name
    let subject = x509_cert.subject_name();
    let cert_name = if let Some(cn) = subject.entries().find(|entry| entry.object().nid() == openssl::nid::Nid::COMMONNAME) {
        cn.data().as_utf8().map(|s| s.to_string()).unwrap_or_else(|_| format!("azure_cert_{}", index))
    } else {
        format!("azure_cert_{}", index)
    };
    
    Ok(Certificate {
        name: cert_name,
        certificate_data_format: "X509".to_string(),
        thumbprint: thumbprint_hex,
        data: cert_data,
        format: "Pfx".to_string(),
        store: "My".to_string(),
    })
}

/// Convenience wrapper for X509 owned types
fn extract_certificate_info(x509_cert: &openssl::x509::X509, index: usize) -> Result<Certificate> {
    extract_certificate_info_from_x509_ref(x509_cert.as_ref(), index)
}

/// Convenience wrapper for X509Ref types
fn extract_certificate_info_ref(x509_cert: &openssl::x509::X509Ref, index: usize) -> Result<Certificate> {
    extract_certificate_info_from_x509_ref(x509_cert, index)
}

/// Fallback function to extract certificate data using simple string parsing
pub fn extract_certificates_fallback(xml: &str) -> Option<Vec<Certificate>> {
    if cfg!(debug_assertions) {
        println!("Analyzing XML structure for certificate extraction...");
    }
    
    // Try to find any base64-encoded certificate data
    let base64_pattern = regex::Regex::new(r"[A-Za-z0-9+/]{100,}={0,2}").ok()?;
    let thumbprint_pattern = regex::Regex::new(r"[0-9A-Fa-f]{40}").ok()?;
    
    let mut certificates = Vec::new();
    
    // Look for base64 data that could be certificates
    for base64_match in base64_pattern.find_iter(xml) {
        let data = base64_match.as_str();
        
        // Try to find a corresponding thumbprint nearby
        let search_area = &xml[base64_match.start().saturating_sub(200)..
                               std::cmp::min(base64_match.end() + 200, xml.len())];
        
        if let Some(thumbprint_match) = thumbprint_pattern.find(search_area) {
            let thumbprint = thumbprint_match.as_str().to_uppercase();
            
            if cfg!(debug_assertions) {
                println!("Found certificate candidate - thumbprint: {}, data length: {}",
                         thumbprint, data.len());
            }
            
            certificates.push(Certificate {
                name: format!("cert_{}", &thumbprint[..8]),
                certificate_data_format: "Pfx".to_string(),
                thumbprint,
                data: data.to_string(),
                format: "Pfx".to_string(),
                store: "My".to_string(),
            });
        }
    }
    
    if certificates.is_empty() {
        None
    } else {
        Some(certificates)
    }
}

/// Find additional private keys in Azure directories
pub fn find_additional_private_keys() -> Vec<String> {
    use std::process::Command;
    
    let mut keys = Vec::new();
    
    if cfg!(debug_assertions) {
        println!("Searching for additional private keys...");
    }
    
    // Check if Certificates.pem contains private keys
    if std::path::Path::new("/var/lib/waagent-rs/Certificates.pem").exists() {
        // Try to extract private keys from the combined certificates file
        let output = Command::new("openssl")
            .args(["pkey", "-in", "/var/lib/waagent-rs/Certificates.pem", "-noout", "-text"])
            .output();
        
        match output {
            Ok(result) => {
                if result.status.success() {
                    if cfg!(debug_assertions) {
                        println!("Found private key in Certificates.pem");
                    }
                    keys.push("/var/lib/waagent-rs/Certificates.pem".to_string());
                }
            }
            Err(_) => {
                if cfg!(debug_assertions) {
                    println!("Failed to check Certificates.pem for private keys");
                }
            }
        }
    }
    
    // Look for any files with the certificate thumbprint as filename
    let thumbprint = "1840F31C4C5A85387399F4C0915D7ACB20BD0A1A";
    let potential_key_files = [
        format!("/var/lib/waagent-rs/{}.pem", thumbprint),
        format!("/var/lib/waagent-rs/{}.key", thumbprint),
        format!("/var/lib/waagent-rs/{}Private.pem", thumbprint),
        format!("/var/lib/waagent-rs/PrivateKey_{}.pem", thumbprint),
    ];
    
    for key_file in &potential_key_files {
        if std::path::Path::new(key_file).exists() {
            if cfg!(debug_assertions) {
                println!("Found potential key file: {}", key_file);
            }
            keys.push(key_file.clone());
        }
    }
    
    if keys.is_empty() && cfg!(debug_assertions) {
        println!("No additional private keys found");
    }
    
    keys
}

/// Download and save certificates from WireServer
pub async fn download_and_save_certificates(
    client: &reqwest::Client, 
    goal_state: &crate::wireserver::GoalState, 
    agent_name: &str
) -> Result<()> {
    use crate::crypto::{generate_transport_certificate, extract_certificates_from_p7m};
    use std::time::Duration;
    
    const WIRESERVER_API_VERSION: &str = "2012-11-30";
    
    let certificates_url = &goal_state.container.role_instance_list.role_instance.configuration.certificates;
    
    if certificates_url.is_empty() {
        println!("No certificates URL in goal state");
        return Ok(());
    }
    
    // First, ensure we have a transport certificate to authenticate with WireServer
    let transport_cert_path = "/var/lib/waagent-rs/TransportCert.pem";
    let transport_key_path = "/var/lib/waagent-rs/TransportPrivate.pem";
    
    if !std::path::Path::new(transport_cert_path).exists() {
        println!("Generating transport certificate...");
        match generate_transport_certificate(transport_cert_path, transport_key_path) {
            Ok(_) => println!("Generated transport certificate"),
            Err(e) => {
                eprintln!("Failed to generate transport certificate: {}", e);
                return Err(format!("Failed to generate transport certificate: {}", e).into());
            }
        }
    }
    
    // Read the transport certificate to send in header
    let transport_cert_pem = std::fs::read_to_string(transport_cert_path)?;
    
    // Extract just the certificate part (remove headers and newlines)
    let cert_base64 = transport_cert_pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<String>();
    
    println!("Downloading certificates from: {}", certificates_url);
    
    let response = client
        .get(certificates_url)
        .header("x-ms-version", WIRESERVER_API_VERSION)
        .header("x-ms-agent-name", agent_name)
        .header("x-ms-guest-agent-public-x509-cert", cert_base64)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        eprintln!("Failed to download certificates: {}", error_text);
        return Err(format!("Certificate download failed with status {}", status).into());
    }
    
    let certificates_xml = response.text().await?;
    
    // Save the raw XML for debugging
    std::fs::write("/var/lib/waagent-rs/Certificates.xml", &certificates_xml)?;
    
    // Parse the CMS/PKCS7 certificate data from XML
    use quick_xml::de::from_str;
    use serde::Deserialize;
    
    #[derive(Debug, Deserialize)]
    struct CertificateData {
        #[serde(rename = "$value")]
        content: String,
    }
    
    #[derive(Debug, Deserialize)]
    struct CertificatesConfig {
        #[serde(rename = "Data")]
        data: CertificateData,
    }
    
    match from_str::<CertificatesConfig>(&certificates_xml) {
        Ok(config) => {
            println!("Parsed certificates XML");
            
            // The data is base64-encoded PKCS7/CMS, but may contain newlines
            // Remove all whitespace before decoding
            let cleaned_base64: String = config.data.content.chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            match BASE64_STANDARD.decode(cleaned_base64) {
                Ok(cert_data) => {
                    // Save as p7m (PKCS7 format)
                    let p7m_path = "/var/lib/waagent-rs/Certificates.p7m";
                    std::fs::write(p7m_path, &cert_data)?;
                    println!("Saved {} bytes to {}", cert_data.len(), p7m_path);
                    
                    // Extract certificates from PKCS7
                    match extract_certificates_from_p7m(p7m_path) {
                        Ok(_) => println!("Successfully extracted certificates from PKCS7"),
                        Err(e) => eprintln!("Failed to extract certificates: {}", e),
                    }
                }
                Err(e) => eprintln!("Failed to decode certificate data: {}", e),
            }
        }
        Err(e) => eprintln!("Failed to parse certificates XML: {}", e),
    }
    
    Ok(())
}
