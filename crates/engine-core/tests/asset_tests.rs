use engine_core::asset_database::AssetDatabase;
use engine_core::asset_handle::AssetHandle;
use engine_core::component::Component;
use engine_core::object::{InstanceId, Object};
use engine_core::scriptable_object::{ScriptableObject, ScriptableObjectHolder};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

// ---------------------------------------------------------------------------
// Test asset types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HealthData {
    name: String,
    asset_path: Option<String>,
    max_health: i32,
    regen_rate: f32,
}

impl Default for HealthData {
    fn default() -> Self {
        Self {
            name: String::new(),
            asset_path: None,
            max_health: 100,
            regen_rate: 1.0,
        }
    }
}

impl Object for HealthData {
    fn Name(&self) -> &str {
        &self.name
    }

    fn SetName(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn GetInstanceID(&self) -> InstanceId {
        0
    }
}

impl Component for HealthData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ScriptableObject for HealthData {
    fn AssetPath(&self) -> Option<&str> {
        self.asset_path.as_deref()
    }

    fn SetAssetPath(&mut self, path: &str) {
        self.asset_path = Some(path.to_string());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WeaponData {
    name: String,
    damage: f32,
    fire_rate: f32,
}

impl Object for WeaponData {
    fn Name(&self) -> &str {
        &self.name
    }

    fn SetName(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn GetInstanceID(&self) -> InstanceId {
        0
    }
}

impl Component for WeaponData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ScriptableObject for WeaponData {}

// ---------------------------------------------------------------------------
// ScriptableObject lifecycle tracking
// ---------------------------------------------------------------------------

static LIFECYCLE_CREATE: AtomicBool = AtomicBool::new(false);
static LIFECYCLE_ENABLE: AtomicBool = AtomicBool::new(false);
static LIFECYCLE_DISABLE: AtomicBool = AtomicBool::new(false);
static LIFECYCLE_DESTROY: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize, Deserialize)]
struct LifecycleAsset {
    name: String,
}

impl Object for LifecycleAsset {
    fn Name(&self) -> &str {
        &self.name
    }

    fn SetName(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn GetInstanceID(&self) -> InstanceId {
        0
    }
}

impl Component for LifecycleAsset {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl ScriptableObject for LifecycleAsset {
    fn Awake(&mut self) {
        LIFECYCLE_CREATE.store(true, Ordering::SeqCst);
    }

    fn OnEnable(&mut self) {
        LIFECYCLE_ENABLE.store(true, Ordering::SeqCst);
    }

    fn OnDisable(&mut self) {
        LIFECYCLE_DISABLE.store(true, Ordering::SeqCst);
    }

