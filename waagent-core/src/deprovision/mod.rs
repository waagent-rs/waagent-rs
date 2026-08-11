//! Deprovision (port of `azurelinuxagent/pa/deprovision/`).
//!
//! Builds a [`DeprovisionPlan`] of warnings + [`DeprovisionAction`]s that, when
//! executed, leave the VM in a state suitable for image capture. Mirrors the
//! design of WALinuxAgent's `DeprovisionHandler`: distro-specific subclasses
//! tweak the default plan; the engine handles confirmation, signal masking and
//! action invocation.

mod default;
mod factory;

pub mod distro;

pub use default::DefaultDeprovisionHandler;
pub use factory::{detect_distro, get_deprovision_handler, DistroId};

use std::fmt;
use std::io::{self, BufRead, Write};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tracing::{error, info, warn};

use crate::config::Config;
use crate::utils::fileutils;

/// A unit of work scheduled by a deprovision handler. Each variant maps to a
/// concrete operation performed during `do_actions`.
#[derive(Debug, Clone)]
pub enum DeprovisionAction {
    /// Remove the listed files/globs (missing entries are ignored).
    RmFiles(Vec<String>),
    /// Remove the listed directories recursively (missing entries ignored).
    RmDirs(Vec<String>),
    /// Reset the system hostname.
    SetHostname(String),
    /// Reset the DHCP-published hostname.
    SetDhcpHostname(String),
    /// Delete a local user account and its home directory.
    DelAccount(String),
    /// Disable the root account password.
    DelRootPassword,
    /// Stop the running waagent service.
    StopAgentService,
}

impl DeprovisionAction {
    fn invoke(&self) {
        match self {
            DeprovisionAction::RmFiles(paths) => fileutils::rm_files(paths),
            DeprovisionAction::RmDirs(paths) => fileutils::rm_dirs(paths),
            DeprovisionAction::SetHostname(name) => set_hostname(name),
            DeprovisionAction::SetDhcpHostname(name) => set_dhcp_hostname(name),
            DeprovisionAction::DelAccount(user) => del_account(user),
            DeprovisionAction::DelRootPassword => del_root_password(),
            DeprovisionAction::StopAgentService => stop_agent_service(),
        }
    }
}

impl fmt::Display for DeprovisionAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeprovisionAction::RmFiles(p) => write!(f, "rm files: {:?}", p),
            DeprovisionAction::RmDirs(p) => write!(f, "rm dirs: {:?}", p),
            DeprovisionAction::SetHostname(n) => write!(f, "set hostname: {}", n),
            DeprovisionAction::SetDhcpHostname(n) => write!(f, "set dhcp hostname: {}", n),
            DeprovisionAction::DelAccount(u) => write!(f, "delete account: {}", u),
            DeprovisionAction::DelRootPassword => write!(f, "disable root password"),
            DeprovisionAction::StopAgentService => write!(f, "stop agent service"),
        }
    }
}

/// Output of a deprovision handler's `setup`: ordered warnings to display to
/// the operator, plus the actions to execute on confirmation.
#[derive(Debug, Default, Clone)]
pub struct DeprovisionPlan {
    pub warnings: Vec<String>,
    pub actions: Vec<DeprovisionAction>,
}

impl DeprovisionPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn warn<S: Into<String>>(&mut self, msg: S) {
        self.warnings.push(msg.into());
    }

    pub fn push(&mut self, action: DeprovisionAction) {
        self.actions.push(action);
    }
}

/// Trait implemented by every distro-specific deprovision handler. The default
/// methods build the same plan as `DeprovisionHandler` in WALinuxAgent;
/// distro implementations override `setup` (calling [`DeprovisionEngine::base_setup`])
/// to add or replace steps.
pub trait DeprovisionHandler {
    fn setup(&self, config: &Config, deluser: bool) -> DeprovisionPlan {
        DeprovisionEngine::base_setup(config, deluser)
    }

    fn setup_changed_unique_id(&self, config: &Config) -> DeprovisionPlan {
        DeprovisionEngine::base_setup_changed_unique_id(config)
    }

    /// Full interactive run: build plan, print warnings, prompt for
    /// confirmation, then execute on `y`. Pass `force=true` to skip the prompt.
    fn run(&self, config: &Config, force: bool, deluser: bool) -> io::Result<()> {
        let plan = self.setup(config, deluser);
        let engine = DeprovisionEngine::new();
        engine.do_warnings(&plan);
        if engine.do_confirmation(force)? {
            engine.do_actions(&plan);
        } else {
            info!("deprovision cancelled by user");
        }
        Ok(())
    }

    /// Non-interactive cleanup invoked when the VM unique id changes.
    fn run_changed_unique_id(&self, config: &Config) {
        let plan = self.setup_changed_unique_id(config);
        let engine = DeprovisionEngine::new();
        engine.do_warnings(&plan);
        engine.do_actions(&plan);
    }
}

