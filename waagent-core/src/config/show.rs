use super::{Config, ConfigValue};
use std::collections::BTreeMap;

impl Config {
    pub fn show(self) -> String {
        let merged = Self::merge_with_defaults(self.config().clone());
        let sorted: BTreeMap<_, _> = merged.into_iter().collect();
        let mut output = String::new();

        for (key, v) in sorted {
            let value = match v {
                ConfigValue::Bool(bool) => bool.to_string(),
                ConfigValue::Integer(int) => int.to_string(),
                ConfigValue::String(string) => string.clone(),
                ConfigValue::Port(Some(port)) => port.to_string(),
                ConfigValue::Port(None) => "None".to_string(),
            };
            output.push_str(&format!("{} = {}\n", key, value));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, ConfigValue};
    use std::collections::HashMap;

    #[test]
    fn test_show_merges_defaults() {
        let mut user_config = HashMap::new();
        user_config.insert("OS.AllowHTTP".to_string(), ConfigValue::Bool(true));

        let config = Config::from_map(user_config);
        let output = config.show();

        // a value provided by user
        assert!(output.contains("OS.AllowHTTP = true"));
        // a default value
        assert!(output.contains("OS.EnableFirewall = false"));
    }

    #[test]
    fn test_show_is_sorted() {
        let config = Config::default();
        let output = config.show();
        let lines: Vec<&str> = output.lines().collect();

        // Check sort
        let mut sorted = lines.clone();
        sorted.sort();
        assert_eq!(lines, sorted);
    }
}
