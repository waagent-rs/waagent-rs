//! Parser for `ovf-env.xml` — port of `azurelinuxagent/common/protocol/ovfenv.py`.
//!
//! The provisioning ISO that Azure attaches at first boot contains an OVF
//! Environment document with a Microsoft-specific
//! `LinuxProvisioningConfigurationSet` element. This module parses just the
//! fields the agent actually needs (hostname, username, SSH keys, custom
//! data, …) without pulling in a full XSD-driven binding.

use std::fs;
use std::io;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::escape::unescape;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

/// Maximum supported OVF version. Newer versions only trigger a warning,
/// matching the behaviour in WALA's `OvfEnv.parse`.
pub const OVF_VERSION: &str = "1.0";

/// `xmlns` for the OVF Environment root element.
pub const OVF_NAMESPACE: &[u8] = b"http://schemas.dmtf.org/ovf/environment/1";
/// `xmlns` for the Microsoft Azure provisioning extensions.
pub const WA_NAMESPACE: &[u8] = b"http://schemas.microsoft.com/windowsazure";

/// A single SSH public key entry from `<SSH><PublicKeys><PublicKey>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SshPublicKey {
    pub path: Option<String>,
    pub fingerprint: Option<String>,
    pub value: Option<String>,
}

/// A single SSH host key pair entry from `<SSH><KeyPairs><KeyPair>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SshKeyPair {
    pub path: Option<String>,
    pub fingerprint: Option<String>,
}

/// Decoded `ovf-env.xml`. Mirrors the public attributes of `OvfEnv` in
/// the Python agent (only the bits we currently consume).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OvfEnv {
    pub version: String,
    pub hostname: String,
    pub username: String,
    pub user_password: Option<String>,
    pub custom_data: Option<String>,
    pub disable_ssh_password_auth: bool,
    pub ssh_pubkeys: Vec<SshPublicKey>,
    pub ssh_keypairs: Vec<SshKeyPair>,
    pub provision_guest_agent: Option<String>,
}

/// Errors returned while parsing `ovf-env.xml`.
#[derive(Debug)]
pub enum OvfError {
    /// I/O error reading the file.
    Io(io::Error),
    /// XML was malformed.
    Xml(quick_xml::Error),
    /// A required element was missing or empty.
    Missing(&'static str),
}

impl std::fmt::Display for OvfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OvfError::Io(e) => write!(f, "{}", e),
            OvfError::Xml(e) => write!(f, "malformed ovf-env.xml: {}", e),
            OvfError::Missing(field) => {
                write!(f, "ovf-env.xml is missing required element: {}", field)
            }
        }
    }
}

impl std::error::Error for OvfError {}

impl From<io::Error> for OvfError {
    fn from(e: io::Error) -> Self {
        OvfError::Io(e)
    }
}

impl From<quick_xml::Error> for OvfError {
    fn from(e: quick_xml::Error) -> Self {
        OvfError::Xml(e)
    }
}

impl From<quick_xml::encoding::EncodingError> for OvfError {
    fn from(e: quick_xml::encoding::EncodingError) -> Self {
        OvfError::Xml(quick_xml::Error::Encoding(e))
    }
}

/// Read and parse an `ovf-env.xml` file from disk.
pub fn load(path: &Path) -> Result<OvfEnv, OvfError> {
    let raw = fs::read_to_string(path)?;
    parse(&raw)
}

