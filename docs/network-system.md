# Network System

The `engine-network` crate provides multiplayer networking through a client/server architecture with ECS integration.

## Architecture

The networking layer follows an **authoritative server** model where the server is the single source of truth for the game world. Clients send inputs and receive state updates; the server runs the full ECS simulation.

```
┌──────────────────────────────────────────────────────────┐
│  Server                                                  │
│  ECS World ──► AuthoritativeServer ──► GameServer        │
│  (simulation)   (tick + snapshot)      (TCP transport)   │
└──────────────────────────┬───────────────────────────────┘
                           │ StateSnapshot / DeltaSnapshot
                           ▼
┌──────────────────────────────────────────────────────────┐
│  Client                                                  │
│  GameClient ──► ClientAuthority ──► ECS World            │
│  (TCP transport)  (apply snapshot)   (render only)       │
│  Interpolator ──► smooth visual state                    │
│  PredictionManager ──► client-side prediction + reconcile│
└──────────────────────────────────────────────────────────┘
```

### Transport

TCP is used for reliable communication via `TcpConnection` and `TcpListener`. UDP is available for lower-latency needs via `UdpSocket`. All messages use a compact binary format with a 1-byte type tag (see `NetworkMessage`).

### Key Modules

| Module | Purpose |
|--------|---------|
| `server` | `GameServer` — TCP listener, session management, message routing |
| `client` | `GameClient` — TCP connection, send/receive, reconnection |
| `message` | `NetworkMessage` enum — all wire protocol message types |
| `authority` | `AuthoritativeServer` / `ClientAuthority` — tick + snapshot lifecycle |
| `snapshot` | `WorldSnapshot`, `SnapshotRegistry`, `NetworkSync` trait |
| `interpolation` | `Interpolator`, `PredictionManager`, `SnapshotBuffer` |
| `reconnect` | `ReconnectManager` — session resumption with delta recovery |
| `routing` | `MessageRouter` — unicast, broadcast, group messaging |
| `session` | `SessionManager` — per-client state and heartbeat tracking |
| `plugin` | `NetworkPlugin` — ECS system registration for automatic networking |

---

## Message Types

All messages are defined in `NetworkMessage` and serialized with a 1-byte type tag followed by type-specific payloads.

### Connection Management

| Variant | Tag | Direction | Description |
|---------|-----|-----------|-------------|
| `Handshake { client_id, version }` | 0 | S→C | Assigns client ID and protocol version |
| `Disconnect { reason }` | 4 | Both | Graceful disconnect with reason |
| `Heartbeat` | 5 | Both | Keepalive ping (handled internally) |

### Game State

| Variant | Tag | Direction | Description |
|---------|-----|-----------|-------------|
| `EntityUpdate { entity_id, position, rotation }` | 1 | Both | Single entity position/rotation update |
| `PlayerInput { client_tick, input_data }` | 2 | C→S | Player input with client timestamp |
| `PredictedInput { client_tick, input_data, is_predicted }` | 13 | C→S | Input with prediction flag for reconciliation |

### Snapshot Synchronization

| Variant | Tag | Direction | Description |
|---------|-----|-----------|-------------|
| `StateSnapshot { tick, entities, despawned }` | 6 | S→C | Full world state snapshot |
| `DeltaSnapshot { tick, changed, despawned }` | 7 | S→C | Only changed entities since last full snapshot |
| `InputAck { client_tick, server_tick }` | 8 | S→C | Server acknowledgment of processed input |
| `Correction { tick, entities }` | 9 | S→C | Server correction to client state |

### Reconnection

| Variant | Tag | Direction | Description |
|---------|-----|-----------|-------------|
| `ReconnectRequest { client_id, reconnect_token, last_tick }` | 11 | C→S | Client requests session resumption |
| `ReconnectAck { client_id, server_tick }` | 12 | S→C | Server confirms reconnection |
| `ReconnectSnapshot { client_id, tick, entities, despawned, missed_ticks }` | 10 | S→C | Full state recovery after reconnect |

