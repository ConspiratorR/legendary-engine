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
    PlayerInput {
        input_data: Vec<u8>,
    },
    /// Chat message.
    Chat {
        sender: String,
        text: String,
    },
    /// Disconnect notification.
    Disconnect {
        reason: String,
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

impl NetworkMessage {
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
            NetworkMessage::EntityUpdate { entity_id, position, rotation } => {
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
            _ => vec![],
        }
    }

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
                Some(NetworkMessage::EntityUpdate { entity_id, position, rotation })
            }
            _ => None,
        }
    }
}