/// Parse `ovf-env.xml` from an in-memory string.
pub fn parse(xml: &str) -> Result<OvfEnv, OvfError> {
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut env = OvfEnv::default();
    // We collect text into the most recently opened element. The Python
    // implementation walks the DOM, but a streaming pass is enough here
    // because the schema is shallow and there are no repeated text nodes
    // we care about beyond direct children.
    let mut path: Vec<TaggedName> = Vec::new();
    let mut text_buf = String::new();

    // Per-PublicKey / KeyPair scratch state.
    let mut current_pubkey: Option<SshPublicKey> = None;
    let mut current_keypair: Option<SshKeyPair> = None;
    // Provisioning version vs platform version: only the one declared inside
    // ProvisioningSection is what WALA validates against OVF_VERSION.
    let mut in_provisioning_section = false;
    let mut in_platform_section = false;

    let mut buf = Vec::new();
    loop {
        match reader.read_resolved_event_into(&mut buf)? {
            (_, Event::Eof) => break,
            (ns, Event::Start(e)) => {
                let local = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                let tag = TaggedName {
                    local: local.clone(),
                    namespace: namespace_kind(&ns),
                };

                match (tag.namespace, tag.local.as_str()) {
                    (Ns::Wa, "ProvisioningSection") => in_provisioning_section = true,
                    (Ns::Wa, "PlatformSettingsSection") => in_platform_section = true,
                    (Ns::Wa, "PublicKey") => current_pubkey = Some(SshPublicKey::default()),
                    (Ns::Wa, "KeyPair") => current_keypair = Some(SshKeyPair::default()),
                    _ => {}
                }

                path.push(tag);
                text_buf.clear();
            }
            (_, Event::Text(t)) => {
                // `xml_content` performs decoding + line-end normalization;
                // `escape::unescape` then resolves XML entities (`&amp;`, …).
                let decoded = t.xml_content()?;
                let unescaped = unescape(&decoded)
                    .map_err(|e| OvfError::Xml(quick_xml::Error::Escape(e)))?;
                text_buf.push_str(&unescaped);
            }
            (ns, Event::End(e)) => {
                let local = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                let kind = namespace_kind(&ns);
                let value = std::mem::take(&mut text_buf);

                if kind == Ns::Wa {
                    match local.as_str() {
                        "Version" if in_provisioning_section && env.version.is_empty() => {
                            env.version = value.trim().to_string();
                        }
                        "HostName" => env.hostname = value.trim().to_string(),
                        "UserName" => env.username = value.trim().to_string(),
                        "UserPassword" => env.user_password = non_empty(&value),
                        "CustomData" => env.custom_data = non_empty(&value),
                        "DisableSshPasswordAuthentication" => {
                            env.disable_ssh_password_auth = value.trim().eq_ignore_ascii_case("true");
                        }
                        "ProvisionGuestAgent" if in_platform_section => {
                            env.provision_guest_agent = non_empty(&value);
                        }
                        "Path" => {
                            if let Some(pk) = current_pubkey.as_mut() {
                                pk.path = non_empty(&value);
                            } else if let Some(kp) = current_keypair.as_mut() {
                                kp.path = non_empty(&value);
                            }
                        }
                        "Fingerprint" => {
                            if let Some(pk) = current_pubkey.as_mut() {
                                pk.fingerprint = non_empty(&value);
                            } else if let Some(kp) = current_keypair.as_mut() {
                                kp.fingerprint = non_empty(&value);
                            }
                        }
                        "Value" => {
                            if let Some(pk) = current_pubkey.as_mut() {
                                pk.value = non_empty(&value.trim());
                            }
                        }
                        "PublicKey" => {
                            if let Some(pk) = current_pubkey.take() {
                                env.ssh_pubkeys.push(pk);
                            }
                        }
                        "KeyPair" => {
                            if let Some(kp) = current_keypair.take() {
                                env.ssh_keypairs.push(kp);
                            }
                        }
                        "ProvisioningSection" => in_provisioning_section = false,
                        "PlatformSettingsSection" => in_platform_section = false,
                        _ => {}
                    }
                }

                path.pop();
            }
            // Self-closing tags such as `<GuestAgentPackageName xsi:nil="true"/>`
            // arrive as Empty; we only care that they don't disturb our state.
            (_, Event::Empty(_)) => {}
            _ => {}
        }
        buf.clear();
    }

    // Validation parity with `_validate_ovf` in the Python agent.
    if env.version.is_empty() {
        return Err(OvfError::Missing("Version"));
    }
    if env.hostname.is_empty() {
        return Err(OvfError::Missing("HostName"));
    }
    if env.username.is_empty() {
        return Err(OvfError::Missing("UserName"));
    }

    if env.version.as_str() > OVF_VERSION {
        tracing::warn!(
            "newer OVF provisioning configuration detected ({} > {}); please consider updating waagent-rs",
            env.version, OVF_VERSION
        );
    }

    Ok(env)
}