### Matchmaking & Rooms

| Variant | Tag | Direction | Description |
|---------|-----|-----------|-------------|
| `CreateRoom { name, max_players, game_mode }` | 14 | C→S | Create a game room |
| `JoinRoom { room_id }` | 15 | C→S | Join an existing room |
| `LeaveRoom` | 16 | C→S | Leave current room |
| `RoomList { rooms }` | 17 | S→C | List of available rooms |
| `MatchFound { room_id, player_ids }` | 18 | S→C | Match found notification |
| `LobbyUpdate { room_id, player_ids, ready_states }` | 19 | S→C | Lobby state update |
| `ReadyUp { ready }` | 20 | C→S | Player ready toggle |

### Chat

| Variant | Tag | Direction | Description |
|---------|-----|-----------|-------------|
| `Chat { sender, text }` | 3 | Both | Chat message |

### NAT Traversal

| Variant | Tag | Direction | Description |
|---------|-----|-----------|-------------|
| `RendezvousRegister { client_id }` | 21 | C→S | Register with rendezvous server |
| `RendezvousPair { peer_addr, peer_id }` | 22 | S→C | Peer address for hole punching |
| `RendezvousResult { success, public_addr }` | 23 | S→C | Hole punching result |

---

## Server Setup

### Basic Server

```rust
use engine_network::{GameServer, ServerConfig, NetworkMessage};

let mut server = GameServer::new(ServerConfig {
    bind_address: "0.0.0.0".to_string(),
    port: 7777,
    max_connections: 32,
    ..Default::default()
});

server.start().expect("failed to start server");

// Game loop
loop {
    // Accept new connections
    let new_clients = server.accept_connections();
    for client_id in &new_clients {
        println!("Client {} connected", client_id);
    }

    // Receive messages from clients
    let messages = server.receive_messages();
    for (client_id, msg) in messages {
        match msg {
            NetworkMessage::Chat { sender, text } => {
                // Broadcast chat to all clients
                server.broadcast(NetworkMessage::Chat {
                    sender: sender.clone(),
                    text,
                });
            }
            NetworkMessage::PlayerInput { client_tick, input_data } => {
                // Process player input...
            }
            _ => {}
        }
    }

    // Send heartbeats and flush outgoing
    server.send_heartbeats();
    server.send_messages();

    // Check for timed-out clients
    let timed_out = server.check_timeouts();
    for client_id in timed_out {
        println!("Client {} timed out", client_id);
    }
}
```

### Server Configuration

```rust
pub struct ServerConfig {
    pub bind_address: String,      // default: "0.0.0.0"
    pub port: u16,                 // default: 7777
    pub max_connections: usize,    // default: 32
    pub session: SessionConfig,    // heartbeat/timeout settings
}
```

### Message Routing

The server supports multiple routing modes:

```rust
// Unicast — send to one client
server.send_to(client_id, NetworkMessage::Chat {
    sender: "server".into(),
    text: "private message".into(),
});

// Broadcast — send to all clients
server.broadcast(NetworkMessage::Chat {
    sender: "server".into(),
    text: "announcement".into(),
});

// Broadcast except — send to all except one
server.broadcast_except(sender_id, NetworkMessage::Chat {
    sender: sender_name,
    text,
});

// Group messaging — send to a named group
server.send_to_group("team1", NetworkMessage::Chat {
    sender: "server".into(),
    text: "team message".into(),
});

// Group membership
server.join_group(client_id, "team1")?;
server.leave_group(client_id, "team1")?;
```

---

## Client Setup

### Basic Client

