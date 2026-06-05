//! # engine-scene
//!
//! Scene management for the RustEngine — scene graph, transforms, animation,
//! serialization, and streaming.
//!
//! ## Scene Graph Model
//!
//! The scene is a **tree of [`SceneNode`](node::SceneNode)s**, each backed by
//! an ECS [`Entity`](engine_ecs::entity::Entity). Parent-child relationships
//! are stored as ECS components:
//!
//! - [`Parent`](hierarchy::Parent) — links a child to its parent.
//! - [`Children`](hierarchy::Children) — lists a node's direct children.
//!
//! The [`SceneManager`](scene_manager::SceneManager) owns the ECS
//! [`World`](engine_ecs::world::World) and provides convenience methods for
//! building and querying the hierarchy.
//!
//! ## Transform Propagation
//!
//! Every node has two transform components:
//!
//! | Component | Coordinate space | Set by user |
//! |---|---|---|
//! | [`Transform`](transform::Transform) | Local (relative to parent) | Yes |
//! | [`GlobalTransform`](transform::GlobalTransform) | World-space (absolute) | No — computed |
//!
//! After modifying any [`Transform`], call
//! [`SceneManager::sync_transforms`] to recompute all
//! [`GlobalTransform`]s. The sync walks the tree top-down, multiplying
//! each local transform by its parent's global transform:
//!
//! ```text
//! global(child) = global(parent) * local(child)
//! ```
//!
//! This means **rotation and scale propagate**: a parent scaled 2×
//! with a child at local (1, 0, 0) produces a child global position
//! of (2, 0, 0).
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
//! | Module | Description |
//! |---|---|
//! | [`serialization`] | Scene file I/O in JSON, RON, and binary formats. |
//! | [`prefab`] | Reusable entity templates with instantiation and property overrides. |
//! | [`multi_scene`] | Multiple scenes loaded simultaneously with namespaced IDs. |
//! | [`sub_scene`] | Distance-based streaming of sub-scenes. |
//! | [`scene_layer`] | Bitmask-based scene categorization. |
//! | [`skeleton`] | Skeletal animation with joint hierarchies and skinning. |
//! | [`ik`] | Inverse kinematics (CCD) and forward kinematics solvers. |
//! | [`keyframe`] | Keyframe animation clips and interpolation. |
//! | [`animation_state`] | Animation state machine with blend transitions. |
//! | [`diff`] | Scene diffing for incremental serialization. |
//! | [`hierarchy`] | `Parent` and `Children` ECS components. |
//! | [`node`] | Lightweight `SceneNode` handle wrapping an `Entity`. |
//! | [`transform`] | `Transform` (local) and `GlobalTransform` (world-space). |
//! | [`error`] | Shared error types. |

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
