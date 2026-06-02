//! Scene layer system for categorizing and filtering scenes.
//!
//! Each scene can be assigned one or more [`SceneLayer`]s using a bitmask.
//! Layers control visibility, rendering order, and interaction between
//! multiple simultaneously loaded scenes.
//!
//! # Example
//!
//! ```rust
//! use engine_scene::scene_layer::SceneLayer;
//!
//! let layers = SceneLayer::DEFAULT | SceneLayer::ENVIRONMENT;
//! assert!(layers.contains(SceneLayer::DEFAULT));
//! assert!(!layers.contains(SceneLayer::UI));
//! ```

use serde::{Deserialize, Serialize};

/// A bitmask of scene layers.
///
/// Each bit represents a layer that a scene can belong to.
/// Scenes can belong to multiple layers simultaneously.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SceneLayer(pub u32);

impl SceneLayer {
    /// The default layer (always present).
    pub const DEFAULT: Self = Self(1 << 0);
    /// Environment / world geometry.
    pub const ENVIRONMENT: Self = Self(1 << 1);
    /// Gameplay objects (players, items, triggers).
    pub const GAMEPLAY: Self = Self(1 << 2);
    /// User interface overlay.
    pub const UI: Self = Self(1 << 3);
    /// Lighting and atmospheric effects.
    pub const LIGHTING: Self = Self(1 << 4);
    /// Post-processing and debug overlays.
    pub const POST_PROCESS: Self = Self(1 << 5);

    /// No layers.
    pub const NONE: Self = Self(0);
    /// All layers.
    pub const ALL: Self = Self(u32::MAX);

    /// Create a layer from a raw bitmask.
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Return the raw bitmask.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Check if `self` contains all bits in `other`.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Check if `self` intersects any bit in `other`.
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    /// Create a layer from a named index (0–31).
    ///
    /// Returns `None` if `index >= 32`.
    pub const fn from_index(index: u8) -> Option<Self> {
        if index < 32 {
            Some(Self(1 << index))
        } else {
            None
        }
    }

    /// Return the index of the lowest set bit, or `None` if empty.
    pub const fn first_layer_index(self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            Some(self.0.trailing_zeros() as u8)
        }
    }

    /// Iterate over all set layer indices.
    pub fn iter(self) -> SceneLayerIter {
        SceneLayerIter { bits: self.0 }
    }
}

impl Default for SceneLayer {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::ops::BitOr for SceneLayer {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for SceneLayer {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitOrAssign for SceneLayer {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAndAssign for SceneLayer {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl std::ops::Not for SceneLayer {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

/// Iterator over the set bits (layer indices) of a [`SceneLayer`].
pub struct SceneLayerIter {
    bits: u32,
}

impl Iterator for SceneLayerIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bits == 0 {
            None
        } else {
            let index = self.bits.trailing_zeros() as u8;
            self.bits &= self.bits - 1;
            Some(index)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_layer() {
        let layer = SceneLayer::DEFAULT;
        assert!(layer.contains(SceneLayer::DEFAULT));
        assert!(!layer.contains(SceneLayer::UI));
    }

    #[test]
    fn test_combined_layers() {
        let layers = SceneLayer::DEFAULT | SceneLayer::ENVIRONMENT | SceneLayer::GAMEPLAY;
        assert!(layers.contains(SceneLayer::DEFAULT));
        assert!(layers.contains(SceneLayer::ENVIRONMENT));
        assert!(layers.contains(SceneLayer::GAMEPLAY));
        assert!(!layers.contains(SceneLayer::UI));
    }

    #[test]
    fn test_intersects() {
        let a = SceneLayer::DEFAULT | SceneLayer::ENVIRONMENT;
        let b = SceneLayer::ENVIRONMENT | SceneLayer::UI;
        assert!(a.intersects(b));

        let c = SceneLayer::UI | SceneLayer::LIGHTING;
        assert!(!a.intersects(c));
    }

    #[test]
    fn test_not() {
        let layer = SceneLayer::DEFAULT;
        let inverted = !layer;
        assert!(!inverted.contains(SceneLayer::DEFAULT));
        assert!(inverted.contains(SceneLayer::ENVIRONMENT));
    }

    #[test]
    fn test_from_index() {
        let layer = SceneLayer::from_index(1).unwrap();
        assert_eq!(layer, SceneLayer::ENVIRONMENT);
        assert!(SceneLayer::from_index(32).is_none());
    }

    #[test]
    fn test_first_layer_index() {
        assert_eq!(SceneLayer::DEFAULT.first_layer_index(), Some(0));
        assert_eq!(SceneLayer::ENVIRONMENT.first_layer_index(), Some(1));
        assert_eq!(SceneLayer::NONE.first_layer_index(), None);
    }

    #[test]
    fn test_iter() {
        let layers = SceneLayer::DEFAULT | SceneLayer::ENVIRONMENT | SceneLayer::UI;
        let indices: Vec<u8> = layers.iter().collect();
        assert_eq!(indices, vec![0, 1, 3]);
    }

    #[test]
    fn test_none_and_all() {
        assert_eq!(SceneLayer::NONE.bits(), 0);
        assert_eq!(SceneLayer::ALL.bits(), u32::MAX);
        assert!(SceneLayer::ALL.contains(SceneLayer::DEFAULT));
        assert!(!SceneLayer::NONE.contains(SceneLayer::DEFAULT));
    }

    #[test]
    fn test_default_is_default_layer() {
        let layer = SceneLayer::default();
        assert_eq!(layer, SceneLayer::DEFAULT);
    }
}
