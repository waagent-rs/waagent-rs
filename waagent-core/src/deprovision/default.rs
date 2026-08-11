//! Default (generic) deprovision handler — equivalent to
//! `azurelinuxagent.pa.deprovision.default.DeprovisionHandler`.

use super::DeprovisionHandler;

/// Generic deprovision handler used when no distro-specific override applies.
pub struct DefaultDeprovisionHandler;

impl DefaultDeprovisionHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultDeprovisionHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl DeprovisionHandler for DefaultDeprovisionHandler {}
