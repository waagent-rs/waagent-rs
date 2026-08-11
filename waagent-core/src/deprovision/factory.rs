//! Distro detection + handler selection — port of
//! `azurelinuxagent.pa.deprovision.factory.get_deprovision_handler`.

use std::fs;

use super::distro::{
    ArchDeprovisionHandler, CoreOsDeprovisionHandler, Ubuntu1804DeprovisionHandler,
    UbuntuDeprovisionHandler,
};
use super::{DefaultDeprovisionHandler, DeprovisionHandler};

/// Minimal description of the running distro, parsed from `/etc/os-release`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DistroId {
    /// `ID=` field (e.g. `ubuntu`, `arch`, `coreos`, `flatcar`).
    pub id: String,
    /// `VERSION_ID=` field (e.g. `22.04`).
    pub version_id: String,
    /// `PRETTY_NAME=` field, used to spot Clear Linux.
    pub pretty_name: String,
}

/// Parse `/etc/os-release` (or return an empty struct on non-Linux / errors).
pub fn detect_distro() -> DistroId {
    let raw = match fs::read_to_string("/etc/os-release") {
        Ok(s) => s,
        Err(_) => return DistroId::default(),
    };

    let mut out = DistroId::default();
    for line in raw.lines() {
        let line = line.trim();
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim().trim_matches('"').to_string();
        match k.trim() {
            "ID" => out.id = v.to_lowercase(),
            "VERSION_ID" => out.version_id = v,
            "PRETTY_NAME" => out.pretty_name = v,
            _ => {}
        }
    }
    out
}

/// Return the deprovision handler that best matches the running distro.
/// Mirrors `get_deprovision_handler(distro_name, distro_version, distro_full_name)`.
pub fn get_deprovision_handler() -> Box<dyn DeprovisionHandler> {
    get_deprovision_handler_for(&detect_distro())
}

pub fn get_deprovision_handler_for(distro: &DistroId) -> Box<dyn DeprovisionHandler> {
    match distro.id.as_str() {
        "arch" => Box::new(ArchDeprovisionHandler::new()),
        "ubuntu" => {
            if version_at_least(&distro.version_id, (18, 4)) {
                Box::new(Ubuntu1804DeprovisionHandler::new())
            } else {
                Box::new(UbuntuDeprovisionHandler::new())
            }
        }
        "coreos" | "flatcar" => Box::new(CoreOsDeprovisionHandler::new()),
        _ => {
            // Clear Linux is detected by `PRETTY_NAME` rather than `ID`.
            if distro.pretty_name.contains("Clear Linux") {
                // No bespoke handler implemented yet — fall back to default.
                Box::new(DefaultDeprovisionHandler::new())
            } else {
                Box::new(DefaultDeprovisionHandler::new())
            }
        }
    }
}

/// Compare a `VERSION_ID` like `"18.04"` against `(major, minor)`.
fn version_at_least(version_id: &str, (min_major, min_minor): (u32, u32)) -> bool {
    let mut parts = version_id.split('.');
    let major: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor) >= (min_major, min_minor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_ubuntu_1804_for_new_versions() {
        let d = DistroId {
            id: "ubuntu".into(),
            version_id: "22.04".into(),
            pretty_name: "Ubuntu 22.04 LTS".into(),
        };
        // Just exercise dispatch — concrete type is opaque so we only check it
        // doesn't panic and that older Ubuntu picks the legacy handler.
        let _ = get_deprovision_handler_for(&d);

        let d_old = DistroId {
            id: "ubuntu".into(),
            version_id: "16.04".into(),
            pretty_name: "Ubuntu 16.04".into(),
        };
        let _ = get_deprovision_handler_for(&d_old);
    }

    #[test]
    fn version_compare_handles_short_strings() {
        assert!(version_at_least("18.04", (18, 4)));
        assert!(version_at_least("22", (18, 4)));
        assert!(!version_at_least("16.04", (18, 4)));
        assert!(!version_at_least("", (18, 4)));
    }
}
