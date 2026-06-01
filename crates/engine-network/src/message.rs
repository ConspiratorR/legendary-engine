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