```rust
use engine_network::{GameClient, ClientConfig, NetworkMessage, ConnectionState};

let mut client = GameClient::new(ClientConfig {
    server_address: "127.0.0.1".to_string(),
    port: 7777,
    connect_timeout_ms: 5000,
    max_reconnect_attempts: 3,
    reconnect_delay_ms: 1000,
});

// Connect to server
client.connect().expect("connection failed");
assert!(client.is_connected());

// Queue a message
client.send(NetworkMessage::Chat {
    sender: "player1".into(),
    text: "hello".into(),
});

// Flush outgoing messages
client.flush_outgoing();

// Receive messages from server
let messages = client.receive();
for msg in messages {
    match msg {
        NetworkMessage::Handshake { client_id, .. } => {
            println!("Assigned client ID: {:?}", client_id);
        }
        NetworkMessage::StateSnapshot { tick, entities, .. } => {
            // Apply snapshot to local world...
        }
        _ => {}
    }
}

// Graceful disconnect
client.disconnect();
```

### Client Configuration

```rust
pub struct ClientConfig {
    pub server_address: String,           // default: "127.0.0.1"
    pub port: u16,                        // default: 7777
    pub connect_timeout_ms: u64,          // default: 5000
    pub max_reconnect_attempts: u32,      // default: 3 (0 = no reconnect)
    pub reconnect_delay_ms: u64,          // default: 1000
}
```

### Reconnection

The client automatically attempts reconnection when configured:

```rust
let mut client = GameClient::new(ClientConfig {
    max_reconnect_attempts: 5,
    reconnect_delay_ms: 2000,
    ..Default::default()
});

// If connection drops, try_reconnect() is called automatically
// by the ECS network_receive_system, or manually:
loop {
    match client.try_reconnect() {
        Ok(true) => println!("Reconnected!"),
        Ok(false) => { /* still waiting or max attempts reached */ }
        Err(e) => eprintln!("Reconnect error: {}", e),
    }
}
```

---

## Authoritative Server Mode

The `AuthoritativeServer` manages the server tick, snapshot generation, and input processing. It is the bridge between the ECS world and the network.

### Server-Side Setup

```rust
use engine_network::{
    AuthoritativeServer, AuthorityConfig, GameServer, ServerConfig,
    SnapshotRegistry, NetworkSync,
};

// 1. Register components for network sync
let mut registry = SnapshotRegistry::new();
registry.register::<Position>();
registry.register::<Health>();
registry.register::<Velocity>();

// 2. Create the authoritative server
let mut auth = AuthoritativeServer::new(
    AuthorityConfig {
        full_snapshot_interval: 60,       // full snapshot every 60 ticks
        max_entities_per_snapshot: 256,   // split large snapshots
        send_deltas: true,                // send deltas between full snapshots
    },
    registry,
);

// 3. Track entities that should be replicated
auth.track_entity(entity_0.index());
auth.track_entity(entity_1.index());

// 4. In the game loop, after ECS simulation:
auth.advance_tick();

// Queue client inputs received from the network
for (client_id, msg) in server.drain_incoming() {
    if let NetworkMessage::PlayerInput { client_tick, input_data } = msg {
        auth.push_input(client_id, client_tick, input_data);
    }
}

// Process pending inputs
let inputs = auth.drain_inputs();
for input in inputs {
    // Apply input to ECS world...
}

// Broadcast world state to all clients
auth.broadcast_state(&world, &mut server);

// Send input acknowledgments
server.send_to(client_id, NetworkMessage::InputAck {
    client_tick,
    server_tick: auth.tick(),
});
```

### Client-Side Setup

```rust
use engine_network::{ClientAuthority, SnapshotRegistry, NetworkSync};

let mut registry = SnapshotRegistry::new();
registry.register::<Position>();
registry.register::<Health>();

let mut client_auth = ClientAuthority::new(registry);

// In the game loop, after receiving messages:
let messages = client.receive();
for msg in messages {
    client_auth.handle_message(&mut world, &msg);
}
```

### Authority Configuration

```rust
pub struct AuthorityConfig {
    pub full_snapshot_interval: u64,      // ticks between full snapshots (0 = every tick)
    pub max_entities_per_snapshot: usize, // split threshold for large snapshots
    pub send_deltas: bool,                // whether to send delta snapshots
}
```

---

## State Snapshot Synchronization

### The `NetworkSync` Trait

Components that need network synchronization implement `NetworkSync`:

