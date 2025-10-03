use super::defaults::NONE_STR;
use super::{Config, ConfigSchema, ConfigValue, ExpectedType, HashMap};
use crate::utils::fileutils::read_file;
use std::path::Path;

impl Config {
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        // This will return Err if read_file returns Err, which would be due to open()
        // https://doc.rust-lang.org/stable/std/fs/struct.OpenOptions.html#method.open
        // will also fail if the data in this stream is not valid UTF-8 then an error is returned and buf is unchanged
        // caller will need to handle errors.
        let data: String = read_file(path)?;
        let parsed_data = Self::parse(&data);

        Ok(Self {
            config: Self::merge_with_defaults(parsed_data),
        })
    }

    fn parse(data: &str) -> HashMap<String, ConfigValue> {
        let mut values = HashMap::new();
        let schema = ConfigSchema::new();
        let defaults = Config::default();

        for line in data.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (key, value) = match split_key_value(line) {
                Some(pair) => pair,
                None => continue,
            };

            if !schema.is_valid_key(&key) {
                continue;
            }

            let expected_type = schema.get_expected_type(&key);
            let config_value = parse_config_value(&key, &value, expected_type, &defaults);

            if let Some(config_value) = config_value {
                values.insert(key.to_string(), config_value);
            }
        }

        values
    }
}

fn split_key_value(line: &str) -> Option<(String, String)> {
    // filter in-line comments
    let line = line.split('#').next()?.trim();

    let mut parts = line.splitn(2, '=');
    let key = parts.next()?.trim();
    if key.is_empty() {
        return None;
    }
    let value = parts.next().unwrap_or("").trim();

    Some((key.to_string(), value.to_string()))
}

fn parse_config_value(
    key: &str,
    value: &str,
    expected_type: Option<&ExpectedType>,
    defaults: &Config,
) -> Option<ConfigValue> {
    match key {
        "HttpProxy.Port" => {
            parse_port_value(value).or_else(|| fallback_to_default(key, defaults))
        }
        _ if expected_type == Some(&ExpectedType::Bool) => {
            parse_bool_value(value).or_else(|| fallback_to_default(key, defaults))
        }
        _ if expected_type == Some(&ExpectedType::String) => {
            parse_string_value(value).or_else(|| fallback_to_default(key, defaults))
        }
        _ if expected_type == Some(&ExpectedType::Integer) => {
            parse_integer_value(value).or_else(|| fallback_to_default(key, defaults))
        }
        _ => None,
    }
}

fn parse_bool_value(value: &str) -> Option<ConfigValue> {
    match value {
        "y" | "Y" => Some(ConfigValue::Bool(true)),
        "n" | "N" => Some(ConfigValue::Bool(false)),
        _ => None,
    }
}

fn parse_string_value(value: &str) -> Option<ConfigValue> {
    match value {
        "\"\"" => Some(ConfigValue::String(String::from(NONE_STR))),
        v if !v.is_empty() => Some(ConfigValue::String(v.to_string())),
        _ => None,
    }
}

fn parse_integer_value(value: &str) -> Option<ConfigValue> {
    value.parse::<u32>().ok().map(ConfigValue::Integer)
}

fn parse_port_value(value: &str) -> Option<ConfigValue> {
    if value == NONE_STR || value.is_empty() {
        Some(ConfigValue::Port(None))
    } else {
        match value.parse::<u16>() {
            Ok(port) => Some(ConfigValue::Port(Some(port))),
            _ => None,
        }
    }
}

fn fallback_to_default(key: &str, defaults: &Config) -> Option<ConfigValue> {
    defaults.get_value(key).cloned()
}

#[cfg(test)]
mod tests {
    use crate::config::types::{Config, ConfigValue};
    #[test]
    fn test_parse_bool() {
        let input = "Extensions.Enabled=y";
        let parsed_input = Config::parse(input);

        assert_eq!(
            parsed_input.get("Extensions.Enabled"),
            Some(&ConfigValue::Bool(true))
        )
    }

    #[test]
    fn test_parse_invalid_bool() {
        let input = "Extensions.Enabled=maybe";
        let parsed = Config::parse(input);

        let default = Config::default();
        let expected = default.get_value("Extensions.Enabled");

        assert_eq!(parsed.get("Extensions.Enabled"), expected);
    }

    #[test]
    fn test_parse_int() {
        let input = "OS.SshClientAliveInterval=6";
        let parsed_input = Config::parse(input);

        assert_eq!(
            parsed_input.get("OS.SshClientAliveInterval"),
            Some(&ConfigValue::Integer(6))
        )
    }

