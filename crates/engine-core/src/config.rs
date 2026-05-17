use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct Config {
    data: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e))?;
        
        Self::from_toml(&content)
    }

    pub fn from_toml(toml_str: &str) -> Result<Self, ConfigError> {
        let mut config = Self::new();
        
        for line in toml_str.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                let value = value.trim_matches('"').trim_matches('\'');
                config.data.insert(key, value.to_string());
            }
        }
        
        Ok(config)
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }

    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.data.get(key).cloned().unwrap_or_else(|| default.to_string())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.data.get(key).and_then(|v| {
            match v.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            }
        })
    }

    pub fn get_i32(&self, key: &str) -> Option<i32> {
        self.data.get(key).and_then(|v| v.parse().ok())
    }

    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.data.get(key).and_then(|v| v.parse().ok())
    }

    pub fn set(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let mut content = String::new();
        
        for (key, value) in &self.data {
            content.push_str(&format!("{} = \"{}\"\n", key, value));
        }
        
        fs::write(path, content)
            .map_err(|e| ConfigError::IoError(e))?;
        
        Ok(())
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    ParseError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(s) => write!(f, "Parse error: {}", s),
        }
    }
}

impl std::error::Error for ConfigError {}
