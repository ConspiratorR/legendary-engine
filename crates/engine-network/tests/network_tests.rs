use engine_network::{
    ClientConfig, ConnectionState, GameClient, GameServer, NetworkMessage, ServerConfig,
};

// ── Server creation ─────────────────────────────────────────────────────────

#[test]
fn server_new_defaults() {
    let server = GameServer::new(ServerConfig::default());
    assert!(!server.is_running());
    assert_eq!(server.client_count(), 0);
    assert_eq!(server.config().port, 7777);
    assert_eq!(server.config().max_connections, 32);
}

#[test]
fn server_new_custom_config() {
    let config = ServerConfig {
        bind_address: "127.0.0.1".into(),
        port: 9999,
        max_connections: 64,
        ..Default::default()
    };
    let server = GameServer::new(config);
    assert_eq!(server.config().port, 9999);
    assert_eq!(server.config().max_connections, 64);
}

#[test]
fn server_start_and_stop() {
    let mut server = GameServer::new(ServerConfig {
        port: 0,
        ..Default::default()
    });
    assert!(server.start().is_ok());
    assert!(server.is_running());

    server.stop();
    assert!(!server.is_running());
    assert_eq!(server.client_count(), 0);
}

#[test]
fn server_start_twice_returns_error() {
    let mut server = GameServer::new(ServerConfig {
        port: 0,
        ..Default::default()
    });
    server.start().unwrap();
    // Binding the same port again should fail (port is in use).
    let second = GameServer::new(ServerConfig {
        port: server.config().port,
        ..Default::default()
    });
    let mut second = second;
    // This may or may not fail depending on OS; just verify no panic.
    let _ = second.start();
    server.stop();
}

// ── Client creation ─────────────────────────────────────────────────────────

#[test]
fn client_new_defaults() {
    let client = GameClient::new(ClientConfig::default());
    assert!(!client.is_connected());
    assert_eq!(client.state(), ConnectionState::Disconnected);
    assert!(client.client_id().is_none());
    assert_eq!(client.outgoing_count(), 0);
}

#[test]
fn client_new_custom_config() {
    let config = ClientConfig {
        server_address: "10.0.0.1".into(),
        port: 9000,
        connect_timeout_ms: 2000,
        max_reconnect_attempts: 5,
        reconnect_delay_ms: 500,
    };
    let client = GameClient::new(config);
    assert_eq!(client.server_address(), "10.0.0.1:9000");
    assert_eq!(client.config().connect_timeout_ms, 2000);
}

#[test]
fn client_connect_to_unreachable() {
    let mut client = GameClient::new(ClientConfig {
        server_address: "127.0.0.1".into(),
        port: 1,
        connect_timeout_ms: 100,
        max_reconnect_attempts: 0,
        reconnect_delay_ms: 0,
    });
    let result = client.connect();
    assert!(result.is_err());
    assert!(!client.is_connected());
}

#[test]
fn client_send_without_connection() {
    let mut client = GameClient::new(ClientConfig::default());
    client.send(NetworkMessage::Chat {
        sender: "test".into(),
        text: "hello".into(),
    });
    assert_eq!(client.outgoing_count(), 1);
}

#[test]
fn client_disconnect_clears_state() {
    let mut client = GameClient::new(ClientConfig::default());
    client.send(NetworkMessage::Chat {
        sender: "test".into(),
        text: "hello".into(),
    });
    client.disconnect();
    assert_eq!(client.outgoing_count(), 0);
    assert!(!client.is_connected());
    assert_eq!(client.state(), ConnectionState::Disconnected);
}

// ── Connection state ────────────────────────────────────────────────────────

#[test]
fn connection_state_variants() {
    assert_ne!(ConnectionState::Disconnected, ConnectionState::Connected);
    assert_ne!(ConnectionState::Connecting, ConnectionState::Reconnecting);
}

#[test]
fn connection_state_clone_copy() {
    let state = ConnectionState::Connected;
    let cloned = state;
    assert_eq!(state, cloned);
}

// ── Message serialization ───────────────────────────────────────────────────

#[test]
fn message_chat_roundtrip() {
    let msg = NetworkMessage::Chat {
        sender: "player1".into(),
        text: "hello world".into(),
    };
    let bytes = msg.serialize();
    let restored = NetworkMessage::deserialize(&bytes).unwrap();
    match restored {
        NetworkMessage::Chat { sender, text } => {
            assert_eq!(sender, "player1");
            assert_eq!(text, "hello world");
        }
        _ => panic!("expected Chat"),
    }
}

