//! Network message types and serialization.

/// Entity component data: `(entity_index, [(type_hash, data_bytes)])`.
pub type EntityComponentData = Vec<(u32, Vec<(u64, Vec<u8>)>)>;

/// Network message type.
#[derive(Debug, Clone)]
pub enum NetworkMessage {
    /// Connection handshake.
    Handshake {
        client_id: Option<u64>,
        version: String,
    },
    /// Entity position update.
    EntityUpdate {
        entity_id: u64,
        position: [f32; 3],
        rotation: [f32; 4],
    },
    /// Player input forwarded to server.
    PlayerInput {
        /// Client-side tick when input was captured.
        client_tick: u64,
        /// Serialized input data.
        input_data: Vec<u8>,
    },
    /// Chat message.
    Chat { sender: String, text: String },
    /// Disconnect notification.
    Disconnect { reason: String },
    /// Heartbeat keepalive.
    Heartbeat,
    /// Full world state snapshot (server → client).
    StateSnapshot {
        /// Server tick number when this snapshot was taken.
        tick: u64,
        /// Serialized world state: (entity_index, [(type_hash, data_bytes)]).
        entities: EntityComponentData,
        /// Entity indices that were despawned since last snapshot.
        despawned: Vec<u32>,
    },
    /// Delta snapshot with only changed entities (server → client).
    DeltaSnapshot {
        /// Server tick number.
        tick: u64,
        /// Changed entities: (entity_index, [(type_hash, data_bytes)]).
        changed: EntityComponentData,
        /// Despawned entity indices.
        despawned: Vec<u32>,
    },
    /// Server acknowledgment of processed input (server → client).
    InputAck {
        /// The client tick whose input was processed.
        client_tick: u64,
        /// Server tick at processing time.
        server_tick: u64,
    },
    /// Server correction to client state (server → client).
    Correction {
        /// Server tick of the correction.
        tick: u64,
        /// Corrected entity states: (entity_index, [(type_hash, data_bytes)]).
        entities: EntityComponentData,
    },
    /// Full world snapshot for reconnection (server → client).
    ReconnectSnapshot {
        /// Client ID this snapshot is for.
        client_id: u64,
        /// Server tick when this snapshot was taken.
        tick: u64,
        /// Serialized world state.
        entities: EntityComponentData,
        /// Entity indices despawned since client disconnected.
        despawned: Vec<u32>,
        /// Number of ticks the client missed.
        missed_ticks: u64,
    },
    /// Client reconnection request with token (client → server).
    ReconnectRequest {
        /// Client ID requesting reconnection.
        client_id: u64,
        /// Token from the original session.
        reconnect_token: u64,
        /// Last tick the client received.
        last_tick: u64,
    },
    /// Server acknowledgment of reconnection (server → client).
    ReconnectAck {
        /// Client ID.
        client_id: u64,
        /// Server tick at reconnection time.
        server_tick: u64,
    },
    /// Client prediction input with tick for reconciliation (client → server).
    PredictedInput {
        /// Client-side tick when input was captured.
        client_tick: u64,
        /// Serialized input data.
        input_data: Vec<u8>,
        /// Whether this is a predicted input.
        is_predicted: bool,
    },
    /// Create a room (client → server).
    CreateRoom {
        /// Room name.
        name: String,
        /// Max players.
        max_players: u32,
        /// Game mode identifier.
        game_mode: String,
    },
    /// Join a room (client → server).
    JoinRoom {
        /// Room ID to join.
        room_id: u64,
    },
    /// Leave current room (client → server).
    LeaveRoom,
    /// Room list response (server → client).
    RoomList {
        /// Serialized room list.
        rooms: Vec<RoomInfo>,
    },
    /// Match found notification (server → client).
    MatchFound {
        /// Room ID assigned to.
        room_id: u64,
        /// Players in the match.
        player_ids: Vec<u64>,
    },
    /// Lobby state update (server → client).
    LobbyUpdate {
        /// Room ID.
        room_id: u64,
        /// Players in lobby.
        player_ids: Vec<u64>,
        /// Ready states per player.
        ready_states: Vec<bool>,
    },
    /// Player ready toggle (client → server).
    ReadyUp {
        /// Whether the player is ready.
        ready: bool,
    },
    /// Rendezvous registration (client → rendezvous server).
    RendezvousRegister {
        /// Client identifier.
        client_id: u64,
    },
    /// Rendezvous pair info (rendezvous server → client).
    RendezvousPair {
        /// Peer's public address.
        peer_addr: String,
        /// Peer's client ID.
        peer_id: u64,
    },
    /// Rendezvous result (rendezvous server → client).
    RendezvousResult {
        /// Whether hole punching succeeded.
        success: bool,
        /// Public address discovered.
        public_addr: String,
    },
}