    #[test]
    fn test_parse_str() {
        let input = "ResourceDisk.MountPoint=/mnt/resource";
        let parsed_input = Config::parse(input);

        assert_eq!(
            parsed_input.get("ResourceDisk.MountPoint"),
            Some(&ConfigValue::String(String::from("/mnt/resource")))
        )
    }

    #[test]
    fn test_parse_mixed() {
        let input = "# Allow fallback to HTTP if HTTPS is unavailable\n# Note: Allowing HTTP (vs. HTTPS) may cause security risks\n# OS.AllowHTTP=n\n\n# Add firewall rules to protect access to Azure host node services\nOS.EnableFirewall=n\n\n# How often (in seconds) to check the firewall rules";
        let parsed_input = Config::parse(input);

        assert_eq!(
            parsed_input.get("OS.EnableFirewall"),
            Some(&ConfigValue::Bool(false))
        )
    }

    #[test]
    fn test_parse_with_whitespace() {
        let input = "\r\n                 Provisioning.ExecuteCustomData = n       \t";
        let parsed_input = Config::parse(input);

        assert_eq!(
            parsed_input.get("Provisioning.ExecuteCustomData"),
            Some(&ConfigValue::Bool(false))
        )
    }

    #[test]
    fn test_ignores_empty() {
        let input = "\n";
        let parsed_input = Config::parse(input);

        assert_eq!(parsed_input.len(), 0);
    }

    #[test]
    fn test_ignores_comments() {
        let input = "# If set, agent will use proxy server to access internet";
        let parsed_input = Config::parse(input);

        assert_eq!(parsed_input.len(), 0);
        assert!(parsed_input.is_empty(), "Should ignore comments")
    }

    #[test]
    fn test_edge_parse_missing_key() {
        let input = "=y";
        let parsed_input = Config::parse(input);

        assert_eq!(parsed_input.len(), 0);
    }

    #[test]
    fn test_edge_parse_missing_value() {
        let input = "ResourceDisk.SwapSizeMB=";
        let parsed_input = Config::parse(input);

        assert_eq!(parsed_input.len(), 1);
        assert_eq!(
            parsed_input.get("ResourceDisk.SwapSizeMB"),
            Some(&ConfigValue::Integer(0))
        );
    }

    #[test]
    fn test_edge_parse_missing_key_value() {
        let input = " = ";
        let parsed_input = Config::parse(input);

        assert_eq!(parsed_input.len(), 0);
    }

    #[test]
    fn test_edge_parse_multi_value() {
        let input = "ResourceDisk.SwapSizeMB=0,200,30";
        let parsed_input = Config::parse(input);

        assert_eq!(parsed_input.len(), 1);
        assert_eq!(
            parsed_input.get("ResourceDisk.SwapSizeMB"),
            Some(&ConfigValue::Integer(0))
        );
    }

    #[test]
    fn test_edge_parse_invalid_key_only() {
        let input = "Fake.key";
        let parsed_input = Config::parse(input);

        assert_eq!(parsed_input.len(), 0);
    }

    #[test]
    fn test_edge_parse_valid_key_only() {
        let input = "Lib.Dir";
        let parsed_input = Config::parse(input);

        assert_eq!(parsed_input.len(), 1);
        assert_eq!(
            parsed_input.get("Lib.Dir"),
            Some(&ConfigValue::String("/var/lib/waagent".to_string()))
        )
    }

    #[test]
    fn test_edge_parse_multiple_equals_signs() {
        let input = "Provisioning.Agent=auto=default";
        let parsed = Config::parse(input);

        assert_eq!(
            parsed.get("Provisioning.Agent"),
            Some(&ConfigValue::String(String::from("auto=default")))
        );
    }

    #[test]
    fn test_edge_parse_duplicate_keys_last_wins() {
        let input = "Logs.Verbose=n\nLogs.Verbose=y";
        let parsed = Config::parse(input);

        assert_eq!(parsed.get("Logs.Verbose"), Some(&ConfigValue::Bool(true)));
    }

    #[test]
    fn test_edge_parse_inline_comment_in_value() {
        let input = "OS.EnableFirewall=y # turn on firewall";
        let parsed = Config::parse(input);

        assert_eq!(
            parsed.get("OS.EnableFirewall"),
            Some(&ConfigValue::Bool(true))
        );
    }

    #[test]
    fn test_edge_parse_utf8_string_value() {
        let input = "Provisioning.Agent=μcloud-init";
        let parsed = Config::parse(input);

        assert_eq!(
            parsed.get("Provisioning.Agent"),
            Some(&ConfigValue::String("μcloud-init".to_string()))
        );
    }
}
