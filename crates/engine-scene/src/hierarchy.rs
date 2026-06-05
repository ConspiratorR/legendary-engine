//! Parent-child hierarchy ECS components.
//!
//! These components form the edges of the scene graph tree. Every scene node
//! (except the root) has a [`Parent`] component, and every node that has
//! children has a [`Children`] component.
//!
//! Prefer using [`SceneManager`](super::scene_manager::SceneManager) to
//! manipulate these — it keeps `Parent` and `Children` in sync.

use engine_ecs::entity::Entity;
use serde::{Deserialize, Serialize};

/// Component linking a child entity to its parent in the scene graph.
///
/// A node has at most one parent. The root node has no `Parent` component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parent(pub Entity);

/// Component listing the child entities of a scene node.
///
/// Order is insertion order (the order children were added via
/// [`SceneManager::set_parent`](super::scene_manager::SceneManager::set_parent)).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Children(pub Vec<Entity>);

impl Children {
    /// Create an empty children list.
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl Default for Children {
    fn default() -> Self {
        Self::new()
    }
}