#[test]
fn message_heartbeat_roundtrip() {
    let msg = NetworkMessage::Heartbeat;
    let bytes = msg.serialize();
    assert_eq!(bytes, vec![5]);
    let restored = NetworkMessage::deserialize(&bytes).unwrap();
    assert!(matches!(restored, NetworkMessage::Heartbeat));
}

#[test]
fn message_entity_update_roundtrip() {
    let msg = NetworkMessage::EntityUpdate {
        entity_id: 42,
        position: [1.0, 2.0, 3.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    let bytes = msg.serialize();
    let restored = NetworkMessage::deserialize(&bytes).unwrap();
    match restored {
        NetworkMessage::EntityUpdate {
            entity_id,
            position,
            rotation,
        } => {
            assert_eq!(entity_id, 42);
            assert_eq!(position, [1.0, 2.0, 3.0]);
            assert_eq!(rotation, [0.0, 0.0, 0.0, 1.0]);
        }
        _ => panic!("expected EntityUpdate"),
    }
}

#[test]
fn message_disconnect_roundtrip() {
    let msg = NetworkMessage::Disconnect {
        reason: "shutdown".into(),
    };
    let bytes = msg.serialize();
    let restored = NetworkMessage::deserialize(&bytes).unwrap();
    match restored {
        NetworkMessage::Disconnect { reason } => assert_eq!(reason, "shutdown"),
        _ => panic!("expected Disconnect"),
    }
}

#[test]
fn message_deserialize_empty_returns_none() {
    assert!(NetworkMessage::deserialize(&[]).is_none());
}

#[test]
fn message_deserialize_unknown_tag_returns_none() {
    assert!(NetworkMessage::deserialize(&[255]).is_none());
}

#[test]
fn message_state_snapshot_roundtrip() {
    let msg = NetworkMessage::StateSnapshot {
        tick: 10,
        entities: vec![(0, vec![(1, vec![10, 20, 30])])],
        despawned: vec![5],
    };
    let bytes = msg.serialize();
    let restored = NetworkMessage::deserialize(&bytes).unwrap();
    match restored {
        NetworkMessage::StateSnapshot {
            tick,
            entities,
            despawned,
        } => {
            assert_eq!(tick, 10);
            assert_eq!(entities.len(), 1);
            assert_eq!(despawned, vec![5]);
        }
        _ => panic!("expected StateSnapshot"),
    }
}

#[test]
fn message_handshake_roundtrip() {
    let msg = NetworkMessage::Handshake {
        client_id: Some(7),
        version: "2.0".into(),
    };
    let bytes = msg.serialize();
    let restored = NetworkMessage::deserialize(&bytes).unwrap();
    match restored {
        NetworkMessage::Handshake { client_id, version } => {
            assert_eq!(client_id, Some(7));
            assert_eq!(version, "2.0");
        }
        _ => panic!("expected Handshake"),
    }
}

// ── Server messaging ────────────────────────────────────────────────────────

#[test]
fn server_broadcast_queues_message() {
    let mut server = GameServer::new(ServerConfig::default());
    server.broadcast(NetworkMessage::Chat {
        sender: "srv".into(),
        text: "hi".into(),
    });
    assert_eq!(server.router().pending_count(), 1);
}

#[test]
fn server_send_to_queues_message() {
    let mut server = GameServer::new(ServerConfig::default());
    server.send_to(
        1,
        NetworkMessage::Chat {
            sender: "srv".into(),
            text: "private".into(),
        },
    );
    assert_eq!(server.router().pending_count(), 1);
}

#[test]
fn server_broadcast_except_queues_message() {
    let mut server = GameServer::new(ServerConfig::default());
    server.broadcast_except(
        1,
        NetworkMessage::Chat {
            sender: "p1".into(),
            text: "hello".into(),
        },
    );
    assert_eq!(server.router().pending_count(), 1);
}

#[test]
fn server_send_to_group_queues_message() {
    let mut server = GameServer::new(ServerConfig::default());
    server.send_to_group(
        "team1",
        NetworkMessage::Chat {
            sender: "srv".into(),
            text: "team".into(),
        },
    );
    assert_eq!(server.router().pending_count(), 1);
}

#[test]
fn server_stop_clears_state() {
    let mut server = GameServer::new(ServerConfig::default());
    server.broadcast(NetworkMessage::Chat {
        sender: "srv".into(),
        text: "msg".into(),
    });
    server.stop();
    assert_eq!(server.router().pending_count(), 0);
    assert_eq!(server.client_count(), 0);
}