```rust
use engine_network::NetworkSync;

#[derive(Debug, Clone)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl NetworkSync for Position {
    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.x.to_le_bytes());
        bytes.extend(&self.y.to_le_bytes());
        bytes.extend(&self.z.to_le_bytes());
        bytes
    }

    fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }
        Some(Self {
            x: f32::from_le_bytes(data[0..4].try_into().ok()?),
            y: f32::from_le_bytes(data[4..8].try_into().ok()?),
            z: f32::from_le_bytes(data[8..12].try_into().ok()?),
        })
    }
}
```

### SnapshotRegistry

The registry maps component types to serialize/deserialize functions:

```rust
let mut registry = SnapshotRegistry::new();
registry.register::<Position>();
registry.register::<Health>();
registry.register::<Velocity>();

// Extract a snapshot from the world
let snapshot = registry.snapshot_entities(&world, &[0, 1, 2], tick);

// Apply a snapshot to the world
registry.apply_snapshot(&mut world, &snapshot);

// Apply a delta (changed entities only)
registry.apply_delta(&mut world, &changed_entities, &despawned_indices);
```

### Delta Compression

Full snapshots are sent at configurable intervals. Between full snapshots, only changed entities are sent as delta snapshots:

```
Tick 0:  FullSnapshot  (all entities)     ──► Sent
Tick 1:  DeltaSnapshot (changed only)     ──► Sent
Tick 2:  DeltaSnapshot (changed only)     ──► Sent
...
Tick 60: FullSnapshot  (all entities)     ──► Sent
```

The delta is computed by comparing the current snapshot against the last full snapshot:

```rust
let current = registry.snapshot_entities(&world, &indices, tick);
let (changed, despawned) = current.delta_from(&last_full_snapshot);
```

---

## Interpolation

The `Interpolator` provides smooth visual state between server snapshots on the client:

```rust
use engine_network::Interpolator;

let mut interpolator = Interpolator::new(
    2,  // interpolation_delay in ticks
    10, // buffer_capacity (number of snapshots to store)
);

// Each frame, push new snapshots and update
interpolator.push_snapshot(received_snapshot);
interpolator.update();

// Get interpolated state for an entity
if let Some(interpolated_data) = interpolator.interpolate_entity(entity_index, type_hash) {
    // Apply to render entity...
}

// Or interpolate all entities at once
let all = interpolator.interpolate_all();
```

### Interpolation Delay

The render tick lags behind the server tick by `interpolation_delay` ticks. This creates a buffer that allows smooth interpolation between two known server states:

```
Server tick:  100 ────────────────────────►
Render tick:   98 ────────────────────────►
              (2 ticks behind)
```

---

## Client-Side Prediction

The `PredictionManager` allows the client to apply inputs immediately for responsiveness, then reconcile with server corrections:

```rust
use engine_network::PredictionManager;

let mut prediction = PredictionManager::new(120);
prediction.snap_threshold = 10.0;  // snap if diff > 10
prediction.smooth_frames = 5;      // smooth over 5 frames

// Store predicted state after applying local input
prediction.store_prediction(client_tick, predicted_entities);

// When server sends a correction:
let corrected = prediction.reconcile(
    correction_tick,
    &server_state,
    &pending_inputs,
);

// If smoothing is active, interpolate toward correction
if let Some(smoothed) = prediction.update_smoothing() {
    // Apply smoothed state...
}
```

### Reconciliation Strategy

- **Large error** (exceeds `snap_threshold`): Snap immediately to server state
- **Small error** (below threshold): Smooth over `smooth_frames` frames using linear interpolation

---

## Reconnection

The `ReconnectManager` handles session resumption after disconnection:

### Server-Side

