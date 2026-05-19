//! Network plugin for engine.
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
/// Plugin that adds networking capabilities.
pub struct NetworkPlugin;

/// Network system configuration.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub is_server: bool,
    pub server_address: String,
    pub port: u16,
    pub max_connections: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            is_server: false,
            server_address: "127.0.0.1".to_string(),
            port: 7777,
            max_connections: 32,
        }
    }
}

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // Add network config as a resource
        let world = app.world_mut();
        world.insert_resource(NetworkConfig::default());
        
        // If server would add systems to process network messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert!(!config.is_server);
        assert_eq!(config.port, 7777);
    }
}