/// Convert `quick-xml`'s resolved namespace into our internal enum.
fn namespace_kind(resolved: &ResolveResult<'_>) -> Ns {
    match resolved {
        ResolveResult::Bound(ns) if ns.as_ref() == OVF_NAMESPACE => Ns::Ovf,
        ResolveResult::Bound(ns) if ns.as_ref() == WA_NAMESPACE => Ns::Wa,
        _ => Ns::Other,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ns {
    Ovf,
    Wa,
    Other,
}

#[derive(Debug, Clone)]
struct TaggedName {
    #[allow(dead_code)]
    local: String,
    #[allow(dead_code)]
    namespace: Ns,
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic fixture matching the structure observed on a freshly
    /// provisioned Ubuntu Azure VM (whitespace, namespaces, element order).
    /// Personally-identifying values are placeholders.
    const SAMPLE_UBUNTU: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<ns0:Environment xmlns:ns0="http://schemas.dmtf.org/ovf/environment/1"
                 xmlns:ns1="http://schemas.microsoft.com/windowsazure"
                 xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <ns1:ProvisioningSection>
    <ns1:Version>1.0</ns1:Version>
    <ns1:LinuxProvisioningConfigurationSet>
      <ns1:ConfigurationSetType>LinuxProvisioningConfiguration</ns1:ConfigurationSetType>
      <ns1:UserName>sampleuser</ns1:UserName>
      <ns1:DisableSshPasswordAuthentication>true</ns1:DisableSshPasswordAuthentication>
      <ns1:SSH>
        <ns1:PublicKeys>
          <ns1:PublicKey>
            <ns1:Path>/home/sampleuser/.ssh/authorized_keys</ns1:Path>
            <ns1:Value>ssh-rsa AAAAEXAMPLEKEY sample@host</ns1:Value>
          </ns1:PublicKey>
        </ns1:PublicKeys>
      </ns1:SSH>
      <ns1:HostName>samplehost</ns1:HostName>
    </ns1:LinuxProvisioningConfigurationSet>
  </ns1:ProvisioningSection>
  <ns1:PlatformSettingsSection>
    <ns1:Version>1.0</ns1:Version>
    <ns1:PlatformSettings>
      <ns1:KmsServerHostname>azkms.core.windows.net</ns1:KmsServerHostname>
      <ns1:ProvisionGuestAgent>true</ns1:ProvisionGuestAgent>
      <ns1:GuestAgentPackageName xsi:nil="true" />
      <ns1:RetainWindowsPEPassInUnattend>true</ns1:RetainWindowsPEPassInUnattend>
      <ns1:RetainOfflineServicingPassInUnattend>true</ns1:RetainOfflineServicingPassInUnattend>
      <ns1:PreprovisionedVm>false</ns1:PreprovisionedVm>
      <ns1:EnableTrustedImageIdentifier>false</ns1:EnableTrustedImageIdentifier>
    </ns1:PlatformSettings>
  </ns1:PlatformSettingsSection>
</ns0:Environment>
"#;

    #[test]
    fn parses_ubuntu_sample() {
        let env = parse(SAMPLE_UBUNTU).expect("parse should succeed");
        assert_eq!(env.version, "1.0");
        assert_eq!(env.hostname, "samplehost");
        assert_eq!(env.username, "sampleuser");
        assert!(env.disable_ssh_password_auth);
        assert_eq!(env.user_password, None);
        assert_eq!(env.custom_data, None);
        assert_eq!(env.provision_guest_agent.as_deref(), Some("true"));

        assert_eq!(env.ssh_pubkeys.len(), 1);
        let pk = &env.ssh_pubkeys[0];
        assert_eq!(pk.path.as_deref(), Some("/home/sampleuser/.ssh/authorized_keys"));
        assert_eq!(pk.value.as_deref(), Some("ssh-rsa AAAAEXAMPLEKEY sample@host"));
        assert_eq!(pk.fingerprint, None);
        assert!(env.ssh_keypairs.is_empty());
    }

    #[test]
    fn parses_keypairs_and_password() {
        let xml = r#"<ns0:Environment xmlns:ns0="http://schemas.dmtf.org/ovf/environment/1"
                                      xmlns:ns1="http://schemas.microsoft.com/windowsazure">
            <ns1:ProvisioningSection>
              <ns1:Version>1.0</ns1:Version>
              <ns1:LinuxProvisioningConfigurationSet>
                <ns1:HostName>h</ns1:HostName>
                <ns1:UserName>u</ns1:UserName>
                <ns1:UserPassword>secret</ns1:UserPassword>
                <ns1:CustomData>Y2xvdWQtaW5pdA==</ns1:CustomData>
                <ns1:DisableSshPasswordAuthentication>false</ns1:DisableSshPasswordAuthentication>
                <ns1:SSH>
                  <ns1:KeyPairs>
                    <ns1:KeyPair>
                      <ns1:Path>/home/u/.ssh/id_rsa</ns1:Path>
                      <ns1:Fingerprint>AB12</ns1:Fingerprint>
                    </ns1:KeyPair>
                  </ns1:KeyPairs>
                </ns1:SSH>
              </ns1:LinuxProvisioningConfigurationSet>
            </ns1:ProvisioningSection>
            <ns1:PlatformSettingsSection>
              <ns1:Version>1.0</ns1:Version>
              <ns1:PlatformSettings>
                <ns1:ProvisionGuestAgent>true</ns1:ProvisionGuestAgent>
              </ns1:PlatformSettings>
            </ns1:PlatformSettingsSection>
        </ns0:Environment>"#;

        let env = parse(xml).unwrap();
        assert_eq!(env.user_password.as_deref(), Some("secret"));
        assert_eq!(env.custom_data.as_deref(), Some("Y2xvdWQtaW5pdA=="));
        assert!(!env.disable_ssh_password_auth);
        assert_eq!(env.ssh_keypairs.len(), 1);
        assert_eq!(env.ssh_keypairs[0].fingerprint.as_deref(), Some("AB12"));
    }

    #[test]
    fn missing_required_field_errors() {
        let xml = r#"<ns0:Environment xmlns:ns0="http://schemas.dmtf.org/ovf/environment/1"
                                      xmlns:ns1="http://schemas.microsoft.com/windowsazure">
            <ns1:ProvisioningSection>
              <ns1:Version>1.0</ns1:Version>
              <ns1:LinuxProvisioningConfigurationSet>
                <ns1:UserName>u</ns1:UserName>
              </ns1:LinuxProvisioningConfigurationSet>
            </ns1:ProvisioningSection>
        </ns0:Environment>"#;
        match parse(xml) {
            Err(OvfError::Missing("HostName")) => {}
            other => panic!("expected Missing(HostName), got {:?}", other),
        }
    }
}
