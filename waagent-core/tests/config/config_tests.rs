use std::path::Path;
use waagent_core::config::{Config, ConfigSchema, ConfigValue, ExpectedType};

#[test]
fn test_default_config_has_all_required_keys() {
    let default_config = Config::default();
    let schema_config = ConfigSchema::new();
    assert!(schema_config
        .schema()
        .keys()
        .all(|k| default_config.config().contains_key(k)));
}

#[test]
fn test_default_config_matches_schema_types() {
    let default_config = Config::default();
    let schema_config = ConfigSchema::new();

    for (key, value) in schema_config.schema() {
        let default_value = match default_config.get_value(key) {
            Some(&ConfigValue::Bool(_)) => &ExpectedType::Bool,
            Some(&ConfigValue::String(_)) => &ExpectedType::String,
            Some(&ConfigValue::Integer(_)) => &ExpectedType::Integer,
            Some(&ConfigValue::Port(_)) => &ExpectedType::Port,
            None => panic!("Missing default config value for key: {}", key),
        };
        assert_eq!(
            value, default_value,
            "Type mismatch for key: {}. Expected: {:?}, Got: {:?}",
            key, value, default_value
        );
    }
}

#[test]
fn test_show_config_output_from_file() {
    let test_config_path: &Path = Path::new("tests/config/data/waagent-test.conf");
    let config = Config::from_file(test_config_path);
    let output = config
        .expect(&format!("Missing file at {:?}", test_config_path))
        .show();

    let schema = ConfigSchema::new();
    let schema_count = schema.schema().keys().count();
    let output_count = output.lines().count();

    assert!(output.contains("Extensions.Enabled = true"));
    assert!(output.contains("Provisioning.Agent = auto"));
    assert!(output.contains("Extension.LogDir = /var/log/azure"));
    assert!(!output.contains("FauxKey1 = Value1"));
    assert!(!output.contains("key = value"));
    assert!(!output.contains("ResourceDisk.MountPoint = /mnt/resource"));
    assert_eq!(schema_count, output_count);
}

#[test]
#[rustfmt::skip]
fn test_config_file_example() {
    let test_config_path: &Path = Path::new("tests/config/data/waagent-test.conf");
    let config = Config::from_file(test_config_path).expect(&format!("Missing file at {:?}",  test_config_path));
    
    assert_ne!(config.get_value(""), Some(&(ConfigValue::String("Value0".to_string()))));
    assert_ne!(config.get_value("FauxKey1"), Some(&ConfigValue::String("Value1".to_string())));
    assert_ne!(config.get_value("FauxKey2"), Some(&ConfigValue::String("Value2 Value2".to_string())));
    assert_ne!(config.get_value("FauxKey3"), Some(&ConfigValue::String("delalloc,rw,noatime,nobarrier,users,mode=777".to_string())));
    assert_ne!(config.get_value(""), Some(&ConfigValue::String("".to_string())));
    assert_ne!(config.get_value("key"), Some(&ConfigValue::String("value".to_string())));

    assert_eq!(config.get_value("Extensions.Enabled"), Some(&ConfigValue::Bool(true)));
    assert_eq!(config.get_value("Provisioning.Agent"), Some(&ConfigValue::String("auto".to_string())));
    assert_eq!(config.get_value("Provisioning.DeleteRootPassword"), Some(&ConfigValue::Bool(true)));
    assert_eq!(config.get_value("Provisioning.RegenerateSshHostKeyPair"), Some(&ConfigValue::Bool(true)));
    assert_eq!(config.get_value("Provisioning.SshHostKeyPairType"), Some(&ConfigValue::String("rsa".to_string())));
    assert_eq!(config.get_value("Provisioning.MonitorHostName"), Some(&ConfigValue::Bool(true)));
    assert_eq!(config.get_value("Provisioning.DecodeCustomData"), Some(&ConfigValue::Bool(false)));
    assert_eq!(config.get_value("Provisioning.ExecuteCustomData"), Some(&ConfigValue::Bool(false)));
    assert_eq!(config.get_value("Provisioning.AllowResetSysUser"), Some(&ConfigValue::Bool(false)));
    assert_eq!(config.get_value("ResourceDisk.Format"), Some(&ConfigValue::Bool(true)));
    assert_eq!(config.get_value("ResourceDisk.Filesystem"), Some(&ConfigValue::String("ext4".to_string())));
    assert_eq!(config.get_value("ResourceDisk.MountPoint"), Some(&ConfigValue::String("/mnt".to_string())));
    assert_eq!(config.get_value("ResourceDisk.EnableSwap"), Some(&ConfigValue::Bool(false)));
    assert_eq!(config.get_value("ResourceDisk.EnableSwapEncryption"), Some(&ConfigValue::Bool(false)));
    assert_eq!(config.get_value("ResourceDisk.SwapSizeMB"), Some(&ConfigValue::Integer(0)));
    assert_eq!(config.get_value("ResourceDisk.MountOptions"), Some(&ConfigValue::String("None".to_string())));
    assert_eq!(config.get_value("Logs.Verbose"), Some(&ConfigValue::Bool(false)));
    assert_eq!(config.get_value("Logs.Collect"), Some(&ConfigValue::Bool(true)));
    assert_eq!(config.get_value("Logs.CollectPeriod"), Some(&ConfigValue::Integer(3600)));
    assert_eq!(config.get_value("OS.EnableFIPS"), Some(&ConfigValue::Bool(true)));
    assert_eq!(config.get_value("OS.RootDeviceScsiTimeout"), Some(&ConfigValue::String("300".to_string())));
    assert_eq!(config.get_value("OS.OpensslPath"), Some(&ConfigValue::String("None".to_string())));
    assert_eq!(config.get_value("OS.SshClientAliveInterval"), Some(&ConfigValue::Integer(42)));
    assert_eq!(config.get_value("OS.SshDir"), Some(&ConfigValue::String("/notareal/path".to_string())));
    assert_eq!(config.get_value("OS.EnableFirewall"), Some(&ConfigValue::Bool(false)));
    assert_eq!(config.get_value("Debug.EnableExtensionPolicy"), Some(&ConfigValue::Bool(false)));

    // check defaults merged
    assert_eq!(config.get_value("Lib.Dir"), Some(&ConfigValue::String("/var/lib/waagent".to_string())));
    assert_eq!(config.get_value("DetectScvmmEnv"), Some(&ConfigValue::Bool(false)));
    assert_eq!(config.get_value("OS.HomeDir"), Some(&ConfigValue::String("/home".to_string())));
}
