//! Unity Object trait — base class for all Unity objects.
//!
//! Maps to `UnityEngine.Object` in Unity's documentation.
//!
//! Every object that can be referenced in Unity inherits from Object.
//! This includes GameObject, Component, ScriptableObject, Material, Texture, Mesh, etc.

use std::fmt;

/// Instance ID for objects (matches Unity's int instance ID).
pub type InstanceId = i32;

/// Base trait for all Unity objects (matches `UnityEngine.Object`).
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/Object.html>
///
/// In Unity, `UnityEngine.Object` is the base class for all Unity objects.
/// It provides:
/// - `name` property (read/write)
/// - `hideFlags` property
/// - `GetInstanceID()` method
/// - Static methods: `Destroy`, `DestroyImmediate`, `Instantiate`, `FindObjectOfType`, etc.
///
/// In Rust, this is a trait because Rust doesn't have class inheritance.
/// All types that can exist in a Unity scene should implement this trait.
pub trait Object: Send + Sync {
    /// Get the name of this object.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object-name.html>
    fn Name(&self) -> &str;

    /// Set the name of this object.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object-name.html>
    fn SetName(&mut self, name: &str);

    /// Get the instance ID of this object.
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.GetInstanceID.html>
    ///
    /// Returns a unique identifier for this object instance.
    /// In Unity this returns `int`, here we use `InstanceId` (i32).
    fn GetInstanceID(&self) -> InstanceId;

    /// Convert to string representation (matches Unity's `ToString()`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.ToString.html>
    fn ToString(&self) -> String {
        self.Name().to_string()
    }

    /// Implicit bool conversion — checks if the object exists (is not null/destroyed).
    ///
    /// # Unity Documentation
    /// Unity's `Object` has an implicit bool operator that returns false
    /// for destroyed objects. In Rust, this is checked via `Option<T>`.
    /// Use `is_valid()` on World to check if a handle is still valid.
    fn is_valid(&self) -> bool {
        true
    }
}

/// Static methods for Unity Object (matches `UnityEngine.Object` static methods).
///
/// In Unity, these are static methods on `Object`. In Rust, they're implemented
/// on a `World` reference since they need access to the object storage.
///
/// # Unity Documentation
/// <https://docs.unity3d.com/ScriptReference/Object.html>
pub struct ObjectUtil;

impl ObjectUtil {
    /// Destroys a GameObject, Component, or asset (matches Unity's `Object.Destroy`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.Destroy.html>
    ///
    /// The object is destroyed after the current Update loop completes (deferred).
    /// For immediate destruction, use `DestroyImmediate`.
    pub fn Destroy() {
        // Handled by World::Destroy()
    }

    /// Destroys a GameObject, Component, or asset after a delay (matches Unity's `Object.Destroy`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.Destroy.html>
    pub fn DestroyDelayed(_t: f32) {
        // Handled by World with delay tracking
    }

    /// Destroys an object immediately (matches Unity's `Object.DestroyImmediate`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.DestroyImmediate.html>
    ///
    /// Unlike `Destroy`, this happens immediately within the call.
    pub fn DestroyImmediate() {
        // Handled by World::DestroyImmediate()
    }

    /// Prevents a GameObject from being destroyed when loading a new scene (matches Unity's `Object.DontDestroyOnLoad`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.DontDestroyOnLoad.html>
    pub fn DontDestroyOnLoad() {
        // Handled by World
    }

    /// Finds any loaded object of the given type (matches Unity's `Object.FindObjectOfType`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.FindObjectOfType.html>
    ///
    /// Returns the first active loaded object of type T.
    pub fn FindObjectOfType<T>() -> Option<T>
    where
        T: Object + 'static,
    {
        // Handled by World::FindObjectOfType()
        None
    }

    /// Finds all loaded objects of the given type (matches Unity's `Object.FindObjectsOfType`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.FindObjectsOfType.html>
    pub fn FindObjectsOfType<T>() -> Vec<T>
    where
        T: Object + 'static,
    {
        // Handled by World::FindObjectsOfType()
        Vec::new()
    }

    /// Clones the object and returns the clone (matches Unity's `Object.Instantiate`).
    ///
    /// # Unity Documentation
    /// <https://docs.unity3d.com/ScriptReference/Object.Instantiate.html>
    ///
    /// In Unity this has many overloads. The basic version clones the object
    /// in the same position/rotation. Other overloads allow specifying position,
    /// rotation, and parent.
    pub fn Instantiate<T: Object + Clone>() -> T {
        // Handled by World::Instantiate()
        panic!("Instantiate must be called on a World instance")
    }
}

/// Null object pattern for Unity's "destroyed object" state.
///
/// In Unity, destroyed objects return `null` when referenced. In Rust,
/// we use `Option<T>` instead. This struct provides a helper for
/// checking if a reference is still valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NullObject;

impl fmt::Display for NullObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "null")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_trait_basic() {
        #[derive(Debug)]
        struct TestObj {
            name: String,
            instance_id: InstanceId,
        }

        impl Object for TestObj {
            fn Name(&self) -> &str {
                &self.name
            }

            fn SetName(&mut self, name: &str) {
                self.name = name.to_string();
            }

            fn GetInstanceID(&self) -> InstanceId {
                self.instance_id
            }
        }

        let mut obj = TestObj {
            name: "TestObject".to_string(),
            instance_id: 1,
        };

        assert_eq!(obj.Name(), "TestObject");
        assert_eq!(obj.GetInstanceID(), 1);
        assert!(obj.is_valid());

        obj.SetName("Renamed");
        assert_eq!(obj.Name(), "Renamed");
        assert_eq!(obj.ToString(), "Renamed");
    }

    #[test]
    fn test_null_object_display() {
        assert_eq!(NullObject.to_string(), "null");
    }
}
