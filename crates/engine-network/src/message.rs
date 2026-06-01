//! Network message types and serialization.

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
    /// Player input.
    PlayerInput { input_data: Vec<u8> },
    /// Chat message.
    Chat { sender: String, text: String },
    /// Disconnect notification.
    Disconnect { reason: String },
    /// Heartbeat keepalive.
    Heartbeat,
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
            NetworkMessage::PlayerInput { input_data } => {
                let mut bytes = vec![2];
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
                if bytes.len() < 5 {
                    return None;
                }
                let len = u32::from_le_bytes(bytes[1..5].try_into().ok()?) as usize;
                if bytes.len() < 5 + len {
                    return None;
                }
                let input_data = bytes[5..5 + len].to_vec();
                Some(NetworkMessage::PlayerInput { input_data })
            }
            3 => {
                if bytes.len() < 5 {
                    return None;
                }
                let sender_len = u32::from_le_bytes(bytes[1..5].try_into().ok()?) as usize;
                if bytes.len() < 5 + sender_len + 4 {
                    return None;
                }
                let sender =
                    String::from_utf8_lossy(&bytes[5..5 + sender_len]).to_string();
                let text_start = 5 + sender_len;
                let text_len =
                    u32::from_le_bytes(bytes[text_start..text_start + 4].try_into().ok()?)
                        as usize;
                if bytes.len() < text_start + 4 + text_len {
                    return None;
                }
                let text = String::from_utf8_lossy(
                    &bytes[text_start + 4..text_start + 4 + text_len],
                )
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
            input_data: vec![1, 2, 3, 4],
        };
        let bytes = msg.serialize();
        let deserialized = NetworkMessage::deserialize(&bytes).unwrap();
        match deserialized {
            NetworkMessage::PlayerInput { input_data } => {
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
}
