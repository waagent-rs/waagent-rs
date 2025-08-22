use super::{ExpectedType, HashMap};

macro_rules! load_schema_hashmap {
    ($hashmap:ident, {
        BOOL_TYPES: {$($bool_key:literal),* $(,)?},
        STRING_TYPES: {$($str_key:literal),* $(,)?},
        INTEGER_TYPES: {$($int_key:literal),* $(,)?},
        PORT_TYPES: {$($port_key:literal),* $(,)?}
    }) => {
        $($hashmap.insert($bool_key.to_string(), ExpectedType::Bool);)*
        $($hashmap.insert($str_key.to_string(), ExpectedType::String);)*
        $($hashmap.insert($int_key.to_string(), ExpectedType::Integer);)*
        $($hashmap.insert($port_key.to_string(), ExpectedType::Port);)*
    };
}

pub struct ConfigSchema {
    schema: HashMap<String, ExpectedType>,
}

impl ConfigSchema {
    pub fn new() -> Self {
        Self {
            schema: get_config_schema(),
        }
    }

    pub fn schema(&self) -> &HashMap<String, ExpectedType> {
        &self.schema
    }

    pub fn get_expected_type(&self, key: &str) -> Option<&ExpectedType> {
        self.schema.get(key)
    }

    pub fn is_valid_key(&self, key: &str) -> bool {
        self.schema.contains_key(key)
    }

    pub fn expected_types(&self) -> std::collections::hash_map::Values<'_, String, ExpectedType> {
        self.schema.values()
    }
}

#[rustfmt::skip]
fn get_config_schema() -> HashMap<String, ExpectedType> {
    let mut schema = HashMap::new();
    load_schema_hashmap! (schema, {
        BOOL_TYPES: {
            "OS.AllowHTTP",
            "OS.EnableFirewall",
            "OS.EnableFIPS",
            "OS.EnableRDMA",
            "OS.UpdateRdmaDriver",
            "OS.CheckRdmaDriver",
            "Logs.Verbose",
            "Logs.Console",
            "Logs.Collect",
            "Extensions.Enabled",
            "Extensions.WaitForCloudInit",
            "Provisioning.AllowResetSysUser",
            "Provisioning.RegenerateSshHostKeyPair",
            "Provisioning.DeleteRootPassword",
            "Provisioning.DecodeCustomData",
            "Provisioning.ExecuteCustomData",
            "Provisioning.MonitorHostName",
            "DetectScvmmEnv",
            "ResourceDisk.Format",
            "ResourceDisk.EnableSwap",
            "ResourceDisk.EnableSwapEncryption",
            "AutoUpdate.Enabled",
            "AutoUpdate.UpdateToLatestVersion",
            "EnableOverProvisioning",
            //
            // "Debug" options are experimental and may be removed in later
            // versions of the Agent.
            //
            "Debug.CgroupLogMetrics",
            "Debug.CgroupDisableOnProcessCheckFailure",
            "Debug.CgroupDisableOnQuotaCheckFailure",
            "Debug.EnableAgentMemoryUsageCheck",
            "Debug.EnableFastTrack",
            "Debug.EnableGAVersioning",
            "Debug.EnableCgroupV2ResourceLimiting",
            "Debug.EnableExtensionPolicy",
        },
        STRING_TYPES: {
            "Lib.Dir",
            "DVD.MountPoint",
            "Pid.File",
            "Extension.LogDir",
            "OS.OpensslPath",
            "OS.SshDir",
            "OS.HomeDir",
            "OS.PasswordPath",
            "OS.SudoersDir",
            "OS.RootDeviceScsiTimeout",
            "Provisioning.Agent",
            "Provisioning.SshHostKeyPairType",
            "Provisioning.PasswordCryptId",
            "HttpProxy.Host",
            "ResourceDisk.MountPoint",
            "ResourceDisk.MountOptions",
            "ResourceDisk.Filesystem",
            "AutoUpdate.GAFamily",
            "Policy.PolicyFilePath",
            "Protocol.EndpointDiscovery",
        },
        INTEGER_TYPES: {
            "Extensions.GoalStatePeriod",
            "Extensions.InitialGoalStatePeriod",
            "Extensions.WaitForCloudInitTimeout",
            "OS.EnableFirewallPeriod",
            "OS.RemovePersistentNetRulesPeriod",
            "OS.RootDeviceScsiTimeoutPeriod",
            "OS.MonitorDhcpClientRestartPeriod",
            "OS.SshClientAliveInterval",
            "Provisioning.MonitorHostNamePeriod",
            "Provisioning.PasswordCryptSaltLength",
            "ResourceDisk.SwapSizeMB",
            "Autoupdate.Frequency",
            "Logs.CollectPeriod",
            //
            //  "Debug" options are experimental and may be removed in later
            //  versions of the Agent.
            //
            "Debug.CgroupCheckPeriod",
            "Debug.AgentCpuQuota",
            "Debug.AgentCpuThrottledTimeThreshold",
            "Debug.AgentMemoryQuota",
            "Debug.EtpCollectionPeriod",
            "Debug.AutoUpdateHotfixFrequency",
            "Debug.AutoUpdateNormalFrequency",
            "Debug.FirewallRulesLogPeriod",
            "Debug.LogCollectorInitialDelay",
        },
        PORT_TYPES: {
            "HttpProxy.Port",

        }
        });

    schema
}