```rust
use engine_network::{ReconnectManager, ReconnectConfig};

let mut reconnect = ReconnectManager::new(ReconnectConfig {
    session_timeout_secs: 60,         // keep sessions for 60s
    max_pending_deltas: 300,          // max deltas to buffer
    full_snapshot_threshold: 60,      // full snapshot if gap > 60 ticks
});

// On first connection, issue a token
let token = reconnect.issue_token(client_id);

// When client disconnects, store their session
reconnect.store_session(client_id, token, last_snapshot, last_tick);

// While disconnected, accumulate deltas
reconnect.push_delta(client_id, tick, changed, despawned);

// On reconnect attempt, verify token and send recovery
if let Some(msg) = reconnect.try_reconnect(client_id, token, last_tick, current_tick) {
    server.send_to(client_id, msg);
}

// Clean up expired sessions
let expired = reconnect.cleanup_expired();
```

### Client-Side

```rust
// Send reconnection request with token
client.send(NetworkMessage::ReconnectRequest {
    client_id,
    reconnect_token: token.0,
    last_tick: client_auth.client_tick(),
});

// Receive ReconnectSnapshot and apply it
let messages = client.receive();
for msg in messages {
    client_auth.handle_message(&mut world, &msg);
}
```

---

## ECS Plugin Integration

The `NetworkPlugin` registers ECS systems for automatic network processing:

```rust
use engine_core::app::AppBuilder;
use engine_network::NetworkPlugin;

let mut app = AppBuilder::new();
app.add_plugin(NetworkPlugin);
```

### Registered Systems

| System | Purpose |
|--------|---------|
| `network_send_system` | Accepts connections, sends heartbeats, flushes outgoing messages |
| `network_receive_system` | Receives incoming messages, handles reconnection attempts |
| `authority_server_system` | Advances server tick, routes player inputs, broadcasts state |
| `authority_client_system` | Receives messages, applies snapshots/corrections to local world |

### NetworkConfig Resource

```rust
pub struct NetworkConfig {
    pub is_server: bool,              // default: false
    pub server_address: String,       // default: "127.0.0.1"
    pub port: u16,                    // default: 7777
    pub max_connections: u32,         // default: 32
}
```

---

## Configuration Options

### ServerConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `bind_address` | `String` | `"0.0.0.0"` | Address to bind to |
| `port` | `u16` | `7777` | Port to listen on |
| `max_connections` | `usize` | `32` | Maximum concurrent connections |
| `session` | `SessionConfig` | default | Heartbeat/timeout settings |

### ClientConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `server_address` | `String` | `"127.0.0.1"` | Server address |
| `port` | `u16` | `7777` | Server port |
| `connect_timeout_ms` | `u64` | `5000` | Connection timeout |
| `max_reconnect_attempts` | `u32` | `3` | Max reconnect attempts (0 = disabled) |
| `reconnect_delay_ms` | `u64` | `1000` | Delay between attempts |

### AuthorityConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `full_snapshot_interval` | `u64` | `60` | Ticks between full snapshots (0 = every tick) |
| `max_entities_per_snapshot` | `usize` | `256` | Entities per message before splitting |
| `send_deltas` | `bool` | `true` | Send delta snapshots between full snapshots |

### ReconnectConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `session_timeout_secs` | `u64` | `60` | How long to keep disconnected sessions |
| `max_pending_deltas` | `usize` | `300` | Max deltas per disconnected client |
| `full_snapshot_threshold` | `u64` | `60` | Tick gap threshold for full vs delta recovery |

### Interpolator

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `interpolation_delay` | `u64` | `2` | Render tick lag behind server |

### PredictionManager

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_predictions` | `usize` | `120` | Max predicted states to store |
| `snap_threshold` | `f32` | `10.0` | Byte-diff threshold for snap vs smooth |
| `smooth_frames` | `u32` | `5` | Frames to smooth small corrections over |

---

## Wire Protocol

Messages use a compact binary format:

- **1-byte type tag** identifies the message variant (0–23)
- **Little-endian** encoding for all multi-byte values
- **Length-prefixed** strings: `u32` length followed by UTF-8 bytes
- **Entity component data**: `u32` entity index → `u64` type hash → `u32` data length → raw bytes

Example: `Heartbeat` is a single byte `[5]`. A `Chat` message is `[3, sender_len(u32), sender_bytes..., text_len(u32), text_bytes...]`.
