use std::process::Command;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Generate a self-signed transport certificate for Azure communication
pub fn generate_transport_certificate(cert_path: &str, key_path: &str) -> Result<()> {
    // Create directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(cert_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Generate a self-signed certificate valid for 3650 days (10 years)
    let output = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-nodes",
            "-subj", "/CN=LinuxTransport",
            "-days", "3650",
            "-newkey", "rsa:2048",
            "-keyout", key_path,
            "-out", cert_path,
        ])
        .output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to generate certificate: {}", error).into());
    }
    
    // Set restrictive permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(cert_path, std::fs::Permissions::from_mode(0o400))?;
        std::fs::set_permissions(key_path, std::fs::Permissions::from_mode(0o400))?;
    }
    
    println!("Generated transport certificate at {} and {}", cert_path, key_path);
    Ok(())
}

/// Extract certificates from PKCS7/P7M file
pub fn extract_certificates_from_p7m(p7m_path: &str) -> Result<()> {
    // The PKCS7 data is CMS enveloped (encrypted), we need to decrypt it first
    // with the transport private key
    let transport_key_path = "/var/lib/waagent-rs/TransportPrivate.pem";
    let transport_cert_path = "/var/lib/waagent-rs/TransportCert.pem";
    
    // Step 1: Decrypt the CMS/PKCS7 enveloped data to get a PFX/PKCS12 file
    // This follows the same approach as the Python WALinuxAgent
    let pfx_file = "/tmp/decrypted.pfx";
    let output = Command::new("openssl")
        .args([
            "cms",
            "-decrypt",
            "-in", p7m_path,
            "-inform", "DER",
            "-recip", transport_cert_path,
            "-inkey", transport_key_path,
        ])
        .output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to decrypt CMS enveloped data: {}", error).into());
    }
    
    // Save decrypted PFX data
    std::fs::write(pfx_file, &output.stdout)?;
    
    // Step 2: Convert PFX/PKCS12 to PEM format
    // The PFX is encrypted with an empty password
    let pem_output = Command::new("openssl")
        .args([
            "pkcs12",
            "-in", pfx_file,
            "-out", "/dev/stdout",
            "-nodes",
            "-password", "pass:",
        ])
        .output()?;
    
    if !pem_output.status.success() {
        let error = String::from_utf8_lossy(&pem_output.stderr);
        return Err(format!("Failed to convert PFX to PEM: {}", error).into());
    }
    
    let certs_pem = String::from_utf8_lossy(&pem_output.stdout);
    
    // Save combined certificates
    let combined_path = "/var/lib/waagent-rs/Certificates.pem";
    std::fs::write(combined_path, certs_pem.as_bytes())?;
    println!("Saved combined certificates to {}", combined_path);
    
    // Extract private key if present
    if certs_pem.contains("-----BEGIN PRIVATE KEY-----") || certs_pem.contains("-----BEGIN RSA PRIVATE KEY-----") {
        let key_start = certs_pem.find("-----BEGIN").unwrap_or(0);
        let key_end = if let Some(pos) = certs_pem.find("-----END PRIVATE KEY-----") {
            pos + "-----END PRIVATE KEY-----".len()
        } else if let Some(pos) = certs_pem.find("-----END RSA PRIVATE KEY-----") {
            pos + "-----END RSA PRIVATE KEY-----".len()
        } else {
            0
        };
        
        if key_end > key_start {
            // We'll save the private key with the certificate thumbprint later
            // For now, just note that we have it
            println!("Found private key in PFX");
        }
    }
    
    // Split into individual certificates and save each with its thumbprint
    let cert_blocks: Vec<&str> = certs_pem
        .split("-----END CERTIFICATE-----")
        .filter(|block| block.contains("-----BEGIN CERTIFICATE-----"))
        .collect();
    
    for (i, cert_block) in cert_blocks.iter().enumerate() {
        let cert_pem = format!("{}-----END CERTIFICATE-----\n", cert_block);
        
        // Calculate thumbprint for this certificate
        match calculate_cert_thumbprint(&cert_pem) {
            Ok(thumbprint) => {
                // Save certificate with thumbprint as filename
                let cert_file = format!("/var/lib/waagent-rs/{}.crt", thumbprint);
                
                // Remove existing file if it exists (to avoid permission issues)
                let _ = std::fs::remove_file(&cert_file);
                
                std::fs::write(&cert_file, cert_pem.as_bytes())?;
                
                // Set restrictive permissions
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&cert_file, std::fs::Permissions::from_mode(0o400))?;
                }
                
                // Extract and save the private key with the same thumbprint
                if certs_pem.contains("-----BEGIN PRIVATE KEY-----") || certs_pem.contains("-----BEGIN RSA PRIVATE KEY-----") {
                    let key_start_marker = if certs_pem.contains("-----BEGIN PRIVATE KEY-----") {
                        "-----BEGIN PRIVATE KEY-----"
                    } else {
                        "-----BEGIN RSA PRIVATE KEY-----"
                    };
                    let key_end_marker = if certs_pem.contains("-----END PRIVATE KEY-----") {
                        "-----END PRIVATE KEY-----"
                    } else {
                        "-----END RSA PRIVATE KEY-----"
                    };
                    
                    if let Some(key_start) = certs_pem.find(key_start_marker) {
                        if let Some(key_end) = certs_pem.find(key_end_marker) {
                            let private_key_pem = &certs_pem[key_start..key_end + key_end_marker.len()];
                            let key_file = format!("/var/lib/waagent-rs/{}.prv", thumbprint);
                            
                            // Remove existing file if it exists
                            let _ = std::fs::remove_file(&key_file);
                            
                            std::fs::write(&key_file, private_key_pem.as_bytes())?;
                            
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                std::fs::set_permissions(&key_file, std::fs::Permissions::from_mode(0o400))?;
                            }
                            
                            println!("Saved private key for certificate {}", thumbprint);
                        }
                    }
                }
                
                println!("Saved certificate {} with thumbprint {}", i + 1, thumbprint);
            }
            Err(e) => {
                eprintln!("Failed to calculate thumbprint for certificate {}: {}", i + 1, e);
            }
        }
    }
    
    Ok(())
}

/// Calculate SHA-1 thumbprint of a certificate
pub fn calculate_cert_thumbprint(cert_pem: &str) -> Result<String> {
    // Write cert to temp file
    let temp_cert = "/tmp/temp_cert.pem";
    std::fs::write(temp_cert, cert_pem)?;
    
    let output = Command::new("openssl")
        .args(["x509", "-in", temp_cert, "-fingerprint", "-sha1", "-noout"])
        .output()?;
    
    if output.status.success() {
        let fingerprint_line = String::from_utf8_lossy(&output.stdout);
        // Extract just the hex part: "SHA1 Fingerprint=AB:CD:EF..."
        if let Some(hex_part) = fingerprint_line.split('=').nth(1) {
            let thumbprint = hex_part.trim().replace(':', "");
            Ok(thumbprint)
        } else {
            Err("Failed to parse certificate fingerprint".into())
        }
    } else {
        Err("Failed to calculate certificate thumbprint".into())
    }
}