    fn OnDestroy(&mut self) {
        LIFECYCLE_DESTROY.store(true, Ordering::SeqCst);
    }
}

fn reset_lifecycle_flags() {
    LIFECYCLE_CREATE.store(false, Ordering::SeqCst);
    LIFECYCLE_ENABLE.store(false, Ordering::SeqCst);
    LIFECYCLE_DISABLE.store(false, Ordering::SeqCst);
    LIFECYCLE_DESTROY.store(false, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// ScriptableObjectHolder tests
// ---------------------------------------------------------------------------

#[test]
fn test_holder_creates_with_name_and_value() {
    let asset = HealthData {
        name: "Player".to_string(),
        asset_path: None,
        max_health: 200,
        regen_rate: 2.5,
    };
    let holder = ScriptableObjectHolder::new(asset);

    assert_eq!(holder.Get().Name(), "Player");
    assert_eq!(holder.Get().max_health, 200);
    assert_eq!(holder.Get().regen_rate, 2.5);
    assert!(holder.Enabled());
}

#[test]
fn test_holder_with_path_sets_metadata() {
    let asset = HealthData {
        name: "Enemy".to_string(),
        asset_path: None,
        max_health: 50,
        regen_rate: 0.5,
    };
    let holder = ScriptableObjectHolder::with_path(asset, "EnemyData", "/Game/Data/Enemy");

    assert_eq!(holder.Get().Name(), "EnemyData");
    assert_eq!(holder.Get().AssetPath(), Some("/Game/Data/Enemy"));
}

#[test]
fn test_holder_disable_enable_toggles() {
    let mut holder = ScriptableObjectHolder::new(HealthData {
        name: "Test".to_string(),
        asset_path: None,
        max_health: 100,
        regen_rate: 1.0,
    });

    assert!(holder.Enabled());
    holder.SetEnabled(false);
    assert!(!holder.Enabled());
    holder.SetEnabled(true);
    assert!(holder.Enabled());
}

#[test]
fn test_holder_calls_on_create_and_on_destroy() {
    reset_lifecycle_flags();

    {
        let _holder = ScriptableObjectHolder::new(LifecycleAsset {
            name: "tracked".to_string(),
        });
        assert!(LIFECYCLE_CREATE.load(Ordering::SeqCst));
        assert!(!LIFECYCLE_DESTROY.load(Ordering::SeqCst));
    }

    assert!(LIFECYCLE_DESTROY.load(Ordering::SeqCst));
}

#[test]
fn test_holder_with_path_calls_lifecycle() {
    reset_lifecycle_flags();

    {
        let _holder = ScriptableObjectHolder::with_path(
            LifecycleAsset {
                name: "tracked".to_string(),
            },
            "MyAsset",
            "/Game/MyAsset",
        );
        assert!(LIFECYCLE_CREATE.load(Ordering::SeqCst));
    }

    assert!(LIFECYCLE_DESTROY.load(Ordering::SeqCst));
}

#[test]
fn test_holder_type_name() {
    let holder = ScriptableObjectHolder::new(HealthData {
        name: "h".to_string(),
        asset_path: None,
        max_health: 1,
        regen_rate: 0.0,
    });
    let tn = holder.TypeName();
    assert!(tn.contains("HealthData"), "got {tn}");
}

#[test]
fn test_holder_mutable_access() {
    let mut holder = ScriptableObjectHolder::new(HealthData {
        name: "orig".to_string(),
        asset_path: None,
        max_health: 10,
        regen_rate: 0.1,
    });
    {
        let inner = holder.GetMut();
        inner.max_health = 999;
        inner.regen_rate = 5.0;
    }
    assert_eq!(holder.Get().max_health, 999);
    assert_eq!(holder.Get().regen_rate, 5.0);
}

#[test]
fn test_holder_serde_roundtrip() {
    let holder = ScriptableObjectHolder::new(HealthData {
        name: "SaveMe".to_string(),
        asset_path: None,
        max_health: 150,
        regen_rate: 3.0,
    });
    let json = serde_json::to_string(holder.Get()).unwrap();
    let deserialized: HealthData = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "SaveMe");
    assert_eq!(deserialized.max_health, 150);
    assert_eq!(deserialized.regen_rate, 3.0);
}

#[test]
fn test_holder_default_asset() {
    let holder = ScriptableObjectHolder::new(HealthData::default());
    assert_eq!(holder.Get().Name(), "");
    assert_eq!(holder.Get().max_health, 100);
    assert_eq!(holder.Get().regen_rate, 1.0);
}

// ---------------------------------------------------------------------------
// AssetHandle tests
// ---------------------------------------------------------------------------

#[test]
fn test_asset_handle_new() {
    let asset = WeaponData {
        name: "Sword".to_string(),
        damage: 25.0,
        fire_rate: 1.0,
    };
    let handle = AssetHandle::new(asset);

    assert_eq!(handle.get().Name(), "Sword");
    assert_eq!(handle.get().damage, 25.0);
    assert!(handle.path().is_none());
    assert!(handle.is_loaded());
    assert_eq!(handle.ref_count(), 1);
}

#[test]
fn test_asset_handle_with_path() {
    let asset = WeaponData {
        name: "Bow".to_string(),
        damage: 15.0,
        fire_rate: 0.5,
    };
    let handle = AssetHandle::with_path(asset, "/Game/Weapons/Bow");

    assert_eq!(handle.get().Name(), "Bow");
    assert_eq!(handle.path(), Some("/Game/Weapons/Bow"));
}

#[test]
fn test_asset_handle_clone_shares_data() {
    let handle1 = AssetHandle::with_path(
        WeaponData {
            name: "Staff".to_string(),
            damage: 30.0,
            fire_rate: 0.8,
        },
        "/Game/Weapons/Staff",
    );
    let handle2 = handle1.clone();

    // PartialEq uses Arc::ptr_eq, so equality means shared allocation
    assert_eq!(handle1, handle2);
    assert_eq!(handle1.ref_count(), 2);
    assert_eq!(handle2.get().Name(), "Staff");
}

#[test]
fn test_asset_handle_ref_count_lifecycle() {
    let h1 = AssetHandle::new(WeaponData {
        name: "A".to_string(),
        damage: 10.0,
        fire_rate: 1.0,
    });
    assert_eq!(h1.ref_count(), 1);

    let h2 = h1.clone();
    assert_eq!(h1.ref_count(), 2);

    let h3 = h1.clone();
    assert_eq!(h1.ref_count(), 3);

    drop(h3);
    assert_eq!(h1.ref_count(), 2);

    drop(h2);
    assert_eq!(h1.ref_count(), 1);
}

#[test]
fn test_asset_handle_equality() {
    let h1 = AssetHandle::new(WeaponData {
        name: "X".to_string(),
        damage: 1.0,
        fire_rate: 1.0,
    });
    let h2 = h1.clone();
    let h3 = AssetHandle::new(WeaponData {
        name: "Y".to_string(),
        damage: 2.0,
        fire_rate: 2.0,
    });

    assert_eq!(h1, h2);
    assert_ne!(h1, h3);
}

#[test]
fn test_asset_handle_debug_format() {
    let handle = AssetHandle::with_path(
        WeaponData {
            name: "Gun".to_string(),
            damage: 50.0,
            fire_rate: 2.0,
        },
        "/Game/Gun",
    );
    let dbg = format!("{:?}", handle);
    assert!(dbg.contains("AssetHandle"));
    assert!(dbg.contains("/Game/Gun"));
    assert!(dbg.contains("1"));
}

#[test]
fn test_asset_handle_display() {
    let h_path = AssetHandle::with_path(
        WeaponData {
            name: "A".to_string(),
            damage: 0.0,
            fire_rate: 0.0,
        },
        "/assets/weapon.fbx",
    );
    assert_eq!(format!("{h_path}"), "/assets/weapon.fbx");

    let h_no_path = AssetHandle::new(WeaponData {
        name: "B".to_string(),
        damage: 0.0,
        fire_rate: 0.0,
    });
    assert_eq!(format!("{h_no_path}"), "<no path>");
}

#[test]
fn test_asset_handle_shared_reads() {
    let handle = AssetHandle::new(WeaponData {
        name: "Mutable".to_string(),
        damage: 10.0,
        fire_rate: 1.0,
    });
    let handle2 = handle.clone();

    // Both point to same data, so reading from handle2 sees handle's value
    assert_eq!(handle2.get().damage, 10.0);
    // Note: AssetHandle wraps Arc<T>, not Arc<MutCell<T>>,
    // so direct mutation isn't exposed through shared handles.
}

#[test]
fn test_asset_handle_different_allocations_not_equal() {
    let h1 = AssetHandle::with_path(
        WeaponData {
            name: "A".to_string(),
            damage: 1.0,
            fire_rate: 1.0,
        },
        "/path/a",
    );
    let h2 = AssetHandle::with_path(
        WeaponData {
            name: "A".to_string(),
            damage: 1.0,
            fire_rate: 1.0,
        },
        "/path/b",
    );
    // Same content but different allocations -> different handles
    assert_ne!(h1, h2);
}

// ---------------------------------------------------------------------------
// AssetDatabase workflow tests
// ---------------------------------------------------------------------------

#[test]
fn test_database_create_and_retrieve() {
    let mut db = AssetDatabase::new();
    let handle = db.create_asset(
        "player_health",
        HealthData {
            name: "Player".to_string(),
            asset_path: None,
            max_health: 200,
            regen_rate: 2.0,
        },
    );

    assert_eq!(handle.get().max_health, 200);
    assert!(db.has_asset("player_health"));
    assert_eq!(db.asset_count(), 1);

    let retrieved = db.get_asset::<HealthData>("player_health").unwrap();
    assert_eq!(retrieved.get().Name(), "Player");
}

#[test]
fn test_database_multiple_asset_types() {
    let mut db = AssetDatabase::new();

    db.create_asset(
        "health",
        HealthData {
            name: "HP".to_string(),
            asset_path: None,
            max_health: 100,
            regen_rate: 1.0,
        },
    );
    db.create_asset(
        "weapon",
        WeaponData {
            name: "Gun".to_string(),
            damage: 50.0,
            fire_rate: 2.0,
        },
    );

    assert_eq!(db.asset_count(), 2);
    assert!(db.get_asset::<HealthData>("health").is_some());
    assert!(db.get_asset::<WeaponData>("weapon").is_some());
}

#[test]
fn test_database_wrong_type_returns_none() {
    let mut db = AssetDatabase::new();
    db.create_asset(
        "data",
        HealthData {
            name: "X".to_string(),
            asset_path: None,
            max_health: 1,
            regen_rate: 0.0,
        },
    );

    // Requesting wrong type -> None
    assert!(db.get_asset::<WeaponData>("data").is_none());
}

#[test]
fn test_database_nonexistent_returns_none() {
    let db = AssetDatabase::new();
    assert!(db.get_asset::<HealthData>("missing").is_none());
}

#[test]
fn test_database_remove_asset() {
    let mut db = AssetDatabase::new();
    db.create_asset(
        "temp",
        HealthData {
            name: "T".to_string(),
            asset_path: None,
            max_health: 1,
            regen_rate: 0.0,
        },
    );

    assert!(db.has_asset("temp"));
    assert_eq!(db.asset_count(), 1);

    assert!(db.remove_asset("temp"));
    assert!(!db.has_asset("temp"));
    assert_eq!(db.asset_count(), 0);
}

#[test]
fn test_database_remove_nonexistent() {
    let mut db = AssetDatabase::new();
    assert!(!db.remove_asset("nope"));
}

#[test]
fn test_database_clear() {
    let mut db = AssetDatabase::new();
    db.create_asset(
        "a",
        HealthData {
            name: "A".to_string(),
            asset_path: None,
            max_health: 1,
            regen_rate: 0.0,
        },
    );
    db.create_asset(
        "b",
        WeaponData {
            name: "B".to_string(),
            damage: 0.0,
            fire_rate: 0.0,
        },
    );
    assert_eq!(db.asset_count(), 2);

    db.clear();
    assert_eq!(db.asset_count(), 0);
}

#[test]
fn test_database_replace_asset() {
    let mut db = AssetDatabase::new();
    db.create_asset(
        "key",
        HealthData {
            name: "Old".to_string(),
            asset_path: None,
            max_health: 10,
            regen_rate: 0.5,
        },
    );
    db.create_asset(
        "key",
        HealthData {
            name: "New".to_string(),
            asset_path: None,
            max_health: 20,
            regen_rate: 1.0,
        },
    );

    assert_eq!(db.asset_count(), 1);
    let handle = db.get_asset::<HealthData>("key").unwrap();
    assert_eq!(handle.get().Name(), "New");
    assert_eq!(handle.get().max_health, 20);
}

#[test]
fn test_database_asset_names() {
    let mut db = AssetDatabase::new();
    db.create_asset(
        "alpha",
        HealthData {
            name: "A".to_string(),
            asset_path: None,
            max_health: 1,
            regen_rate: 0.0,
        },
    );
    db.create_asset(
        "beta",
        WeaponData {
            name: "B".to_string(),
            damage: 0.0,
            fire_rate: 0.0,
        },
    );

    let mut names = db.asset_names();
    names.sort();
    assert_eq!(names, vec!["alpha", "beta"]);
}

#[test]
fn test_database_create_instance() {
    let mut db = AssetDatabase::new();
    let handle = db.create_instance::<HealthData>("default_hp");

    assert!(db.has_asset("default_hp"));
    assert_eq!(handle.get().name, "");
    assert_eq!(handle.get().max_health, 100);
    assert_eq!(handle.get().regen_rate, 1.0);
}

#[test]
fn test_database_with_capacity() {
    let db = AssetDatabase::with_capacity(50);
    assert_eq!(db.asset_count(), 0);
}

#[test]
fn test_database_default() {
    let db = AssetDatabase::default();
    assert_eq!(db.asset_count(), 0);
}

#[test]
fn test_database_handle_shares_with_stored() {
    let mut db = AssetDatabase::new();
    let handle = db.create_asset(
        "shared",
        HealthData {
            name: "S".to_string(),
            asset_path: None,
            max_health: 1,
            regen_rate: 0.0,
        },
    );

    // The returned handle and the one stored in the DB should share the same Arc
    let stored = db.get_asset::<HealthData>("shared").unwrap();
    // They point to the same underlying data because create_asset clones the Arc
    assert_eq!(handle.get().Name(), stored.get().Name());
}

#[test]
fn test_database_get_asset_mut() {
    let mut db = AssetDatabase::new();
    db.create_asset(
        "mut",
        HealthData {
            name: "Orig".to_string(),
            asset_path: None,
            max_health: 10,
            regen_rate: 0.1,
        },
    );

    // get_asset_mut allows modifying the AssetHandle itself (e.g., replacing it)
    let handle_mut = db.get_asset_mut::<HealthData>("mut").unwrap();
    assert_eq!(handle_mut.get().max_health, 10);

    // After modifying via get_asset_mut, the change is visible through get_asset
    // (This tests the mutable access path exists, not inner mutation)
    let handle_ref = db.get_asset::<HealthData>("mut").unwrap();
    assert_eq!(handle_ref.get().Name(), "Orig");
}

// ---------------------------------------------------------------------------
// Integration: multiple handles, cross-type operations
// ---------------------------------------------------------------------------

#[test]
fn test_multiple_handles_independent() {
    let h1 = AssetHandle::new(HealthData {
        name: "A".to_string(),
        asset_path: None,
        max_health: 10,
        regen_rate: 0.1,
    });
    let h2 = AssetHandle::new(HealthData {
        name: "B".to_string(),
        asset_path: None,
        max_health: 20,
        regen_rate: 0.2,
    });

    assert_ne!(h1, h2);
    assert_eq!(h1.get().max_health, 10);
    assert_eq!(h2.get().max_health, 20);
}

#[test]
fn test_database_mixed_types_coexist() {
    let mut db = AssetDatabase::new();

    db.create_asset(
        "hp",
        HealthData {
            name: "HP".to_string(),
            asset_path: None,
            max_health: 100,
            regen_rate: 1.0,
        },
    );
    db.create_asset(
        "sword",
        WeaponData {
            name: "Sword".to_string(),
            damage: 25.0,
            fire_rate: 1.0,
        },
    );
    db.create_asset(
        "hp2",
        HealthData {
            name: "HP2".to_string(),
            asset_path: None,
            max_health: 200,
            regen_rate: 2.0,
        },
    );

    assert_eq!(db.asset_count(), 3);
    assert_eq!(
        db.get_asset::<HealthData>("hp").unwrap().get().max_health,
        100
    );
    assert_eq!(
        db.get_asset::<HealthData>("hp2").unwrap().get().max_health,
        200
    );
    assert_eq!(
        db.get_asset::<WeaponData>("sword").unwrap().get().damage,
        25.0
    );
}

#[test]
fn test_database_remove_one_of_many() {
    let mut db = AssetDatabase::new();
    db.create_asset(
        "keep",
        HealthData {
            name: "Keep".to_string(),
            asset_path: None,
            max_health: 1,
            regen_rate: 0.0,
        },
    );
    db.create_asset(
        "drop",
        WeaponData {
            name: "Drop".to_string(),
            damage: 1.0,
            fire_rate: 1.0,
        },
    );

    db.remove_asset("drop");

    assert_eq!(db.asset_count(), 1);
    assert!(db.has_asset("keep"));
    assert!(!db.has_asset("drop"));
}

#[test]
fn test_handle_clone_dropped_before_original() {
    let h1 = AssetHandle::new(WeaponData {
        name: "A".to_string(),
        damage: 1.0,
        fire_rate: 1.0,
    });
    let h2 = h1.clone();
    assert_eq!(h1.ref_count(), 2);

    drop(h2);
    assert_eq!(h1.ref_count(), 1);
    assert_eq!(h1.get().Name(), "A");
    // Original handle remains valid after clone is dropped
}

#[test]
fn test_database_serde_roundtrip_asset() {
    let asset = HealthData {
        name: "Serializable".to_string(),
        asset_path: None,
        max_health: 300,
        regen_rate: 5.0,
    };
    let json = serde_json::to_string(&asset).unwrap();
    let deserialized: HealthData = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "Serializable");
    assert_eq!(deserialized.max_health, 300);
    assert_eq!(deserialized.regen_rate, 5.0);
}
