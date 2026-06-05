//! # engine-scene
//!
//! Scene management for the RustEngine.
//!
//! Provides a scene graph with parent-child hierarchy,
//! [`Transform`](transform::Transform)/[`GlobalTransform`](transform::GlobalTransform) synchronization,
//! and serialization support.
//!
//! ## Quick Start
//!
//! ```rust
//! use engine_scene::scene_manager::SceneManager;
//! use engine_scene::transform::Transform;
//!
//! let mut sm = SceneManager::new();
//! let root = sm.root();
//!
//! let child = sm.add_node("Child")
//!     .with_transform(Transform::from_xyz(0.0, 5.0, 0.0))
//!     .build();
//!
//! sm.set_parent(child, root);
//! sm.sync_transforms();
//! ```
//!
//! ## Modules
//!
//! - [`serialization`] — Scene file I/O in JSON, RON, and binary formats.
//! - [`prefab`] — Reusable entity templates with instantiation and property overrides.
//! - [`multi_scene`] — Multiple scenes loaded simultaneously with namespaced IDs.
//! - [`sub_scene`] — Distance-based streaming of sub-scenes.
//! - [`scene_layer`] — Bitmask-based scene categorization.
//! - [`skeleton`] — Skeletal animation with joint hierarchies and skinning.
//! - [`ik`] — Inverse kinematics (CCD) and forward kinematics solvers.
//! - [`keyframe`] — Keyframe animation clips and interpolation.
//! - [`animation_state`] — Animation state machine with blend transitions.
//! - [`diff`] — Scene diffing for incremental serialization.

pub mod animation_state;
pub mod diff;
pub mod error;
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

pub use error::SceneError;
