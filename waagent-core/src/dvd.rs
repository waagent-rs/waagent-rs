//! Provisioning DVD helpers — port of `OSUtil.{get_dvd_device, mount_dvd,
//! umount_dvd, get_mount_point}` from `azurelinuxagent/common/osutil/default.py`.
//!
//! Azure attaches a small ISO to the VM at first boot containing
//! `ovf-env.xml` and certificate material. This module locates that device
//! and mounts it read-only so the rest of the agent can read its contents.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use tracing::{debug, info, warn};

use crate::config::{Config, ConfigValue};

/// Default mount point if the config does not override `DVD.MountPoint`.
pub const DEFAULT_MOUNT_POINT: &str = "/mnt/cdrom/secure";

/// Filesystem types tried by the WALA agent, in the same order.
const DVD_FS_TYPES: &str = "udf,iso9660,vfat";

/// Errors returned by the DVD helpers.
#[derive(Debug)]
pub enum DvdError {
    /// No device under `/dev` matched the DVD pattern.
    DeviceNotFound { dev_dir: PathBuf, candidates: Vec<String> },
    /// `mount` failed every retry.
    MountFailed { device: PathBuf, mount_point: PathBuf, last_error: String },
    /// `umount` failed.
    UmountFailed { mount_point: PathBuf, error: String },
    /// I/O error while inspecting `/dev`, creating the mount point, etc.
    Io(io::Error),
}

impl std::fmt::Display for DvdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DvdError::DeviceNotFound { dev_dir, candidates } => write!(
                f,
                "no DVD device found under {}; candidates were {:?}",
                dev_dir.display(),
                candidates
            ),
            DvdError::MountFailed { device, mount_point, last_error } => write!(
                f,
                "failed to mount {} at {}: {}",
                device.display(),
                mount_point.display(),
                last_error
            ),
            DvdError::UmountFailed { mount_point, error } => {
                write!(f, "failed to unmount {}: {}", mount_point.display(), error)
            }
            DvdError::Io(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for DvdError {}

impl From<io::Error> for DvdError {
    fn from(e: io::Error) -> Self {
        DvdError::Io(e)
    }
}

/// Configuration for [`mount_dvd`]. Mirrors the keyword arguments on
/// `OSUtil.mount_dvd` so callers can tune retry behavior.
#[derive(Debug, Clone)]
pub struct MountOptions {
    pub max_retries: u32,
    pub sleep_between_retries: Duration,
    /// Override the auto-detected device (e.g. `/dev/sr0`).
    pub device: Option<PathBuf>,
    /// Override the configured mount point.
    pub mount_point: Option<PathBuf>,
}

impl Default for MountOptions {
    fn default() -> Self {
        Self {
            max_retries: 6,
            sleep_between_retries: Duration::from_secs(5),
            device: None,
            mount_point: None,
        }
    }
}

/// Result of a successful mount: the device that was mounted and where.
#[derive(Debug, Clone)]
pub struct MountedDvd {
    pub device: PathBuf,
    pub mount_point: PathBuf,
    /// `true` when the device was already mounted before this call.
    pub already_mounted: bool,
}

/// Locate the provisioning DVD by scanning entries in `dev_dir`.
///
/// Mirrors `DefaultOSUtil.get_dvd_device`: matches the first entry whose name
/// matches `sr[0-9] | hd[c-z] | cdrom[0-9] | cd[0-9] | vd[b-z]`.
pub fn get_dvd_device(dev_dir: &Path) -> Result<PathBuf, DvdError> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir(dev_dir)? {
        let entry = entry?;
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue, // skip non-utf8 device names
        };
        candidates.push(name.clone());
        if matches_dvd_pattern(&name) {
            return Ok(dev_dir.join(name));
        }
    }
    Err(DvdError::DeviceNotFound {
        dev_dir: dev_dir.to_path_buf(),
        candidates,
    })
}