/// Engine that owns side-effecting concerns (signal handling, prompting,
/// invocation order). Kept separate so handlers stay declarative.
pub struct DeprovisionEngine {
    actions_running: Arc<AtomicBool>,
}

impl DeprovisionEngine {
    pub fn new() -> Self {
        Self {
            actions_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Build the plan that matches `DeprovisionHandler.setup` in WALinuxAgent.
    pub fn base_setup(config: &Config, deluser: bool) -> DeprovisionPlan {
        let mut plan = DeprovisionPlan::new();

        // 1. Stop the agent service.
        plan.warn("WARNING! The waagent service will be stopped.");
        plan.push(DeprovisionAction::StopAgentService);

        // 2. Regenerate SSH host keys when configured.
        if config_bool(config, "Provisioning.RegenerateSshHostKeyPair", false) {
            let ssh_dir = config_string(config, "OS.SshDir", "/etc/ssh");
            let key_type = config_string(config, "Provisioning.SshHostKeyPairType", "rsa");
            plan.warn("WARNING! All SSH host key pairs will be deleted.");
            plan.push(DeprovisionAction::RmFiles(vec![format!(
                "{}/ssh_host_{}_key*",
                ssh_dir.trim_end_matches('/'),
                key_type
            )]));
        }

        // 3. DHCP leases.
        del_dhcp_lease(&mut plan);

        // 4. Reset hostname.
        plan.push(DeprovisionAction::SetHostname("localhost.localdomain".into()));
        plan.push(DeprovisionAction::SetDhcpHostname(
            "localhost.localdomain".into(),
        ));

        // 5. Optionally disable the root password.
        if config_bool(config, "Provisioning.DeleteRootPassword", false) {
            plan.warn(
                "WARNING! root password will be disabled. You will not be able to login as root.",
            );
            plan.push(DeprovisionAction::DelRootPassword);
        }

        // 6. Lib & extension log directories.
        let lib_dir = config_string(config, "Lib.Dir", "/var/lib/waagent");
        let ext_log = config_string(config, "Extension.LogDir", "/var/log/azure");
        plan.push(DeprovisionAction::RmDirs(vec![lib_dir.clone(), ext_log]));

        // 7. Misc files.
        let log_file = config_string(config, "Logs.File", "/var/log/waagent.log");
        plan.push(DeprovisionAction::RmFiles(vec![
            "/root/.bash_history".into(),
            log_file,
        ]));
        // BSD-family random seed / IPsec keys (no-op on Linux).
        plan.push(DeprovisionAction::RmFiles(vec![
            "/etc/random.seed".into(),
            "/var/db/host.random".into(),
            "/etc/isakmpd/local.pub".into(),
            "/etc/isakmpd/private/local.key".into(),
            "/etc/iked/private/local.key".into(),
            "/etc/iked/local.pub".into(),
        ]));

        // 8. /etc/resolv.conf (distros override this).
        plan.warn("WARNING! /etc/resolv.conf will be deleted.");
        plan.push(DeprovisionAction::RmFiles(vec!["/etc/resolv.conf".into()]));

        // 9. Optionally remove the provisioned user account. Mirrors
        // `del_user` in WALA: read the username from the cached ovf-env.xml
        // (preferred) or from the freshly-mounted provisioning DVD.
        if deluser {
            match resolve_provisioned_username(config) {
                Ok(username) => {
                    plan.warn(format!(
                        "WARNING! {} account and entire home directory will be deleted.",
                        username
                    ));
                    plan.push(DeprovisionAction::DelAccount(username));
                }
                Err(reason) => {
                    plan.warn(format!(
                        "WARNING! ovf-env.xml is not available ({}). Skip delete user.",
                        reason
                    ));
                }
            }
        }

        // 10. Persisted firewall service unit (drop-in path mirrors WALA).
        plan.push(DeprovisionAction::RmFiles(vec![
            "/lib/systemd/system/waagent-network-setup.service".into(),
            "/etc/systemd/system/waagent-network-setup.service".into(),
            format!("{}/waagent-network-setup.py", lib_dir.trim_end_matches('/')),
        ]));

        // 11. Agent cgroup drop-in files.
        plan.push(DeprovisionAction::RmFiles(vec![
            "/etc/systemd/system/walinuxagent.service.d/10-Slice.conf".into(),
            "/etc/systemd/system/walinuxagent.service.d/11-CPUAccounting.conf".into(),
            "/etc/systemd/system/walinuxagent.service.d/12-CPUQuota.conf".into(),
            "/etc/systemd/system/walinuxagent.service.d/13-MemoryAccounting.conf".into(),
            "/etc/systemd/system/azure-walinuxagent-logcollector.slice".into(),
        ]));

        plan
    }

    /// Plan used by `run_changed_unique_id` (subset of the full setup).
    pub fn base_setup_changed_unique_id(config: &Config) -> DeprovisionPlan {
        let mut plan = DeprovisionPlan::new();
        del_dhcp_lease(&mut plan);
        del_lib_dir_files(&mut plan, config);
        del_ext_handler_files(&mut plan, config);
        // Persisted firewall + cgroup drop-ins, same as full setup tail.
        let lib_dir = config_string(config, "Lib.Dir", "/var/lib/waagent");
        plan.push(DeprovisionAction::RmFiles(vec![
            "/lib/systemd/system/waagent-network-setup.service".into(),
            "/etc/systemd/system/waagent-network-setup.service".into(),
            format!("{}/waagent-network-setup.py", lib_dir.trim_end_matches('/')),
        ]));
        plan.push(DeprovisionAction::RmFiles(vec![
            "/etc/systemd/system/walinuxagent.service.d/10-Slice.conf".into(),
            "/etc/systemd/system/walinuxagent.service.d/11-CPUAccounting.conf".into(),
            "/etc/systemd/system/walinuxagent.service.d/12-CPUQuota.conf".into(),
            "/etc/systemd/system/walinuxagent.service.d/13-MemoryAccounting.conf".into(),
            "/etc/systemd/system/azure-walinuxagent-logcollector.slice".into(),
        ]));
        plan
    }

    pub fn do_warnings(&self, plan: &DeprovisionPlan) {
        for w in &plan.warnings {
            println!("{}", w);
        }
    }

    pub fn do_confirmation(&self, force: bool) -> io::Result<bool> {
        if force {
            return Ok(true);
        }
        print!("Do you want to proceed (y/n) ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input)?;
        Ok(input.trim_start().to_lowercase().starts_with('y'))
    }

    pub fn do_actions(&self, plan: &DeprovisionPlan) {
        self.actions_running.store(true, Ordering::SeqCst);
        for action in &plan.actions {
            info!(target: "deprovision", "{}", action);
            action.invoke();
        }
        self.actions_running.store(false, Ordering::SeqCst);
    }

    /// Whether actions are currently executing — exposed so a SIGINT handler
    /// can decide whether interruption is safe (matches WALA's behavior).
    pub fn actions_running(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.actions_running)
    }
}

impl Default for DeprovisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers used by the default plan and distro overrides.
// ---------------------------------------------------------------------------

pub(crate) fn config_string(config: &Config, key: &str, fallback: &str) -> String {
    match config.get_value(key) {
        Some(crate::config::ConfigValue::String(s)) if s != "None" => s.clone(),
        _ => fallback.to_string(),
    }
}

pub(crate) fn config_bool(config: &Config, key: &str, fallback: bool) -> bool {
    match config.get_value(key) {
        Some(crate::config::ConfigValue::Bool(b)) => *b,
        _ => fallback,
    }
}

/// Locate the provisioned username for `--deluser`. We mirror WALA's
/// `protocol_util.get_ovf_env`: prefer the cached `ovf-env.xml` under
/// `Lib.Dir`, and fall back to mounting the provisioning DVD when missing.
fn resolve_provisioned_username(config: &Config) -> Result<String, String> {
    let lib_dir = config_string(config, "Lib.Dir", "/var/lib/waagent");
    let cached = std::path::PathBuf::from(lib_dir.trim_end_matches('/')).join("ovf-env.xml");
    if cached.is_file() {
        return crate::ovf::load(&cached)
            .map(|env| env.username)
            .map_err(|e| format!("failed to parse cached {}: {}", cached.display(), e));
    }

    let mounted = crate::dvd::mount_dvd(config, &crate::dvd::MountOptions::default())
        .map_err(|e| format!("could not mount provisioning DVD: {}", e))?;
    let path = mounted.mount_point.join("ovf-env.xml");
    let result = crate::ovf::load(&path)
        .map(|env| env.username)
        .map_err(|e| format!("failed to parse {}: {}", path.display(), e));

    // Only unmount what we mounted ourselves.
    if !mounted.already_mounted {
        if let Err(err) = crate::dvd::umount_dvd(config, Some(&mounted.mount_point)) {
            warn!("failed to unmount provisioning DVD: {}", err);
        }
    }
    result
}

fn del_dhcp_lease(plan: &mut DeprovisionPlan) {
    plan.warn("WARNING! Cached DHCP leases will be deleted.");
    plan.push(DeprovisionAction::RmDirs(vec![
        "/var/lib/dhclient".into(),
        "/var/lib/dhcpcd".into(),
        "/var/lib/dhcp".into(),
    ]));
    // BSD / NetworkManager / systemd-networkd lease files.
    plan.push(DeprovisionAction::RmFiles(vec![
        "/var/db/dhclient.leases.*".into(),
        "/var/lib/NetworkManager/dhclient-*.lease".into(),
        "/run/systemd/netif/leases/*".into(),
    ]));
}

fn del_lib_dir_files(plan: &mut DeprovisionPlan, config: &Config) {
    let lib_dir = config_string(config, "Lib.Dir", "/var/lib/waagent");
    let lib = lib_dir.trim_end_matches('/');
    let known = [
        "HostingEnvironmentConfig.xml",
        "Incarnation",
        "partition",
        "Protocol",
        "SharedConfig.xml",
        "WireServerEndpoint",
        "published_hostname",
        "fast_track.json",
        "initial_goal_state",
        "waagent_rsm_update",
        "waagent_initial_update",
    ];
    let mut files: Vec<String> = known.iter().map(|f| format!("{}/{}", lib, f)).collect();
    for pat in ["Extensions.*.xml", "ExtensionsConfig.*.xml", "GoalState.*.xml"] {
        files.push(format!("{}/{}", lib, pat));
    }
    plan.push(DeprovisionAction::RmFiles(files));
}

fn del_ext_handler_files(plan: &mut DeprovisionPlan, config: &Config) {
    let lib_dir = config_string(config, "Lib.Dir", "/var/lib/waagent");
    let lib = lib_dir.trim_end_matches('/');
    // Remove well-known per-extension state files via globs. Unlike the Python
    // implementation, this approximation does not scan subdirectories or
    // exclude agent self-update payload folders such as `WALinuxAgent-*`; it
    // simply matches the corresponding paths under every handler subdirectory.
    plan.push(DeprovisionAction::RmFiles(vec![
        format!("{}/*/status/*.status", lib),
        format!("{}/*/config/*.settings", lib),
        format!("{}/*/config/HandlerStatus", lib),
        format!("{}/*/mrseq", lib),
    ]));
}

// ---------------------------------------------------------------------------
// Side-effecting helpers (Linux-only). On other targets they degrade to a
// warning so the build stays green.
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn set_hostname(name: &str) {
    run("hostnamectl", &["set-hostname", name])
        .or_else(|| run("hostname", &[name]));
}

#[cfg(not(target_os = "linux"))]
fn set_hostname(name: &str) {
    warn!("set_hostname({}) skipped: not implemented on this platform", name);
}

#[cfg(target_os = "linux")]
fn set_dhcp_hostname(name: &str) {
    // Best-effort: write the new hostname into /etc/hostname so DHCP clients
    // pick it up at the next renewal. Distros with non-standard layouts (e.g.
    // FreeBSD's rc.conf) are out of scope for this port.
    if let Err(err) = std::fs::write("/etc/hostname", format!("{}\n", name)) {
        warn!("failed to update /etc/hostname: {}", err);
    }
}

#[cfg(not(target_os = "linux"))]
fn set_dhcp_hostname(name: &str) {
    warn!("set_dhcp_hostname({}) skipped: not implemented on this platform", name);
}

#[cfg(target_os = "linux")]
fn del_account(user: &str) {
    // Hard-fail safety: refuse to delete root or empty usernames.
    if user.is_empty() || user == "root" {
        error!("refusing to delete account '{}'", user);
        return;
    }
    run("userdel", &["-r", "-f", user]);
}

#[cfg(not(target_os = "linux"))]
fn del_account(user: &str) {
    warn!("del_account({}) skipped: not implemented on this platform", user);
}

#[cfg(target_os = "linux")]
fn del_root_password() {
    // `passwd -l root` locks the account; `usermod -p '!'` clears the hash.
    run("passwd", &["-l", "root"]);
}

#[cfg(not(target_os = "linux"))]
fn del_root_password() {
    warn!("del_root_password skipped: not implemented on this platform");
}

#[cfg(target_os = "linux")]
fn stop_agent_service() {
    // Try the new and legacy unit names; ignore failures since the unit may
    // not exist on minimal images.
    run("systemctl", &["stop", "waagent-rs.service"]);
    run("systemctl", &["stop", "walinuxagent.service"]);
}

#[cfg(not(target_os = "linux"))]
fn stop_agent_service() {
    warn!("stop_agent_service skipped: not implemented on this platform");
}

#[cfg(target_os = "linux")]
fn run(cmd: &str, args: &[&str]) -> Option<()> {
    match Command::new(cmd).args(args).status() {
        Ok(s) if s.success() => Some(()),
        Ok(s) => {
            warn!("{} {:?} exited with {}", cmd, args, s);
            None
        }
        Err(err) => {
            warn!("failed to spawn {}: {}", cmd, err);
            None
        }
    }
}
