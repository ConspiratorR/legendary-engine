//! Type-erased resource registry for global singletons.

use std::any::TypeId;
use std::collections::HashMap;

/// A type-erased registry for global resources (singletons).
///
/// Resources are identified by [`TypeId`] and stored as `Box<dyn Any>`.
/// This is separate from the ECS [`World`](engine_ecs::world::World)'s
/// resource storage and is used for editor/framework-level state.
pub struct ResourceRegistry {
    resources: HashMap<TypeId, Box<dyn std::any::Any>>,
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    /// Insert a resource, replacing any existing value of the same type.
    pub fn insert<T: 'static>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    /// Get a shared reference to a resource.
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.resources.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    /// Get an exclusive reference to a resource.
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<T>()
    }
}
