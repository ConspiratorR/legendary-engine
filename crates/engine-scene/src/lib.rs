//! Scene graph with ECS-backed hierarchy and transforms.
//!
//! Provides [`scene_manager::SceneManager`] for building a tree of [`node::SceneNode`]s with
//! parent/child relationships, local [`transform::Transform`]s, and computed
//! [`transform::GlobalTransform`]s. Also includes skeletal animation, inverse
//! kinematics, and keyframe animation systems.
//!
//! The [`serialization`] module provides scene file I/O in JSON, RON, and binary formats.

pub mod animation_state;
pub mod hierarchy;
pub mod ik;
pub mod keyframe;
pub mod node;
pub mod scene_manager;
pub mod serialization;
pub mod skeleton;
pub mod transform;
