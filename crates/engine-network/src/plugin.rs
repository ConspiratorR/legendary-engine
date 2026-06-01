//! Network plugin for engine.
use crate::client::GameClient;
use crate::server::GameServer;
use engine_core::app::AppBuilder;
use engine_core::plugin::Plugin;
use engine_ecs::world::World;

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
        let world = app.world_mut();
        world.insert_resource(NetworkConfig::default());

        // Register ECS systems for network processing
        app.add_system(network_send_system);
        app.add_system(network_receive_system);
    }
}

/// ECS system that sends queued outgoing messages.
///
/// This system reads from the `GameServer` or `GameClient` resource
/// and flushes outgoing messages through the network connections.
pub fn network_send_system(world: &mut World) {
    // Server-side: accept connections, send heartbeats, flush messages
    if let Some(server) = world.get_resource_mut::<GameServer>()
        && server.is_running()
    {
        server.accept_connections();
        server.send_heartbeats();
        server.send_messages();
        server.check_timeouts();
    }

    // Client-side: flush outgoing messages
    if let Some(client) = world.get_resource_mut::<GameClient>()
        && client.is_connected()
    {
        client.flush_outgoing();
    }
}

/// ECS system that receives incoming network messages.
///
/// This system reads incoming messages from the `GameServer` or `GameClient`
/// resource and makes them available for other systems to process.
pub fn network_receive_system(world: &mut World) {
    // Server-side: receive messages from clients
    if let Some(server) = world.get_resource_mut::<GameServer>()
        && server.is_running()
    {
        server.receive_messages();
    }

    // Client-side: receive messages from server, handle reconnection
    if let Some(client) = world.get_resource_mut::<GameClient>() {
        if client.is_connected() {
            client.receive();
        } else {
            let _ = client.try_reconnect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ClientConfig;
    use crate::server::ServerConfig;

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert!(!config.is_server);
        assert_eq!(config.port, 7777);
    }

    #[test]
    fn test_network_send_system_with_server() {
        let mut world = World::new();
        let server = GameServer::new(ServerConfig::default());
        world.insert_resource(server);
        // Should not panic even without a running server
        network_send_system(&mut world);
    }

    #[test]
    fn test_network_send_system_with_client() {
        let mut world = World::new();
        let client = GameClient::new(ClientConfig::default());
        world.insert_resource(client);
        // Should not panic even without a connection
        network_send_system(&mut world);
    }

    #[test]
    fn test_network_receive_system_with_server() {
        let mut world = World::new();
        let server = GameServer::new(ServerConfig::default());
        world.insert_resource(server);
        // Should not panic even without a running server
        network_receive_system(&mut world);
    }

    #[test]
    fn test_network_receive_system_with_client() {
        let mut world = World::new();
        let client = GameClient::new(ClientConfig::default());
        world.insert_resource(client);
        // Should not panic even without a connection
        network_receive_system(&mut world);
    }
}
