use super::HashMap;
use crate::config::defaults::get_config_defaults;

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigValue {
    Bool(bool),
    String(String),
    Integer(u32), // config values shouldn't be negative and are probably not greater than u32::MAX
    Port(Option<u16>), //u16 because ports 2^16 = 0-65535
}

#[derive(Debug, PartialEq)]
pub enum ExpectedType {
    Bool,
    Integer,
    String,
    Port,
}

pub struct Config {
    pub config: HashMap<String, ConfigValue>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn config(&self) -> &HashMap<String, ConfigValue> {
        &self.config
    }

    pub fn get_value(&self, key: &str) -> Option<&ConfigValue> {
        self.config.get(key)
    }

    pub fn from_map(hashmap: HashMap<String, ConfigValue>) -> Self {
        Self { config: hashmap }
    }

    #[rustfmt::skip]
    pub fn merge_with_defaults(hashmap: HashMap<String, ConfigValue>) -> HashMap<String, ConfigValue> {
        let mut merged = get_config_defaults();
        merged.extend(hashmap);

        merged
    }
}