/// Read the configured mount point (`DVD.MountPoint`) or fall back to
/// [`DEFAULT_MOUNT_POINT`].
pub fn configured_mount_point(config: &Config) -> PathBuf {
    match config.get_value("DVD.MountPoint") {
        Some(ConfigValue::String(s)) if !s.is_empty() && s != "None" => PathBuf::from(s),
        _ => PathBuf::from(DEFAULT_MOUNT_POINT),
    }
}

/// Mount the provisioning DVD read-only with retry semantics matching WALA.
///
/// On success returns the device + mount point that ended up mounted. If the
/// device was already mounted (anywhere), that pre-existing mount point is
/// returned and `mount` is **not** invoked again.
pub fn mount_dvd(config: &Config, opts: &MountOptions) -> Result<MountedDvd, DvdError> {
    let device = match &opts.device {
        Some(d) => d.clone(),
        None => get_dvd_device(Path::new("/dev"))?,
    };
    let mount_point = opts
        .mount_point
        .clone()
        .unwrap_or_else(|| configured_mount_point(config));

    // If `mount` already lists this device, skip remounting (matches WALA).
    if let Some(existing) = current_mount_point_for(&device)? {
        info!(
            "{} is already mounted at {}",
            device.display(),
            existing.display()
        );
        return Ok(MountedDvd {
            device,
            mount_point: existing,
            already_mounted: true,
        });
    }

    if !mount_point.is_dir() {
        fs::create_dir_all(&mount_point)?;
    }

    let max = opts.max_retries.max(1);
    let mut last_err = String::new();
    for retry in 1..=max {
        match try_mount(&device, &mount_point) {
            Ok(()) => {
                info!(
                    "successfully mounted {} at {}",
                    device.display(),
                    mount_point.display()
                );
                return Ok(MountedDvd {
                    device,
                    mount_point,
                    already_mounted: false,
                });
            }
            Err(err) => {
                warn!(
                    "mounting dvd failed [retry {}/{}, sleeping {:?}]: {}",
                    retry, max, opts.sleep_between_retries, err
                );
                last_err = err;
                if retry < max {
                    thread::sleep(opts.sleep_between_retries);
                }
            }
        }
    }

    Err(DvdError::MountFailed {
        device,
        mount_point,
        last_error: last_err,
    })
}

/// Unmount whatever is at `mount_point` (or the configured one when `None`).
pub fn umount_dvd(config: &Config, mount_point: Option<&Path>) -> Result<(), DvdError> {
    let mp = mount_point
        .map(Path::to_path_buf)
        .unwrap_or_else(|| configured_mount_point(config));
    debug!("unmounting {}", mp.display());

    match Command::new("umount").arg(&mp).output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(DvdError::UmountFailed {
            mount_point: mp,
            error: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        }),
        Err(err) => Err(DvdError::UmountFailed {
            mount_point: mp,
            error: err.to_string(),
        }),
    }
}

// ---------------------------------------------------------------------------
// internals
// ---------------------------------------------------------------------------

/// Match the same regex used by WALA: `sr[0-9] | hd[c-z] | cdrom[0-9] | cd[0-9] | vd[b-z]`.
fn matches_dvd_pattern(name: &str) -> bool {
    fn one_digit(rest: &str) -> bool {
        rest.len() == 1 && rest.chars().next().unwrap().is_ascii_digit()
    }
    if let Some(rest) = name.strip_prefix("sr") {
        return one_digit(rest);
    }
    if let Some(rest) = name.strip_prefix("hd") {
        return rest.len() == 1 && matches!(rest.chars().next().unwrap(), 'c'..='z');
    }
    if let Some(rest) = name.strip_prefix("cdrom") {
        return one_digit(rest);
    }
    if let Some(rest) = name.strip_prefix("cd") {
        return one_digit(rest);
    }
    if let Some(rest) = name.strip_prefix("vd") {
        return rest.len() == 1 && matches!(rest.chars().next().unwrap(), 'b'..='z');
    }
    false
}

