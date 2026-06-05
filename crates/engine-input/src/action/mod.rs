//! Action mapping: abstracts physical inputs into named game actions.
//!
//! [`ActionMap`] stores bindings from keys/axes to action names, and
//! [`Binding`] describes a single physical-to-action mapping.

pub mod action_map;
pub mod binding;
pub use action_map::{ActionMap, ActionState, action_update_system};
pub use binding::Binding;
