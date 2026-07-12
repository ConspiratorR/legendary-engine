use serde::{Deserialize, Serialize};
use std::fmt;

/// Lightweight handle to a GameObject (index + generation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameObjectHandle {
    index: u32,
    generation: u32,
}

impl GameObjectHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Get the index (for internal use).
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Get the generation (for internal use).
    pub fn generation(&self) -> u32 {
        self.generation
    }
}

impl fmt::Display for GameObjectHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GameObject({}:{})", self.index, self.generation)
    }
}
