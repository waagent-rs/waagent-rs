use super::{Config, ConfigValue, HashMap};

macro_rules! load_defaults_hashmap {
    ($hashmap:ident, {
        BOOL_OPTIONS: {$($bool_key:literal => $bool_value:expr),* $(,)?},
        STRING_OPTIONS: {$($str_key:literal => $str_value:expr),* $(,)?},
        INTEGER_OPTIONS: {$($int_key:literal => $int_value:expr),* $(,)?},
        PORT_OPTIONS: {$($port_key:literal => $port_value:expr),* $(,)?}
    }) => {
        $($hashmap.insert($bool_key.to_string(), ConfigValue::Bool($bool_value));)*
        $($hashmap.insert($str_key.to_string(), ConfigValue::String($str_value.to_string()));)*
        $($hashmap.insert($int_key.to_string(), ConfigValue::Integer($int_value));)*
        $($hashmap.insert($port_key.to_string(), ConfigValue::Port($port_value));)*
    };
}

pub const NONE_STR: &str = "None";

impl Default for Config {
    fn default() -> Self {
        Self {
            config: get_config_defaults(),
        }
    }
}

#[rustfmt::skip]
pub fn get_config_defaults() -> HashMap<String,ConfigValue> {
    let mut defaults = HashMap::new();
    let quota = 30 * (1024_u32.pow(2));

    load_defaults_hashmap!(defaults, {
        BOOL_OPTIONS: {
            "OS.AllowHTTP" => false,
            "OS.EnableFirewall" => false,
            "OS.EnableFIPS" => false,
            "OS.EnableRDMA" => false,
            "OS.UpdateRdmaDriver" => false,
            "OS.CheckRdmaDriver" => false,
            "Logs.Verbose" => false,
            "Logs.Console" => true,
            "Logs.Collect" => true,
            "Extensions.Enabled" => true,
            "Extensions.WaitForCloudInit" => false,
            "Provisioning.AllowResetSysUser" => false,
            "Provisioning.RegenerateSshHostKeyPair" => false,
            "Provisioning.DeleteRootPassword" => false,
            "Provisioning.DecodeCustomData" => false,
            "Provisioning.ExecuteCustomData" => false,
            "Provisioning.MonitorHostName" => false,
            "DetectScvmmEnv" => false,
            "ResourceDisk.Format" => false,
            "ResourceDisk.EnableSwap" => false,
            "ResourceDisk.EnableSwapEncryption" => false,
            "AutoUpdate.Enabled" => true,
            "AutoUpdate.UpdateToLatestVersion" => true,
            "EnableOverProvisioning" => true,
            //
            // "Debug" options are experimental and may be removed in later
            // versions of the Agent.
            //
            "Debug.CgroupLogMetrics" => false,
            "Debug.CgroupDisableOnProcessCheckFailure" => true,
            "Debug.CgroupDisableOnQuotaCheckFailure" => true,
            "Debug.EnableAgentMemoryUsageCheck" => false,
            "Debug.EnableFastTrack" => true,
            "Debug.EnableGAVersioning" => true,
            "Debug.EnableCgroupV2ResourceLimiting" => false,
            "Debug.EnableExtensionPolicy" => false
        },
        STRING_OPTIONS: {
            "Lib.Dir" => "/var/lib/waagent",
            "DVD.MountPoint" => "/mnt/cdrom/secure",
            "Pid.File" => "/var/run/waagent.pid",
            "Extension.LogDir" => "/var/log/azure",
            "OS.OpensslPath" => "/usr/bin/openssl",
            "OS.SshDir" => "/etc/ssh",
            "OS.HomeDir" => "/home",
            "OS.PasswordPath" => "/etc/shadow",
            "OS.SudoersDir" => "/etc/sudoers.d",
            "OS.RootDeviceScsiTimeout" => NONE_STR,
            "Provisioning.Agent" => "auto",
            "Provisioning.SshHostKeyPairType" => "rsa",
            "Provisioning.PasswordCryptId" => "6",
            "HttpProxy.Host" => NONE_STR,
            "ResourceDisk.MountPoint" => "/mnt/resource",
            "ResourceDisk.MountOptions" => NONE_STR,
            "ResourceDisk.Filesystem" => "ext3",
            "AutoUpdate.GAFamily" => "Prod",
            "Policy.PolicyFilePath" => "/etc/waagent_policy.json",
            "Protocol.EndpointDiscovery" => "dhcp"
        },
        INTEGER_OPTIONS: {
            "Extensions.GoalStatePeriod" => 6,
            "Extensions.InitialGoalStatePeriod" => 6,
            "Extensions.WaitForCloudInitTimeout" => 3600,
            "OS.EnableFirewallPeriod" => 300,
            "OS.RemovePersistentNetRulesPeriod" => 30,
            "OS.RootDeviceScsiTimeoutPeriod" => 30,
            "OS.MonitorDhcpClientRestartPeriod" => 30,
            "OS.SshClientAliveInterval" => 180,
            "Provisioning.MonitorHostNamePeriod" => 30,
            "Provisioning.PasswordCryptSaltLength" => 10,
            "ResourceDisk.SwapSizeMB" => 0,
            "Autoupdate.Frequency" => 3600,
            "Logs.CollectPeriod" => 3600,
            //
            //  "Debug" options are experimental and may be removed in later
            //  versions of the Agent.
            //
            "Debug.CgroupCheckPeriod" => 300,
            "Debug.AgentCpuQuota" => 50,
            "Debug.AgentCpuThrottledTimeThreshold" => 120,
            "Debug.AgentMemoryQuota" => quota, // 30 MiB is the max memory config can take https://github.com/Azure/WALinuxAgent/issues/2931
            "Debug.EtpCollectionPeriod" => 300,
            "Debug.AutoUpdateHotfixFrequency" => 14400,
            "Debug.AutoUpdateNormalFrequency" => 86400,
            "Debug.FirewallRulesLogPeriod" => 86400,
            "Debug.LogCollectorInitialDelay" => 5 * 60
        },
        PORT_OPTIONS: {
            "HttpProxy.Port" => None,
        }
    });

    defaults
}