/// Room information for room list messages.
#[derive(Debug, Clone)]
pub struct RoomInfo {
    /// Unique room ID.
    pub id: u64,
    /// Room name.
    pub name: String,
    /// Current player count.
    pub player_count: u32,
    /// Maximum players.
    pub max_players: u32,
    /// Game mode.
    pub game_mode: String,
    /// Room state.
    pub state: RoomState,
}

/// State of a game room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomState {
    /// Waiting for players.
    Waiting,
    /// Game in progress.
    InGame,
    /// Room closed.
    Closed,
}

/// Reliability mode for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reliability {
    /// Unreliable - no guarantee of delivery or order.
    Unreliable,
    /// Reliable unordered - guaranteed delivery but no order.
    ReliableUnordered,
    /// Reliable ordered - guaranteed delivery and order.
    ReliableOrdered,
}

fn serialize_entity_components(bytes: &mut Vec<u8>, entities: &EntityComponentData) {
    bytes.extend(&(entities.len() as u32).to_le_bytes());
    for (entity_idx, components) in entities {
        bytes.extend(&entity_idx.to_le_bytes());
        bytes.extend(&(components.len() as u32).to_le_bytes());
        for (type_hash, data) in components {
            bytes.extend(&type_hash.to_le_bytes());
            bytes.extend(&(data.len() as u32).to_le_bytes());
            bytes.extend(data);
        }
    }
}

fn deserialize_entity_components(bytes: &[u8], offset: &mut usize) -> Option<EntityComponentData> {
    if bytes.len() < *offset + 4 {
        return None;
    }
    let entity_count = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().ok()?) as usize;
    *offset += 4;

    let mut entities = Vec::with_capacity(entity_count);
    for _ in 0..entity_count {
        if bytes.len() < *offset + 4 {
            return None;
        }
        let entity_idx = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().ok()?);
        *offset += 4;

        if bytes.len() < *offset + 4 {
            return None;
        }
        let comp_count = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().ok()?) as usize;
        *offset += 4;

        let mut components = Vec::with_capacity(comp_count);
        for _ in 0..comp_count {
            if bytes.len() < *offset + 8 {
                return None;
            }
            let type_hash = u64::from_le_bytes(bytes[*offset..*offset + 8].try_into().ok()?);
            *offset += 8;

            if bytes.len() < *offset + 4 {
                return None;
            }
            let data_len =
                u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().ok()?) as usize;
            *offset += 4;

            if bytes.len() < *offset + data_len {
                return None;
            }
            let data = bytes[*offset..*offset + data_len].to_vec();
            *offset += data_len;

            components.push((type_hash, data));
        }
        entities.push((entity_idx, components));
    }
    Some(entities)
}

fn serialize_despawned(bytes: &mut Vec<u8>, despawned: &[u32]) {
    bytes.extend(&(despawned.len() as u32).to_le_bytes());
    for idx in despawned {
        bytes.extend(&idx.to_le_bytes());
    }
}

fn deserialize_despawned(bytes: &[u8], offset: &mut usize) -> Option<Vec<u32>> {
    if bytes.len() < *offset + 4 {
        return None;
    }
    let count = u32::from_le_bytes(bytes[*offset..*offset + 4].try_into().ok()?) as usize;
    *offset += 4;

    let mut despawned = Vec::with_capacity(count);
    for _ in 0..count {
        if bytes.len() < *offset + 4 {
            return None;
        }
        despawned.push(u32::from_le_bytes(
            bytes[*offset..*offset + 4].try_into().ok()?,
        ));
        *offset += 4;
    }
    Some(despawned)
}

