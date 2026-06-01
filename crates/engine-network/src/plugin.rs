//! Network plugin for engine.
use crate::authority::{AuthoritativeServer, ClientAuthority};
use crate::client::GameClient;
use crate::message::NetworkMessage;
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
        app.add_system(authority_server_system);
        app.add_system(authority_client_system);
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

/// ECS system that processes authoritative server logic each tick.
///
/// Advances the server tick, drains incoming messages from the game server
/// and routes `PlayerInput` messages to the authoritative server's input queue,
/// then broadcasts the current world state.
pub fn authority_server_system(world: &mut World) {
    // Take the authoritative server out to avoid borrow conflicts
    let mut auth = match world.remove_resource::<AuthoritativeServer>() {
        Some(a) => a,
        None => return,
    };

    auth.advance_tick();

    // Take game server out so we can use both resources and the world
    let mut server = match world.remove_resource::<GameServer>() {
        Some(s) => s,
        None => {
            world.insert_resource(auth);
            return;
        }
    };

    // Drain incoming messages and route player inputs
    let messages = server.drain_incoming();
    for (client_id, msg) in messages {
        if let NetworkMessage::PlayerInput {
            client_tick,
            input_data,
        } = msg
        {
            auth.push_input(client_id, client_tick, input_data);
        }
    }

    // Broadcast current world state
    auth.broadcast_state(world, &mut server);

    // Put both resources back
    world.insert_resource(server);
    world.insert_resource(auth);
}

/// ECS system that processes incoming authoritative messages on the client side.
///
/// Receives messages from the game client and applies snapshots/corrections
/// to the local ECS world.
pub fn authority_client_system(world: &mut World) {
    // Take client authority out to avoid borrow conflicts
    let mut client_auth = match world.remove_resource::<ClientAuthority>() {
        Some(c) => c,
        None => return,
    };

    // Take game client out so we can use both and the world
    let mut client = match world.remove_resource::<GameClient>() {
        Some(c) => c,
        None => {
            world.insert_resource(client_auth);
            return;
        }
    };

    let messages = client.receive();
    for msg in &messages {
        client_auth.handle_message(world, msg);
    }

    // Put both resources back
    world.insert_resource(client);
    world.insert_resource(client_auth);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::ClientConfig;
    use crate::server::ServerConfig;
    use crate::snapshot::{NetworkSync, SnapshotRegistry};

    #[derive(Debug, Clone)]
    struct TestPos(f32, f32, f32);

    impl NetworkSync for TestPos {
        fn serialize(&self) -> Vec<u8> {
            let mut b = Vec::new();
            b.extend(&self.0.to_le_bytes());
            b.extend(&self.1.to_le_bytes());
            b.extend(&self.2.to_le_bytes());
            b
        }
        fn deserialize(data: &[u8]) -> Option<Self> {
            if data.len() < 12 {
                return None;
            }
            Some(TestPos(
                f32::from_le_bytes(data[0..4].try_into().ok()?),
                f32::from_le_bytes(data[4..8].try_into().ok()?),
                f32::from_le_bytes(data[8..12].try_into().ok()?),
            ))
        }
    }

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
        network_send_system(&mut world);
    }

    #[test]
    fn test_network_send_system_with_client() {
        let mut world = World::new();
        let client = GameClient::new(ClientConfig::default());
        world.insert_resource(client);
        network_send_system(&mut world);
    }

    #[test]
    fn test_network_receive_system_with_server() {
        let mut world = World::new();
        let server = GameServer::new(ServerConfig::default());
        world.insert_resource(server);
        network_receive_system(&mut world);
    }

    #[test]
    fn test_network_receive_system_with_client() {
        let mut world = World::new();
        let client = GameClient::new(ClientConfig::default());
        world.insert_resource(client);
        network_receive_system(&mut world);
    }

    #[test]
    fn test_authority_server_system_no_auth() {
        let mut world = World::new();
        // No AuthoritativeServer resource — should return early
        authority_server_system(&mut world);
    }

    #[test]
    fn test_authority_server_system_with_auth() {
        let mut world = World::new();
        let mut reg = SnapshotRegistry::new();
        reg.register::<TestPos>();
        let auth = AuthoritativeServer::new(crate::authority::AuthorityConfig::default(), reg);
        world.insert_resource(auth);
        // No GameServer — should still not panic
        authority_server_system(&mut world);
    }

    #[test]
    fn test_authority_server_system_with_both() {
        let mut world = World::new();
        let mut reg = SnapshotRegistry::new();
        reg.register::<TestPos>();
        let auth = AuthoritativeServer::new(crate::authority::AuthorityConfig::default(), reg);
        world.insert_resource(auth);
        world.insert_resource(GameServer::new(ServerConfig::default()));
        authority_server_system(&mut world);
    }

    #[test]
    fn test_authority_client_system_no_auth() {
        let mut world = World::new();
        authority_client_system(&mut world);
    }

    #[test]
    fn test_authority_client_system_with_auth() {
        let mut world = World::new();
        let mut reg = SnapshotRegistry::new();
        reg.register::<TestPos>();
        let client_auth = ClientAuthority::new(reg);
        world.insert_resource(client_auth);
        // No GameClient — should still not panic
        authority_client_system(&mut world);
    }

    #[test]
    fn test_authority_client_system_with_both() {
        let mut world = World::new();
        let mut reg = SnapshotRegistry::new();
        reg.register::<TestPos>();
        let client_auth = ClientAuthority::new(reg);
        world.insert_resource(client_auth);
        world.insert_resource(GameClient::new(ClientConfig::default()));
        authority_client_system(&mut world);
    }
}
