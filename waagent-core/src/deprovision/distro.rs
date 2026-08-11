//! Distro-specific deprovision handlers (port of
//! `azurelinuxagent/pa/deprovision/{ubuntu,arch,coreos,clearlinux}.py`).

use std::path::Path;

use super::{DeprovisionAction, DeprovisionEngine, DeprovisionHandler, DeprovisionPlan};
use crate::config::Config;

// ---------------------------------------------------------------------------
// Ubuntu (pre-18.04): replace `del_resolv` with the resolvconf-aware variant.
// ---------------------------------------------------------------------------
pub struct UbuntuDeprovisionHandler;

impl UbuntuDeprovisionHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UbuntuDeprovisionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DeprovisionHandler for UbuntuDeprovisionHandler {
    fn setup(&self, config: &Config, deluser: bool) -> DeprovisionPlan {
        let mut plan = DeprovisionEngine::base_setup(config, deluser);
        retarget_resolv_for_ubuntu(&mut plan);
        plan
    }
}

// ---------------------------------------------------------------------------
// Ubuntu >= 18.04: do not touch /etc/resolv.conf at all.
// ---------------------------------------------------------------------------
pub struct Ubuntu1804DeprovisionHandler;

impl Ubuntu1804DeprovisionHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Ubuntu1804DeprovisionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DeprovisionHandler for Ubuntu1804DeprovisionHandler {
    fn setup(&self, config: &Config, deluser: bool) -> DeprovisionPlan {
        let mut plan = DeprovisionEngine::base_setup(config, deluser);
        drop_resolv_steps(&mut plan);
        plan.warn(
            "WARNING! /etc/resolv.conf will NOT be removed; this is a behavior change \
             from earlier versions of Ubuntu.",
        );
        plan
    }
}

// ---------------------------------------------------------------------------
// Arch / CoreOS / Flatcar: also wipe /etc/machine-id.
// ---------------------------------------------------------------------------
pub struct ArchDeprovisionHandler;

impl ArchDeprovisionHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ArchDeprovisionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DeprovisionHandler for ArchDeprovisionHandler {
    fn setup(&self, config: &Config, deluser: bool) -> DeprovisionPlan {
        let mut plan = DeprovisionEngine::base_setup(config, deluser);
        plan.warn("WARNING! /etc/machine-id will be removed.");
        plan.push(DeprovisionAction::RmFiles(vec!["/etc/machine-id".into()]));
        plan
    }
}

pub struct CoreOsDeprovisionHandler;

impl CoreOsDeprovisionHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CoreOsDeprovisionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DeprovisionHandler for CoreOsDeprovisionHandler {
    fn setup(&self, config: &Config, deluser: bool) -> DeprovisionPlan {
        let mut plan = DeprovisionEngine::base_setup(config, deluser);
        plan.warn("WARNING! /etc/machine-id will be removed.");
        plan.push(DeprovisionAction::RmFiles(vec!["/etc/machine-id".into()]));
        plan
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Replace the generic `/etc/resolv.conf` removal with the Ubuntu-specific
/// rewrite that respects resolvconf-managed setups.
fn retarget_resolv_for_ubuntu(plan: &mut DeprovisionPlan) {
    drop_resolv_steps(plan);

    let resolv_is_managed = Path::new("/etc/resolv.conf")
        .read_link()
        .map(|t| t == Path::new("/run/resolvconf/resolv.conf"))
        .unwrap_or(false);

    if !resolv_is_managed {
        plan.warn("WARNING! /etc/resolv.conf will be deleted.");
        plan.push(DeprovisionAction::RmFiles(vec!["/etc/resolv.conf".into()]));
    } else {
        plan.warn(
            "WARNING! /etc/resolvconf/resolv.conf.d/tail and \
             /etc/resolvconf/resolv.conf.d/original will be deleted.",
        );
        plan.push(DeprovisionAction::RmFiles(vec![
            "/etc/resolvconf/resolv.conf.d/tail".into(),
            "/etc/resolvconf/resolv.conf.d/original".into(),
        ]));
    }
}

/// Strip the warnings and actions that the base plan emits for /etc/resolv.conf
/// so a distro override can substitute its own behavior.
fn drop_resolv_steps(plan: &mut DeprovisionPlan) {
    plan.warnings
        .retain(|w| !w.contains("/etc/resolv.conf"));
    plan.actions.retain(|a| match a {
        DeprovisionAction::RmFiles(files) => !files.iter().any(|f| f == "/etc/resolv.conf"),
        _ => true,
    });
}