/// Run `mount -o ro -t udf,iso9660,vfat <device> <mount_point>`.
fn try_mount(device: &Path, mount_point: &Path) -> Result<(), String> {
    let output = Command::new("mount")
        .args(["-o", "ro", "-t", DVD_FS_TYPES])
        .arg(device)
        .arg(mount_point)
        .output()
        .map_err(|e| format!("failed to spawn mount: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "mount exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Return the existing mount point for `device`, if any. Uses `mount` (no
/// args) so we don't depend on `/proc/mounts` formatting.
fn current_mount_point_for(device: &Path) -> Result<Option<PathBuf>, DvdError> {
    let output = match Command::new("mount").output() {
        Ok(o) => o,
        Err(err) => {
            // Treat a missing `mount` binary as "nothing mounted" rather than
            // a hard failure — this lets unit tests run on systems without it.
            warn!("could not run `mount`: {}", err);
            return Ok(None);
        }
    };
    if !output.status.success() {
        warn!(
            "`mount` exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return Ok(None);
    }
    let listing = String::from_utf8_lossy(&output.stdout).into_owned();
    Ok(get_mount_point(&listing, device))
}

/// Parse the output of `mount` and return the mount point for `device`, if
/// listed. Equivalent to `DefaultOSUtil.get_mount_point`.
///
/// Lines look like: `/dev/sr0 on /mnt/cdrom/secure type iso9660 (ro,relatime)`.
pub fn get_mount_point(mount_listing: &str, device: &Path) -> Option<PathBuf> {
    let needle = device.to_string_lossy();
    for line in mount_listing.lines() {
        if !line.contains(needle.as_ref()) {
            continue;
        }
        let mut tokens = line.split_whitespace();
        // Expect: <device> on <mount_point> type ...
        let _dev = tokens.next();
        let on = tokens.next();
        let mp = tokens.next();
        if on == Some("on") {
            if let Some(mp) = mp {
                return Some(PathBuf::from(mp));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dvd_pattern_matches_expected_devices() {
        for ok in ["sr0", "sr9", "hdc", "hdz", "cdrom0", "cd5", "vdb", "vdz"] {
            assert!(matches_dvd_pattern(ok), "expected match for {}", ok);
        }
        for bad in ["sda", "sda1", "sr10", "hda", "hdb", "vd", "vda", "loop0", ""] {
            assert!(!matches_dvd_pattern(bad), "expected non-match for {}", bad);
        }
    }

    #[test]
    fn parses_mount_listing() {
        let listing = "\
/dev/sda1 on / type ext4 (rw,relatime)
proc on /proc type proc (rw)
/dev/sr0 on /mnt/cdrom/secure type iso9660 (ro,relatime)
tmpfs on /run type tmpfs (rw,nosuid)
";
        assert_eq!(
            get_mount_point(listing, Path::new("/dev/sr0")),
            Some(PathBuf::from("/mnt/cdrom/secure"))
        );
        assert_eq!(get_mount_point(listing, Path::new("/dev/sr1")), None);
    }

    #[test]
    fn configured_mount_point_falls_back_to_default() {
        let cfg = Config::default();
        assert_eq!(configured_mount_point(&cfg), PathBuf::from("/mnt/cdrom/secure"));
    }

    #[test]
    fn get_dvd_device_finds_match_in_temp_dir() {
        let dir = std::env::temp_dir().join(format!("waagent_dvd_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for n in ["sda", "sda1", "sr0"] {
            fs::File::create(dir.join(n)).unwrap();
        }

        let found = get_dvd_device(&dir).unwrap();
        assert_eq!(found.file_name().unwrap(), "sr0");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn get_dvd_device_errors_when_no_match() {
        let dir = std::env::temp_dir().join(format!("waagent_dvd_test_none_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::File::create(dir.join("sda")).unwrap();

        let err = get_dvd_device(&dir).unwrap_err();
        assert!(matches!(err, DvdError::DeviceNotFound { .. }));

        fs::remove_dir_all(&dir).unwrap();
    }
}
