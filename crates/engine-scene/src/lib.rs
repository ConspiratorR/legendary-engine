//! Scene graph with ECS-backed hierarchy and transforms.
//!
//! Provides [`scene_manager::SceneManager`] for building a tree of [`node::SceneNode`]s with
//! parent/child relationships, local [`transform::Transform`]s, and computed
//! [`transform::GlobalTransform`]s. Also includes skeletal animation, inverse
//! kinematics, and keyframe animation systems.
//!
//! The [`serialization`] module provides scene file I/O in JSON, RON, and binary formats.
//!
//! The [`prefab`] module provides reusable entity templates with instantiation,
//! property overrides, and nested prefab support.
//!
//! The [`multi_scene`] module manages multiple scenes loaded simultaneously,
//! merging their entities with namespaced IDs.
//!
//! The [`sub_scene`] module provides distance-based streaming of sub-scenes.
//!
//! The [`scene_layer`] module provides bitmask-based scene categorization.

pub mod animation_state;
pub mod diff;
pub mod hierarchy;
pub mod ik;
pub mod keyframe;
pub mod multi_scene;
pub mod node;
pub mod prefab;
pub mod prefab_instantiate;
pub mod prefab_registry;
pub mod scene_layer;
pub mod scene_manager;
pub mod serialization;
pub mod skeleton;
pub mod sub_scene;
pub mod transform;