impl NetworkMessage {
    /// Serialize the message to bytes.
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            NetworkMessage::Handshake { client_id, version } => {
                let mut bytes = vec![0];
                if let Some(id) = client_id {
                    bytes.extend(&id.to_le_bytes());
                }
                bytes.extend(version.as_bytes());
                bytes
            }
            NetworkMessage::EntityUpdate {
                entity_id,
                position,
                rotation,
            } => {
                let mut bytes = vec![1];
                bytes.extend(&entity_id.to_le_bytes());
                bytes.extend(&position[0].to_le_bytes());
                bytes.extend(&position[1].to_le_bytes());
                bytes.extend(&position[2].to_le_bytes());
                bytes.extend(&rotation[0].to_le_bytes());
                bytes.extend(&rotation[1].to_le_bytes());
                bytes.extend(&rotation[2].to_le_bytes());
                bytes.extend(&rotation[3].to_le_bytes());
                bytes
            }
            NetworkMessage::PlayerInput {
                client_tick,
                input_data,
            } => {
                let mut bytes = vec![2];
                bytes.extend(&client_tick.to_le_bytes());
                bytes.extend(&(input_data.len() as u32).to_le_bytes());
                bytes.extend(input_data);
                bytes
            }
            NetworkMessage::Chat { sender, text } => {
                let mut bytes = vec![3];
                let sender_bytes = sender.as_bytes();
                bytes.extend(&(sender_bytes.len() as u32).to_le_bytes());
                bytes.extend(sender_bytes);
                let text_bytes = text.as_bytes();
                bytes.extend(&(text_bytes.len() as u32).to_le_bytes());
                bytes.extend(text_bytes);
                bytes
            }
            NetworkMessage::Disconnect { reason } => {
                let mut bytes = vec![4];
                let reason_bytes = reason.as_bytes();
                bytes.extend(&(reason_bytes.len() as u32).to_le_bytes());
                bytes.extend(reason_bytes);
                bytes
            }
            NetworkMessage::Heartbeat => vec![5],
            NetworkMessage::StateSnapshot {
                tick,
                entities,
                despawned,
            } => {
                let mut bytes = vec![6];
                bytes.extend(&tick.to_le_bytes());
                serialize_entity_components(&mut bytes, entities);
                serialize_despawned(&mut bytes, despawned);
                bytes
            }
            NetworkMessage::DeltaSnapshot {
                tick,
                changed,
                despawned,
            } => {
                let mut bytes = vec![7];
                bytes.extend(&tick.to_le_bytes());
                serialize_entity_components(&mut bytes, changed);
                serialize_despawned(&mut bytes, despawned);
                bytes
            }
            NetworkMessage::InputAck {
                client_tick,
                server_tick,
            } => {
                let mut bytes = vec![8];
                bytes.extend(&client_tick.to_le_bytes());
                bytes.extend(&server_tick.to_le_bytes());
                bytes
            }
            NetworkMessage::Correction { tick, entities } => {
                let mut bytes = vec![9];
                bytes.extend(&tick.to_le_bytes());
                serialize_entity_components(&mut bytes, entities);
                bytes
            }
            NetworkMessage::ReconnectSnapshot {
                client_id,
                tick,
                entities,
                despawned,
                missed_ticks,
            } => {
                let mut bytes = vec![10];
                bytes.extend(&client_id.to_le_bytes());
                bytes.extend(&tick.to_le_bytes());
                serialize_entity_components(&mut bytes, entities);
                serialize_despawned(&mut bytes, despawned);
                bytes.extend(&missed_ticks.to_le_bytes());
                bytes
            }
            NetworkMessage::ReconnectRequest {
                client_id,
                reconnect_token,
                last_tick,
            } => {
                let mut bytes = vec![11];
                bytes.extend(&client_id.to_le_bytes());
                bytes.extend(&reconnect_token.to_le_bytes());
                bytes.extend(&last_tick.to_le_bytes());
                bytes
            }
            NetworkMessage::ReconnectAck {
                client_id,
                server_tick,
            } => {
                let mut bytes = vec![12];
                bytes.extend(&client_id.to_le_bytes());
                bytes.extend(&server_tick.to_le_bytes());
                bytes
            }
            NetworkMessage::PredictedInput {
                client_tick,
                input_data,
                is_predicted,
            } => {
                let mut bytes = vec![13];
                bytes.extend(&client_tick.to_le_bytes());
                bytes.extend(&(input_data.len() as u32).to_le_bytes());
                bytes.extend(input_data);
                bytes.push(if *is_predicted { 1 } else { 0 });
                bytes
            }
            NetworkMessage::CreateRoom {
                name,
                max_players,
                game_mode,
            } => {
                let mut bytes = vec![14];
                let name_bytes = name.as_bytes();
                bytes.extend(&(name_bytes.len() as u32).to_le_bytes());
                bytes.extend(name_bytes);
                bytes.extend(&max_players.to_le_bytes());
                let mode_bytes = game_mode.as_bytes();
                bytes.extend(&(mode_bytes.len() as u32).to_le_bytes());
                bytes.extend(mode_bytes);
                bytes
            }
            NetworkMessage::JoinRoom { room_id } => {
                let mut bytes = vec![15];
                bytes.extend(&room_id.to_le_bytes());
                bytes
            }
            NetworkMessage::LeaveRoom => vec![16],
            NetworkMessage::RoomList { rooms } => {
                let mut bytes = vec![17];
                bytes.extend(&(rooms.len() as u32).to_le_bytes());
                for room in rooms {
                    bytes.extend(&room.id.to_le_bytes());
                    let name_bytes = room.name.as_bytes();
                    bytes.extend(&(name_bytes.len() as u32).to_le_bytes());
                    bytes.extend(name_bytes);
                    bytes.extend(&room.player_count.to_le_bytes());
                    bytes.extend(&room.max_players.to_le_bytes());
                    let mode_bytes = room.game_mode.as_bytes();
                    bytes.extend(&(mode_bytes.len() as u32).to_le_bytes());
                    bytes.extend(mode_bytes);
                    bytes.push(match room.state {
                        RoomState::Waiting => 0,
                        RoomState::InGame => 1,
                        RoomState::Closed => 2,
                    });
                }
                bytes
            }
            NetworkMessage::MatchFound {
                room_id,
                player_ids,
            } => {
                let mut bytes = vec![18];
                bytes.extend(&room_id.to_le_bytes());
                bytes.extend(&(player_ids.len() as u32).to_le_bytes());
                for id in player_ids {
                    bytes.extend(&id.to_le_bytes());
                }
                bytes
            }
            NetworkMessage::LobbyUpdate {
                room_id,
                player_ids,
                ready_states,
            } => {
                let mut bytes = vec![19];
                bytes.extend(&room_id.to_le_bytes());
                bytes.extend(&(player_ids.len() as u32).to_le_bytes());
                for id in player_ids {
                    bytes.extend(&id.to_le_bytes());
                }
                for ready in ready_states {
                    bytes.push(if *ready { 1 } else { 0 });
                }
                bytes
            }
            NetworkMessage::ReadyUp { ready } => {
                vec![20, if *ready { 1 } else { 0 }]
            }
            NetworkMessage::RendezvousRegister { client_id } => {
                let mut bytes = vec![21];
                bytes.extend(&client_id.to_le_bytes());
                bytes
            }
            NetworkMessage::RendezvousPair { peer_addr, peer_id } => {
                let mut bytes = vec![22];
                let addr_bytes = peer_addr.as_bytes();
                bytes.extend(&(addr_bytes.len() as u32).to_le_bytes());
                bytes.extend(addr_bytes);
                bytes.extend(&peer_id.to_le_bytes());
                bytes
            }
            NetworkMessage::RendezvousResult {
                success,
                public_addr,
            } => {
                let mut bytes = vec![23];
                bytes.push(if *success { 1 } else { 0 });
                let addr_bytes = public_addr.as_bytes();
                bytes.extend(&(addr_bytes.len() as u32).to_le_bytes());
                bytes.extend(addr_bytes);
                bytes
            }
        }
    }

    /// Deserialize bytes into a message.
    pub fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }

        match bytes[0] {
            0 => {
                let client_id = if bytes.len() > 1 {
                    Some(u64::from_le_bytes(bytes[1..9].try_into().ok()?))
                } else {
                    None
                };
                let version = if bytes.len() > 9 {
                    String::from_utf8_lossy(&bytes[9..]).to_string()
                } else {
                    String::new()
                };
                Some(NetworkMessage::Handshake { client_id, version })
            }
            1 => {
                if bytes.len() < 1 + 8 + 12 + 16 {
                    return None;
                }
                let entity_id = u64::from_le_bytes(bytes[1..9].try_into().ok()?);
                let position = [
                    f32::from_le_bytes(bytes[9..13].try_into().ok()?),
                    f32::from_le_bytes(bytes[13..17].try_into().ok()?),
                    f32::from_le_bytes(bytes[17..21].try_into().ok()?),
                ];
                let rotation = [
                    f32::from_le_bytes(bytes[21..25].try_into().ok()?),
                    f32::from_le_bytes(bytes[25..29].try_into().ok()?),
                    f32::from_le_bytes(bytes[29..33].try_into().ok()?),
                    f32::from_le_bytes(bytes[33..37].try_into().ok()?),
                ];
                Some(NetworkMessage::EntityUpdate {
                    entity_id,
                    position,
                    rotation,
                })
            }
            2 => {
                if bytes.len() < 1 + 8 + 4 {
                    return None;
                }
                let client_tick = u64::from_le_bytes(bytes[1..9].try_into().ok()?);
                let len = u32::from_le_bytes(bytes[9..13].try_into().ok()?) as usize;
                if bytes.len() < 13 + len {
                    return None;
                }
                let input_data = bytes[13..13 + len].to_vec();
                Some(NetworkMessage::PlayerInput {
                    client_tick,
                    input_data,
                })
            }
            3 => {
                if bytes.len() < 5 {
                    return None;
                }
                let sender_len = u32::from_le_bytes(bytes[1..5].try_into().ok()?) as usize;
                if bytes.len() < 5 + sender_len + 4 {
                    return None;
                }
                let sender = String::from_utf8_lossy(&bytes[5..5 + sender_len]).to_string();
                let text_start = 5 + sender_len;
                let text_len =
                    u32::from_le_bytes(bytes[text_start..text_start + 4].try_into().ok()?) as usize;
                if bytes.len() < text_start + 4 + text_len {
                    return None;
                }
                let text =
                    String::from_utf8_lossy(&bytes[text_start + 4..text_start + 4 + text_len])
                        .to_string();
                Some(NetworkMessage::Chat { sender, text })
            }
            4 => {
                if bytes.len() < 5 {
                    return None;
                }
                let len = u32::from_le_bytes(bytes[1..5].try_into().ok()?) as usize;
                if bytes.len() < 5 + len {
                    return None;
                }
                let reason = String::from_utf8_lossy(&bytes[5..5 + len]).to_string();
                Some(NetworkMessage::Disconnect { reason })
            }
            5 => Some(NetworkMessage::Heartbeat),
            6 => {
                let mut offset = 1;
                if bytes.len() < offset + 8 {
                    return None;
                }
                let tick = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
                offset += 8;

                let entities = deserialize_entity_components(bytes, &mut offset)?;
                let despawned = deserialize_despawned(bytes, &mut offset)?;

                Some(NetworkMessage::StateSnapshot {
                    tick,
                    entities,
                    despawned,
                })
            }
            7 => {
                let mut offset = 1;
                if bytes.len() < offset + 8 {
                    return None;
                }
                let tick = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
                offset += 8;

                let changed = deserialize_entity_components(bytes, &mut offset)?;
                let despawned = deserialize_despawned(bytes, &mut offset)?;

                Some(NetworkMessage::DeltaSnapshot {
                    tick,
                    changed,
                    despawned,
                })
            }
            8 => {
                if bytes.len() < 1 + 8 + 8 {
                    return None;
                }
                let client_tick = u64::from_le_bytes(bytes[1..9].try_into().ok()?);
                let server_tick = u64::from_le_bytes(bytes[9..17].try_into().ok()?);
                Some(NetworkMessage::InputAck {
                    client_tick,
                    server_tick,
                })
            }
            9 => {
                let mut offset = 1;
                if bytes.len() < offset + 8 {
                    return None;
                }
                let tick = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
                offset += 8;

                let entities = deserialize_entity_components(bytes, &mut offset)?;

                Some(NetworkMessage::Correction { tick, entities })
            }
            10 => {
                let mut offset = 1;
                if bytes.len() < offset + 8 {
                    return None;
                }
                let client_id = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
                offset += 8;
                if bytes.len() < offset + 8 {
                    return None;
                }
                let tick = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
                offset += 8;
                let entities = deserialize_entity_components(bytes, &mut offset)?;
                let despawned = deserialize_despawned(bytes, &mut offset)?;
                if bytes.len() < offset + 8 {
                    return None;
                }
                let missed_ticks = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
                Some(NetworkMessage::ReconnectSnapshot {
                    client_id,
                    tick,
                    entities,
                    despawned,
                    missed_ticks,
                })
            }
            11 => {
                if bytes.len() < 1 + 8 + 8 + 8 {
                    return None;
                }
                let client_id = u64::from_le_bytes(bytes[1..9].try_into().ok()?);
                let reconnect_token = u64::from_le_bytes(bytes[9..17].try_into().ok()?);
                let last_tick = u64::from_le_bytes(bytes[17..25].try_into().ok()?);
                Some(NetworkMessage::ReconnectRequest {
                    client_id,
                    reconnect_token,
                    last_tick,
                })
            }
            12 => {
                if bytes.len() < 1 + 8 + 8 {
                    return None;
                }
                let client_id = u64::from_le_bytes(bytes[1..9].try_into().ok()?);
                let server_tick = u64::from_le_bytes(bytes[9..17].try_into().ok()?);
                Some(NetworkMessage::ReconnectAck {
                    client_id,
                    server_tick,
                })
            }
            13 => {
                if bytes.len() < 1 + 8 + 4 {
                    return None;
                }
                let client_tick = u64::from_le_bytes(bytes[1..9].try_into().ok()?);
                let len = u32::from_le_bytes(bytes[9..13].try_into().ok()?) as usize;
                if bytes.len() < 13 + len + 1 {
                    return None;
                }
                let input_data = bytes[13..13 + len].to_vec();
                let is_predicted = bytes[13 + len] != 0;
                Some(NetworkMessage::PredictedInput {
                    client_tick,
                    input_data,
                    is_predicted,
                })
            }
            14 => {
                let mut offset = 1;
                if bytes.len() < offset + 4 {
                    return None;
                }
                let name_len =
                    u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
                offset += 4;
                if bytes.len() < offset + name_len {
                    return None;
                }
                let name = String::from_utf8_lossy(&bytes[offset..offset + name_len]).to_string();
                offset += name_len;
                if bytes.len() < offset + 4 {
                    return None;
                }
                let max_players = u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?);
                offset += 4;
                if bytes.len() < offset + 4 {
                    return None;
                }
                let mode_len =
                    u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
                offset += 4;
                if bytes.len() < offset + mode_len {
                    return None;
                }
                let game_mode =
                    String::from_utf8_lossy(&bytes[offset..offset + mode_len]).to_string();
                Some(NetworkMessage::CreateRoom {
                    name,
                    max_players,
                    game_mode,
                })
            }
            15 => {
                if bytes.len() < 1 + 8 {
                    return None;
                }
                let room_id = u64::from_le_bytes(bytes[1..9].try_into().ok()?);
                Some(NetworkMessage::JoinRoom { room_id })
            }
            16 => Some(NetworkMessage::LeaveRoom),
            17 => {
                let mut offset = 1;
                if bytes.len() < offset + 4 {
                    return None;
                }
                let room_count =
                    u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
                offset += 4;
                let mut rooms = Vec::with_capacity(room_count);
                for _ in 0..room_count {
                    if bytes.len() < offset + 8 {
                        return None;
                    }
                    let id = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
                    offset += 8;
                    if bytes.len() < offset + 4 {
                        return None;
                    }
                    let name_len =
                        u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
                    offset += 4;
                    if bytes.len() < offset + name_len {
                        return None;
                    }
                    let name =
                        String::from_utf8_lossy(&bytes[offset..offset + name_len]).to_string();
                    offset += name_len;
                    if bytes.len() < offset + 8 {
                        return None;
                    }
                    let player_count =
                        u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?);
                    offset += 4;
                    let max_players =
                        u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?);
                    offset += 4;
                    if bytes.len() < offset + 4 {
                        return None;
                    }
                    let mode_len =
                        u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
                    offset += 4;
                    if bytes.len() < offset + mode_len + 1 {
                        return None;
                    }
                    let game_mode =
                        String::from_utf8_lossy(&bytes[offset..offset + mode_len]).to_string();
                    offset += mode_len;
                    let state = match bytes[offset] {
                        0 => RoomState::Waiting,
                        1 => RoomState::InGame,
                        _ => RoomState::Closed,
                    };
                    offset += 1;
                    rooms.push(RoomInfo {
                        id,
                        name,
                        player_count,
                        max_players,
                        game_mode,
                        state,
                    });
                }
                Some(NetworkMessage::RoomList { rooms })
            }
            18 => {
                let mut offset = 1;
                if bytes.len() < offset + 8 {
                    return None;
                }
                let room_id = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
                offset += 8;
                if bytes.len() < offset + 4 {
                    return None;
                }
                let count = u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
                offset += 4;
                let mut player_ids = Vec::with_capacity(count);
                for _ in 0..count {
                    if bytes.len() < offset + 8 {
                        return None;
                    }
                    player_ids.push(u64::from_le_bytes(
                        bytes[offset..offset + 8].try_into().ok()?,
                    ));
                    offset += 8;
                }
                Some(NetworkMessage::MatchFound {
                    room_id,
                    player_ids,
                })
            }
            19 => {
                let mut offset = 1;
                if bytes.len() < offset + 8 {
                    return None;
                }
                let room_id = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
                offset += 8;
                if bytes.len() < offset + 4 {
                    return None;
                }
                let count = u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
                offset += 4;
                let mut player_ids = Vec::with_capacity(count);
                for _ in 0..count {
                    if bytes.len() < offset + 8 {
                        return None;
                    }
                    player_ids.push(u64::from_le_bytes(
                        bytes[offset..offset + 8].try_into().ok()?,
                    ));
                    offset += 8;
                }
                let mut ready_states = Vec::with_capacity(count);
                for _ in 0..count {
                    if bytes.len() < offset + 1 {
                        return None;
                    }
                    ready_states.push(bytes[offset] != 0);
                    offset += 1;
                }
                Some(NetworkMessage::LobbyUpdate {
                    room_id,
                    player_ids,
                    ready_states,
                })
            }
            20 => {
                if bytes.len() < 2 {
                    return None;
                }
                Some(NetworkMessage::ReadyUp {
                    ready: bytes[1] != 0,
                })
            }
            21 => {
                if bytes.len() < 1 + 8 {
                    return None;
                }
                let client_id = u64::from_le_bytes(bytes[1..9].try_into().ok()?);
                Some(NetworkMessage::RendezvousRegister { client_id })
            }
            22 => {
                let mut offset = 1;
                if bytes.len() < offset + 4 {
                    return None;
                }
                let addr_len =
                    u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
                offset += 4;
                if bytes.len() < offset + addr_len + 8 {
                    return None;
                }
                let peer_addr =
                    String::from_utf8_lossy(&bytes[offset..offset + addr_len]).to_string();
                offset += addr_len;
                let peer_id = u64::from_le_bytes(bytes[offset..offset + 8].try_into().ok()?);
                Some(NetworkMessage::RendezvousPair { peer_addr, peer_id })
            }
            23 => {
                if bytes.len() < 2 {
                    return None;
                }
                let success = bytes[1] != 0;
                let mut offset = 2;
                if bytes.len() < offset + 4 {
                    return None;
                }
                let addr_len =
                    u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
                offset += 4;
                if bytes.len() < offset + addr_len {
                    return None;
                }
                let public_addr =
                    String::from_utf8_lossy(&bytes[offset..offset + addr_len]).to_string();
                Some(NetworkMessage::RendezvousResult {
                    success,
                    public_addr,
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handshake_roundtrip() {
        let msg = NetworkMessage::Handshake {
            client_id: Some(42),
            version: "1.0".to_string(),
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::Handshake { client_id, version } => {
                assert_eq!(client_id, Some(42));
                assert_eq!(version, "1.0");
            }
            _ => panic!("wrong message type"),
        }
    }

    #[test]
    fn test_entity_update_roundtrip() {
        let msg = NetworkMessage::EntityUpdate {
            entity_id: 1,
            position: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::EntityUpdate {
                entity_id,
                position,
                rotation,
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(position, [1.0, 2.0, 3.0]);
                assert_eq!(rotation, [0.0, 0.0, 0.0, 1.0]);
            }
            _ => panic!("wrong message type"),
        }
    }

    #[test]
    fn test_player_input_roundtrip() {
        let msg = NetworkMessage::PlayerInput {
            client_tick: 42,
            input_data: vec![1, 2, 3, 4],
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::PlayerInput {
                client_tick,
                input_data,
            } => {
                assert_eq!(client_tick, 42);
                assert_eq!(input_data, vec![1, 2, 3, 4]);
            }
            _ => panic!("wrong message type"),
        }
    }

    #[test]
    fn test_chat_roundtrip() {
        let msg = NetworkMessage::Chat {
            sender: "player1".to_string(),
            text: "hello world".to_string(),
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::Chat { sender, text } => {
                assert_eq!(sender, "player1");
                assert_eq!(text, "hello world");
            }
            _ => panic!("wrong message type"),
        }
    }

    #[test]
    fn test_disconnect_roundtrip() {
        let msg = NetworkMessage::Disconnect {
            reason: "server shutdown".to_string(),
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::Disconnect { reason } => {
                assert_eq!(reason, "server shutdown");
            }
            _ => panic!("wrong message type"),
        }
    }

    #[test]
    fn test_heartbeat_roundtrip() {
        let msg = NetworkMessage::Heartbeat;
        let bytes = msg.serialize();
        assert_eq!(bytes, vec![5]);
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        assert!(matches!(deserialized, NetworkMessage::Heartbeat));
    }

    #[test]
    fn test_deserialize_empty() {
        assert!(NetworkMessage::deserialize(&[]).is_none());
    }

    #[test]
    fn test_deserialize_unknown_type() {
        assert!(NetworkMessage::deserialize(&[255]).is_none());
    }

    #[test]
    fn test_state_snapshot_roundtrip() {
        let msg = NetworkMessage::StateSnapshot {
            tick: 42,
            entities: vec![(
                0,
                vec![(
                    123,
                    vec![
                        1.0f32.to_le_bytes().to_vec(),
                        2.0f32.to_le_bytes().to_vec(),
                        3.0f32.to_le_bytes().to_vec(),
                    ]
                    .concat(),
                )],
            )],
            despawned: vec![5],
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::StateSnapshot {
                tick,
                entities,
                despawned,
            } => {
                assert_eq!(tick, 42);
                assert_eq!(entities.len(), 1);
                assert_eq!(entities[0].0, 0);
                assert_eq!(entities[0].1.len(), 1);
                assert_eq!(entities[0].1[0].0, 123);
                assert_eq!(despawned, vec![5]);
            }
            _ => panic!("wrong message type"),
        }
    }

    #[test]
    fn test_state_snapshot_empty() {
        let msg = NetworkMessage::StateSnapshot {
            tick: 0,
            entities: vec![],
            despawned: vec![],
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        assert!(matches!(
            deserialized,
            NetworkMessage::StateSnapshot {
                tick: 0,
                ref entities,
                ref despawned,
            } if entities.is_empty() && despawned.is_empty()
        ));
    }

    #[test]
    fn test_state_snapshot_multiple_entities() {
        let msg = NetworkMessage::StateSnapshot {
            tick: 10,
            entities: vec![
                (0, vec![(1, vec![1, 2, 3])]),
                (1, vec![(1, vec![4, 5, 6]), (2, vec![7, 8])]),
            ],
            despawned: vec![2, 3],
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::StateSnapshot {
                tick,
                entities,
                despawned,
            } => {
                assert_eq!(tick, 10);
                assert_eq!(entities.len(), 2);
                assert_eq!(entities[0].1.len(), 1);
                assert_eq!(entities[1].1.len(), 2);
                assert_eq!(despawned, vec![2, 3]);
            }
            _ => panic!("wrong message type"),
        }
    }

    #[test]
    fn test_delta_snapshot_roundtrip() {
        let msg = NetworkMessage::DeltaSnapshot {
            tick: 10,
            changed: vec![(1, vec![(2, vec![42])])],
            despawned: vec![],
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::DeltaSnapshot {
                tick,
                changed,
                despawned,
            } => {
                assert_eq!(tick, 10);
                assert_eq!(changed.len(), 1);
                assert_eq!(changed[0].0, 1);
                assert!(despawned.is_empty());
            }
            _ => panic!("wrong message type"),
        }
    }

    #[test]
    fn test_input_ack_roundtrip() {
        let msg = NetworkMessage::InputAck {
            client_tick: 5,
            server_tick: 100,
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::InputAck {
                client_tick,
                server_tick,
            } => {
                assert_eq!(client_tick, 5);
                assert_eq!(server_tick, 100);
            }
            _ => panic!("wrong message type"),
        }
    }

    #[test]
    fn test_correction_roundtrip() {
        let msg = NetworkMessage::Correction {
            tick: 50,
            entities: vec![(0, vec![(1, vec![9, 8, 7])])],
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::Correction { tick, entities } => {
                assert_eq!(tick, 50);
                assert_eq!(entities.len(), 1);
                assert_eq!(entities[0].1[0].1, vec![9, 8, 7]);
            }
            _ => panic!("wrong message type"),
        }
    }
}
