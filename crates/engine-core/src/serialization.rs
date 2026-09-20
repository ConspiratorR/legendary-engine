use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::component::Component;
use crate::gameobject::{GameObject, GameObjectHandle};
use crate::scriptable_asset::AssetRef;
use crate::transform::Transform;
use crate::world::World;
use engine_math::{Quat, Vec3};

/// Serialized component data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentData {
    pub type_name: String,
    pub properties: HashMap<String, serde_json::Value>,
}

impl ComponentData {
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            properties: HashMap::new(),
        }
    }

    pub fn with_property(mut self, key: &str, value: serde_json::Value) -> Self {
        self.properties.insert(key.to_string(), value);
        self
    }

    /// Store an [`AssetRef`] GUID under `key` (P3.3 scene asset references).
    pub fn insert_asset_ref(&mut self, key: &str, r: &AssetRef) {
        self.properties
            .insert(key.to_string(), serde_json::json!({ "guid": r.guid }));
    }

    /// Read an [`AssetRef`] from `key`; missing/null becomes `AssetRef::null()`.
    pub fn get_asset_ref(&self, key: &str) -> Option<AssetRef> {
        let v = self.properties.get(key)?;
        if v.is_null() {
            return Some(AssetRef::null());
        }
        if let Some(guid) = v.as_str() {
            return Some(if guid.is_empty() {
                AssetRef::null()
            } else {
                AssetRef::from_guid(guid)
            });
        }
        let guid = v.get("guid")?.as_str()?;
        Some(if guid.is_empty() {
            AssetRef::null()
        } else {
            AssetRef::from_guid(guid)
        })
    }
}

/// Serialized Transform data (built-in, not a Component).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformData {
    pub local_position: Vec3,
    pub local_rotation: Quat,
    pub local_scale: Vec3,
}

impl Default for TransformData {
    fn default() -> Self {
        Self {
            local_position: Vec3::ZERO,
            local_rotation: Quat::IDENTITY,
            local_scale: Vec3::ONE,
        }
    }
}

/// Serialized GameObject data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameObjectData {
    pub name: String,
    pub tag: String,
    pub layer: i32,
    pub active: bool,
    pub transform: TransformData,
    pub components: Vec<ComponentData>,
    pub children: Vec<GameObjectData>,
}

/// Complete scene data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneData {
    pub name: String,
    pub version: u32,
    pub game_objects: Vec<GameObjectData>,
}

/// Trait for formatting components during serialization.
pub trait ComponentFormatter: Send + Sync {
    fn format(&self, component: &dyn Component) -> Option<ComponentData>;
    fn type_name(&self) -> &str;
}

/// Trait for deserializing components during scene loading.
pub trait ComponentDeserializer: Send + Sync {
    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>>;
    fn type_name(&self) -> &str;
}

/// Scene serializer for saving and loading scenes.
pub struct SceneSerializer {
    formatters: Vec<Box<dyn ComponentFormatter>>,
    deserializers: HashMap<String, Box<dyn ComponentDeserializer>>,
}

impl SceneSerializer {
    /// Serializer with built-in Material / SpriteRenderer formatters.
    pub fn new() -> Self {
        let mut s = Self {
            formatters: Vec::new(),
            deserializers: HashMap::new(),
        };
        s.AddFormatter(Box::new(MaterialFormatter));
        s.AddFormatter(Box::new(SpriteRendererFormatter));
        s.AddFormatter(Box::new(RigidbodyFormatter));
        s.AddFormatter(Box::new(AudioSourceFormatter));
        s.AddFormatter(Box::new(LightFormatter));
        s.AddFormatter(Box::new(CameraFormatter));
        s.AddFormatter(Box::new(ScriptBehaviourFormatter));
        s.AddDeserializer(Box::new(MaterialDeserializer));
        s.AddDeserializer(Box::new(SpriteRendererDeserializer));
        s.AddDeserializer(Box::new(RigidbodyDeserializer));
        s.AddDeserializer(Box::new(AudioSourceDeserializer));
        s.AddDeserializer(Box::new(LightDeserializer));
        s.AddDeserializer(Box::new(CameraDeserializer));
        s.AddDeserializer(Box::new(ScriptBehaviourDeserializer));
        s
    }

    pub fn AddFormatter(&mut self, formatter: Box<dyn ComponentFormatter>) {
        self.formatters.push(formatter);
    }

    pub fn AddDeserializer(&mut self, deserializer: Box<dyn ComponentDeserializer>) {
        self.deserializers
            .insert(deserializer.type_name().to_string(), deserializer);
    }

    /// Serialize a World into SceneData.
    pub fn Save(&self, world: &World, name: &str) -> SceneData {
        let roots = world.GetRootGameObjects();
        let game_objects = roots
            .iter()
            .map(|&handle| self.SerializeGameObject(world, handle))
            .collect();

        SceneData {
            name: name.to_string(),
            version: 1,
            game_objects,
        }
    }

    /// Serialize a single GameObject and its children.
    fn SerializeGameObject(&self, world: &World, handle: GameObjectHandle) -> GameObjectData {
        let name = world.GetName(handle).to_string();
        let tag = world.GetTag(handle).to_string();
        let layer = world.GetLayer(handle);
        let active = world.IsActive(handle);

        // Transform: dual-read — feature on prefers ECS Transform; feature off array.
        // Callers that need array caches refreshed should call
        // `World::prepare_scene_io_cache` first.
        let transform_data = if let Some(t) = world.GetTransform(handle) {
            TransformData {
                local_position: t.LocalPosition(),
                local_rotation: t.LocalRotation(),
                local_scale: t.LocalScale(),
            }
        } else {
            TransformData {
                local_position: Vec3::ZERO,
                local_rotation: Quat::IDENTITY,
                local_scale: Vec3::ONE,
            }
        };

        // Serialize components via registered formatters (Material, SpriteRenderer, …)
        let mut components = world
            .get_gameobject(handle)
            .map(|go| {
                let mut out = Vec::new();
                for c in go.Components() {
                    for f in &self.formatters {
                        if let Some(data) = f.format(c.as_ref()) {
                            out.push(data);
                        }
                    }
                }
                out
            })
            .unwrap_or_default();

        // MonoBehaviours: CollectMonoBehaviours uses ECS Instances under feature on.
        for (type_name, enabled, props) in world.CollectMonoBehaviours(handle) {
            let mut data = ComponentData::new(type_name.clone());
            data.properties
                .insert("script_type".into(), serde_json::json!(type_name));
            data.properties
                .insert("enabled".into(), serde_json::json!(enabled));
            if let Some(p) = props {
                data.properties.insert("props".into(), p);
            }
            components.push(data);
        }

        // Serialize children (dual-read GetChildren: ECS authority when feature on)
        let children = world
            .GetChildren(handle)
            .iter()
            .map(|&child_handle| self.SerializeGameObject(world, child_handle))
            .collect();

        GameObjectData {
            name,
            tag,
            layer,
            active,
            transform: transform_data,
            components,
            children,
        }
    }

    /// Deserialize SceneData into a World.
    pub fn Load(&self, scene: &SceneData, world: &mut World) -> Vec<GameObjectHandle> {
        scene
            .game_objects
            .iter()
            .map(|go_data| self.SpawnGameObject(world, go_data))
            .collect()
    }

    /// Spawn a GameObject from serialized data.
    fn SpawnGameObject(&self, world: &mut World, data: &GameObjectData) -> GameObjectHandle {
        let handle = world.CreateGameObject(&data.name);
        world.SetTag(handle, &data.tag);
        world.SetLayer(handle, data.layer);
        world.SetActive(handle, data.active);

        // Set Transform via write-through so dual-read stays coherent
        let _ = world.with_transform_mut(handle, |t| {
            t.SetLocalPosition(data.transform.local_position);
            t.SetLocalRotation(data.transform.local_rotation);
            t.SetLocalScale(data.transform.local_scale);
        });

        // Restore components + MonoBehaviours
        let mut scripts: Vec<(String, bool, Option<serde_json::Value>)> = Vec::new();
        for cd in &data.components {
            // Script components: have script_type (or type_name is a registered MonoBehaviour)
            if let Some(st) = cd
                .properties
                .get("script_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
            {
                let enabled = cd
                    .properties
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let props = cd.properties.get("props").cloned();
                scripts.push((st, enabled, props));
                continue;
            }
            if let Some(d) = self.deserializers.get(cd.type_name.as_str())
                && let Some(c) = d.deserialize(cd)
            {
                world.AddComponentBoxed(handle, c);
            }
        }
        if !scripts.is_empty() {
            world.RestoreMonoBehaviours(handle, &scripts);
        }

        // Spawn children and attach them
        for child_data in &data.children {
            let child_handle = self.SpawnGameObject(world, child_data);
            world.SetParent(child_handle, Some(handle));
        }

        // Load boundary: materialize ECS mirrors; feature-on authority switches to ECS.
        world.seed_ecs_from_array(handle);
        // Feature-on: prefer rebuilding holders from ECS metadata when present.
        #[cfg(feature = "unity-world-primary")]
        {
            let _ = world.restore_monobehaviours_from_ecs(handle);
        }

        handle
    }

    /// Save with array caches refreshed from ECS authority when the feature is on.
    pub fn SavePrepared(&self, world: &mut World, name: &str) -> SceneData {
        world.prepare_scene_io_cache();
        self.Save(world, name)
    }

    /// Save a scene (snake_case alias for Save).
    pub fn save(&self, world: &World, name: &str) -> SceneData {
        self.Save(world, name)
    }

    /// Load a scene (snake_case alias for Load).
    pub fn load(&self, scene: &SceneData, world: &mut World) -> Vec<GameObjectHandle> {
        self.Load(scene, world)
    }
}

impl Default for SceneSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Save a scene to JSON string (does not refresh caches; prefer
/// [`SaveSceneJsonPrepared`] when `unity-world-primary` may be on).
pub fn SaveSceneJson(world: &World, name: &str) -> Result<String, serde_json::Error> {
    let serializer = SceneSerializer::new();
    let scene = serializer.Save(world, name);
    serde_json::to_string_pretty(&scene)
}

/// Save a scene to JSON after refreshing pose/hierarchy caches from ECS
/// authority when `unity-world-primary` is enabled (phase 13).
pub fn SaveSceneJsonPrepared(world: &mut World, name: &str) -> Result<String, serde_json::Error> {
    let serializer = SceneSerializer::new();
    let scene = serializer.SavePrepared(world, name);
    serde_json::to_string_pretty(&scene)
}

/// Built-in formatter for [`crate::components::Material`].
struct MaterialFormatter;

impl ComponentFormatter for MaterialFormatter {
    fn type_name(&self) -> &str {
        "Material"
    }

    fn format(&self, component: &dyn Component) -> Option<ComponentData> {
        let m = component
            .as_any()
            .downcast_ref::<crate::components::Material>()?;
        let mut data = ComponentData::new("Material");
        data.properties
            .insert("base_color".into(), serde_json::json!(m.base_color));
        data.properties
            .insert("metallic".into(), serde_json::json!(m.metallic));
        data.properties
            .insert("smoothness".into(), serde_json::json!(m.smoothness));
        if !m.normal_map.is_empty() {
            data.properties
                .insert("normal_map".into(), serde_json::json!(m.normal_map));
        }
        Some(data)
    }
}

struct MaterialDeserializer;

impl ComponentDeserializer for MaterialDeserializer {
    fn type_name(&self) -> &str {
        "Material"
    }

    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>> {
        let mut m = crate::components::Material::default();
        if let Some(c) = data.properties.get("base_color")
            && let Ok(arr) = serde_json::from_value::<[f32; 4]>(c.clone())
        {
            m.base_color = arr;
        }
        if let Some(v) = data.properties.get("metallic").and_then(|v| v.as_f64()) {
            m.metallic = v as f32;
        }
        if let Some(v) = data.properties.get("smoothness").and_then(|v| v.as_f64()) {
            m.smoothness = v as f32;
        }
        if let Some(v) = data.properties.get("normal_map").and_then(|v| v.as_str()) {
            m.normal_map = v.to_string();
        }
        Some(Box::new(m))
    }
}

/// Built-in formatter for [`crate::components::SpriteRenderer`].
struct SpriteRendererFormatter;

impl ComponentFormatter for SpriteRendererFormatter {
    fn type_name(&self) -> &str {
        "SpriteRenderer"
    }

    fn format(&self, component: &dyn Component) -> Option<ComponentData> {
        let sr = component
            .as_any()
            .downcast_ref::<crate::components::SpriteRenderer>()?;
        let mut data = ComponentData::new("SpriteRenderer");
        data.properties
            .insert("sprite".into(), serde_json::json!(sr.sprite));
        data.properties
            .insert("color".into(), serde_json::json!(sr.color));
        data.properties
            .insert("flip_x".into(), serde_json::json!(sr.flip_x));
        data.properties
            .insert("flip_y".into(), serde_json::json!(sr.flip_y));
        data.properties
            .insert("sorting_order".into(), serde_json::json!(sr.sorting_order));
        Some(data)
    }
}

struct SpriteRendererDeserializer;

impl ComponentDeserializer for SpriteRendererDeserializer {
    fn type_name(&self) -> &str {
        "SpriteRenderer"
    }

    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>> {
        let mut sr = crate::components::SpriteRenderer::default();
        if let Some(v) = data.properties.get("sprite").and_then(|v| v.as_str()) {
            sr.sprite = v.to_string();
        }
        if let Some(c) = data.properties.get("color")
            && let Ok(arr) = serde_json::from_value::<[f32; 4]>(c.clone())
        {
            sr.color = arr;
        }
        if let Some(v) = data.properties.get("flip_x").and_then(|v| v.as_bool()) {
            sr.flip_x = v;
        }
        if let Some(v) = data.properties.get("flip_y").and_then(|v| v.as_bool()) {
            sr.flip_y = v;
        }
        if let Some(v) = data
            .properties
            .get("sorting_order")
            .and_then(|v| v.as_i64())
        {
            sr.sorting_order = v as i32;
        }
        Some(Box::new(sr))
    }
}

/// Built-in formatter for [`crate::components::Rigidbody`].
struct RigidbodyFormatter;

impl ComponentFormatter for RigidbodyFormatter {
    fn type_name(&self) -> &str {
        "Rigidbody"
    }

    fn format(&self, component: &dyn Component) -> Option<ComponentData> {
        let rb = component
            .as_any()
            .downcast_ref::<crate::components::Rigidbody>()?;
        let mut data = ComponentData::new("Rigidbody");
        data.properties
            .insert("mass".into(), serde_json::json!(rb.mass));
        data.properties
            .insert("drag".into(), serde_json::json!(rb.drag));
        data.properties
            .insert("angular_drag".into(), serde_json::json!(rb.angular_drag));
        data.properties
            .insert("use_gravity".into(), serde_json::json!(rb.use_gravity));
        data.properties
            .insert("is_kinematic".into(), serde_json::json!(rb.is_kinematic));
        data.properties.insert(
            "velocity".into(),
            serde_json::json!([rb.velocity.x, rb.velocity.y, rb.velocity.z]),
        );
        data.properties.insert(
            "angular_velocity".into(),
            serde_json::json!([
                rb.angular_velocity.x,
                rb.angular_velocity.y,
                rb.angular_velocity.z
            ]),
        );
        Some(data)
    }
}

struct RigidbodyDeserializer;

impl ComponentDeserializer for RigidbodyDeserializer {
    fn type_name(&self) -> &str {
        "Rigidbody"
    }

    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>> {
        let mut rb = crate::components::Rigidbody::default();
        if let Some(v) = data.properties.get("mass").and_then(|v| v.as_f64()) {
            rb.mass = v as f32;
        }
        if let Some(v) = data.properties.get("drag").and_then(|v| v.as_f64()) {
            rb.drag = v as f32;
        }
        if let Some(v) = data.properties.get("angular_drag").and_then(|v| v.as_f64()) {
            rb.angular_drag = v as f32;
        }
        if let Some(v) = data.properties.get("use_gravity").and_then(|v| v.as_bool()) {
            rb.use_gravity = v;
        }
        if let Some(v) = data
            .properties
            .get("is_kinematic")
            .and_then(|v| v.as_bool())
        {
            rb.is_kinematic = v;
        }
        if let Some(v) = data.properties.get("velocity").and_then(|v| v.as_array())
            && v.len() == 3
        {
            rb.velocity = Vec3::new(
                v[0].as_f64()? as f32,
                v[1].as_f64()? as f32,
                v[2].as_f64()? as f32,
            );
        }
        Some(Box::new(rb))
    }
}

/// Built-in formatter for [`crate::components::AudioSource`].
struct AudioSourceFormatter;

impl ComponentFormatter for AudioSourceFormatter {
    fn type_name(&self) -> &str {
        "AudioSource"
    }

    fn format(&self, component: &dyn Component) -> Option<ComponentData> {
        let a = component
            .as_any()
            .downcast_ref::<crate::components::AudioSource>()?;
        let mut data = ComponentData::new("AudioSource");
        data.properties
            .insert("clip".into(), serde_json::json!(a.clip));
        data.properties
            .insert("volume".into(), serde_json::json!(a.volume));
        data.properties
            .insert("pitch".into(), serde_json::json!(a.pitch));
        data.properties
            .insert("loop_playing".into(), serde_json::json!(a.loop_playing));
        data.properties
            .insert("play_on_awake".into(), serde_json::json!(a.play_on_awake));
        data.properties
            .insert("spatial_blend".into(), serde_json::json!(a.spatial_blend));
        Some(data)
    }
}

struct AudioSourceDeserializer;

impl ComponentDeserializer for AudioSourceDeserializer {
    fn type_name(&self) -> &str {
        "AudioSource"
    }

    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>> {
        let mut a = crate::components::AudioSource::default();
        if let Some(v) = data.properties.get("clip").and_then(|v| v.as_str()) {
            a.clip = v.to_string();
        }
        if let Some(v) = data.properties.get("volume").and_then(|v| v.as_f64()) {
            a.volume = v as f32;
        }
        if let Some(v) = data.properties.get("pitch").and_then(|v| v.as_f64()) {
            a.pitch = v as f32;
        }
        if let Some(v) = data
            .properties
            .get("loop_playing")
            .and_then(|v| v.as_bool())
        {
            a.loop_playing = v;
        }
        if let Some(v) = data
            .properties
            .get("play_on_awake")
            .and_then(|v| v.as_bool())
        {
            a.play_on_awake = v;
        }
        if let Some(v) = data
            .properties
            .get("spatial_blend")
            .and_then(|v| v.as_f64())
        {
            a.spatial_blend = v as f32;
        }
        Some(Box::new(a))
    }
}

/// Built-in formatter for [`crate::components::Light`].
struct LightFormatter;

impl ComponentFormatter for LightFormatter {
    fn type_name(&self) -> &str {
        "Light"
    }

    fn format(&self, component: &dyn Component) -> Option<ComponentData> {
        let l = component
            .as_any()
            .downcast_ref::<crate::components::Light>()?;
        use crate::components::LightType;
        let type_str = match l.light_type {
            LightType::Directional => "Directional",
            LightType::Point => "Point",
            LightType::Spot => "Spot",
        };
        let mut data = ComponentData::new("Light");
        data.properties
            .insert("light_type".into(), serde_json::json!(type_str));
        data.properties
            .insert("color".into(), serde_json::json!(l.color));
        data.properties
            .insert("intensity".into(), serde_json::json!(l.intensity));
        data.properties
            .insert("range".into(), serde_json::json!(l.range));
        data.properties
            .insert("shadows".into(), serde_json::json!(l.shadows));
        Some(data)
    }
}

struct LightDeserializer;

impl ComponentDeserializer for LightDeserializer {
    fn type_name(&self) -> &str {
        "Light"
    }

    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>> {
        use crate::components::LightType;
        let mut l = crate::components::Light::default();
        if let Some(v) = data.properties.get("light_type").and_then(|v| v.as_str()) {
            l.light_type = match v {
                "Directional" => LightType::Directional,
                "Spot" => LightType::Spot,
                _ => LightType::Point,
            };
        }
        if let Some(c) = data.properties.get("color")
            && let Ok(arr) = serde_json::from_value::<[f32; 3]>(c.clone())
        {
            l.color = arr;
        }
        if let Some(v) = data.properties.get("intensity").and_then(|v| v.as_f64()) {
            l.intensity = v as f32;
        }
        if let Some(v) = data.properties.get("range").and_then(|v| v.as_f64()) {
            l.range = v as f32;
        }
        if let Some(v) = data.properties.get("shadows").and_then(|v| v.as_bool()) {
            l.shadows = v;
        }
        Some(Box::new(l))
    }
}

/// Built-in formatter for [`crate::components::Camera`].
struct CameraFormatter;

impl ComponentFormatter for CameraFormatter {
    fn type_name(&self) -> &str {
        "Camera"
    }

    fn format(&self, component: &dyn Component) -> Option<ComponentData> {
        let c = component
            .as_any()
            .downcast_ref::<crate::components::Camera>()?;
        let mut data = ComponentData::new("Camera");
        data.properties
            .insert("field_of_view".into(), serde_json::json!(c.field_of_view));
        data.properties
            .insert("near_clip".into(), serde_json::json!(c.near_clip));
        data.properties
            .insert("far_clip".into(), serde_json::json!(c.far_clip));
        data.properties
            .insert("orthographic".into(), serde_json::json!(c.orthographic));
        data.properties.insert(
            "background_color".into(),
            serde_json::json!(c.background_color),
        );
        data.properties
            .insert("depth".into(), serde_json::json!(c.depth));
        Some(data)
    }
}

struct CameraDeserializer;

impl ComponentDeserializer for CameraDeserializer {
    fn type_name(&self) -> &str {
        "Camera"
    }

    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>> {
        let mut c = crate::components::Camera::default();
        if let Some(v) = data
            .properties
            .get("field_of_view")
            .and_then(|v| v.as_f64())
        {
            c.field_of_view = v as f32;
        }
        if let Some(v) = data.properties.get("near_clip").and_then(|v| v.as_f64()) {
            c.near_clip = v as f32;
        }
        if let Some(v) = data.properties.get("far_clip").and_then(|v| v.as_f64()) {
            c.far_clip = v as f32;
        }
        if let Some(v) = data
            .properties
            .get("orthographic")
            .and_then(|v| v.as_bool())
        {
            c.orthographic = v;
        }
        if let Some(bg) = data.properties.get("background_color")
            && let Ok(arr) = serde_json::from_value::<[f32; 4]>(bg.clone())
        {
            c.background_color = arr;
        }
        if let Some(v) = data.properties.get("depth").and_then(|v| v.as_f64()) {
            c.depth = v as f32;
        }
        Some(Box::new(c))
    }
}

/// Built-in formatter for [`crate::components::ScriptBehaviour`].
struct ScriptBehaviourFormatter;

impl ComponentFormatter for ScriptBehaviourFormatter {
    fn type_name(&self) -> &str {
        "ScriptBehaviour"
    }

    fn format(&self, component: &dyn Component) -> Option<ComponentData> {
        let s = component
            .as_any()
            .downcast_ref::<crate::components::ScriptBehaviour>()?;
        let mut data = ComponentData::new("ScriptBehaviour");
        data.properties
            .insert("script_path".into(), serde_json::json!(s.script_path));
        data.properties
            .insert("enabled".into(), serde_json::json!(s.enabled));
        data.properties
            .insert("properties".into(), serde_json::json!(s.properties));
        Some(data)
    }
}

struct ScriptBehaviourDeserializer;

impl ComponentDeserializer for ScriptBehaviourDeserializer {
    fn type_name(&self) -> &str {
        "ScriptBehaviour"
    }

    fn deserialize(&self, data: &ComponentData) -> Option<Box<dyn Component>> {
        let mut s = crate::components::ScriptBehaviour::default();
        if let Some(v) = data.properties.get("script_path").and_then(|v| v.as_str()) {
            s.script_path = v.to_string();
        }
        if let Some(v) = data.properties.get("enabled").and_then(|v| v.as_bool()) {
            s.enabled = v;
        }
        if let Some(obj) = data
            .properties
            .get("properties")
            .and_then(|v| v.as_object())
        {
            for (k, val) in obj {
                if let Some(sv) = val.as_str() {
                    s.properties.insert(k.clone(), sv.to_string());
                }
            }
        }
        Some(Box::new(s))
    }
}

/// Load a scene from JSON string.
pub fn LoadSceneJson(
    json: &str,
    world: &mut World,
) -> Result<Vec<GameObjectHandle>, serde_json::Error> {
    let scene: SceneData = serde_json::from_str(json)?;
    let serializer = SceneSerializer::new();
    Ok(serializer.Load(&scene, world))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;

    #[test]
    fn test_scene_serializer_new() {
        let s = SceneSerializer::new();
        // Material, SpriteRenderer, Rigidbody, AudioSource, Light, Camera, ScriptBehaviour
        assert_eq!(s.formatters.len(), 7);
        assert_eq!(s.deserializers.len(), 7);
    }

    #[test]
    fn test_material_sprite_roundtrip() {
        use crate::components::{Material, SpriteRenderer};

        let mut world = World::new();
        let go = world.CreateGameObject("Prop");
        world.AddComponent(
            go,
            Material {
                base_color: [1.0, 0.5, 0.25, 1.0],
                metallic: 0.3,
                smoothness: 0.7,
                ..Default::default()
            },
        );
        world.AddComponent(
            go,
            SpriteRenderer {
                sprite: "hero.png".into(),
                color: [0.9, 0.1, 0.1, 1.0],
                flip_x: true,
                flip_y: false,
                sorting_order: 3,
            },
        );

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "Props");
        assert_eq!(scene.game_objects[0].components.len(), 2);

        let mut world2 = World::new();
        s.Load(&scene, &mut world2);
        let go2 = world2.Find("Prop").unwrap();
        let m = world2.GetComponent::<Material>(go2).unwrap();
        assert_eq!(m.base_color, [1.0, 0.5, 0.25, 1.0]);
        assert!((m.metallic - 0.3).abs() < 1e-5);
        let sr = world2.GetComponent::<SpriteRenderer>(go2).unwrap();
        assert_eq!(sr.sprite, "hero.png");
        assert!(sr.flip_x);
        assert_eq!(sr.sorting_order, 3);
    }

    #[test]
    fn test_component_data_asset_ref_property() {
        let mut data = ComponentData::new("EnemyBrain");
        let r = AssetRef::from_guid("deadbeefcafe");
        data.insert_asset_ref("data", &r);
        let back = data.get_asset_ref("data").unwrap();
        assert_eq!(back.guid(), "deadbeefcafe");

        let mut null_data = ComponentData::new("X");
        null_data.insert_asset_ref("data", &AssetRef::null());
        assert!(null_data.get_asset_ref("data").unwrap().is_null());
    }

    #[test]
    fn test_monobehaviour_scene_roundtrip_via_registry() {
        use crate::behaviour::BehaviourState;
        use crate::monobehaviour::{MonoBehaviour, register_mono_behaviour};
        use crate::{Behaviour, Component};

        #[derive(Debug, Default)]
        struct SceneMarker {
            hits: u32,
            state: BehaviourState,
        }

        impl Component for SceneMarker {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        impl Behaviour for SceneMarker {
            fn Enabled(&self) -> bool {
                self.state.enabled()
            }
            fn SetEnabled(&mut self, enabled: bool) {
                self.state.set_enabled(enabled);
            }
            fn IsActiveAndEnabled(&self) -> bool {
                self.state.enabled()
            }
            fn set_gameobject(&mut self, handle: crate::GameObjectHandle) {
                self.state.set_gameobject(handle);
            }
            fn gameobject_handle(&self) -> Option<crate::GameObjectHandle> {
                self.state.gameobject()
            }
        }

        impl MonoBehaviour for SceneMarker {
            fn SerializeProps(&self) -> Option<serde_json::Value> {
                Some(serde_json::json!({ "hits": self.hits }))
            }
            fn DeserializeProps(&mut self, props: &serde_json::Value) {
                self.hits = props.get("hits").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            }
        }

        register_mono_behaviour::<SceneMarker>();

        let mut world = World::new();
        let go = world.CreateGameObject("Scripted");
        {
            let mut m = SceneMarker::default();
            m.hits = 7;
            world.AddMonoBehaviour(go, m);
        }

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "Scripts");
        let json = serde_json::to_string_pretty(&scene).unwrap();
        assert!(json.contains("SceneMarker"));
        assert!(json.contains("\"hits\": 7"));

        let mut world2 = World::new();
        s.Load(&scene, &mut world2);
        let go2 = world2.Find("Scripted").unwrap();
        assert_eq!(world2.MonoBehaviourCount(go2), 1);
        let collected = world2.CollectMonoBehaviours(go2);
        assert_eq!(collected.len(), 1);
        let props = collected[0].2.clone().expect("props");
        assert_eq!(props.get("hits").unwrap().as_u64(), Some(7));
    }

    #[test]
    fn test_save_empty_world() {
        let world = World::new();
        let s = SceneSerializer::new();
        let scene = s.Save(&world, "EmptyScene");

        assert_eq!(scene.name, "EmptyScene");
        assert_eq!(scene.version, 1);
        assert!(scene.game_objects.is_empty());
    }

    #[test]
    fn test_save_single_root() {
        let mut world = World::new();
        world.CreateGameObject("Player");

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "TestScene");

        assert_eq!(scene.game_objects.len(), 1);
        assert_eq!(scene.game_objects[0].name, "Player");
        assert_eq!(scene.game_objects[0].tag, "Untagged");
        assert_eq!(scene.game_objects[0].layer, 0);
        assert!(scene.game_objects[0].active);
    }

    #[test]
    fn test_save_hierarchy() {
        let mut world = World::new();
        let root = world.CreateGameObject("Root");
        let child1 = world.CreateGameObject("Child1");
        let child2 = world.CreateGameObject("Child2");
        world.SetParent(child1, Some(root));
        world.SetParent(child2, Some(root));

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "Hierarchy");

        assert_eq!(scene.game_objects.len(), 1);
        assert_eq!(scene.game_objects[0].children.len(), 2);
        assert_eq!(scene.game_objects[0].children[0].name, "Child1");
        assert_eq!(scene.game_objects[0].children[1].name, "Child2");
    }

    #[test]
    fn test_save_only_serializes_roots() {
        let mut world = World::new();
        let root = world.CreateGameObject("Root");
        let child = world.CreateGameObject("Child");
        world.SetParent(child, Some(root));

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "Test");

        assert_eq!(scene.game_objects.len(), 1);
        assert_eq!(scene.game_objects[0].children.len(), 1);
    }

    #[test]
    fn test_load_empty_scene() {
        let mut world = World::new();
        let scene = SceneData {
            name: "Empty".into(),
            version: 1,
            game_objects: vec![],
        };

        let s = SceneSerializer::new();
        let handles = s.Load(&scene, &mut world);

        assert!(handles.is_empty());
    }

    #[test]
    fn test_load_single_object() {
        let mut world = World::new();
        let scene = SceneData {
            name: "Test".into(),
            version: 1,
            game_objects: vec![GameObjectData {
                name: "LoadedObj".into(),
                tag: "Enemy".into(),
                layer: 5,
                active: false,
                transform: TransformData {
                    local_position: Vec3::new(1.0, 2.0, 3.0),
                    local_rotation: Quat::IDENTITY,
                    local_scale: Vec3::ONE,
                },
                components: vec![],
                children: vec![],
            }],
        };

        let s = SceneSerializer::new();
        let handles = s.Load(&scene, &mut world);

        assert_eq!(handles.len(), 1);
        assert_eq!(world.GetName(handles[0]), "LoadedObj");
        assert_eq!(world.GetTag(handles[0]), "Enemy");
        assert_eq!(world.GetLayer(handles[0]), 5);
        assert!(!world.IsActive(handles[0]));

        // Verify Transform
        let t = world.GetTransform(handles[0]).unwrap();
        assert_eq!(t.LocalPosition(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_load_hierarchy() {
        let mut world = World::new();
        let scene = SceneData {
            name: "Hierarchy".into(),
            version: 1,
            game_objects: vec![GameObjectData {
                name: "Root".into(),
                tag: "Untagged".into(),
                layer: 0,
                active: true,
                transform: TransformData {
                    local_position: Vec3::ZERO,
                    local_rotation: Quat::IDENTITY,
                    local_scale: Vec3::ONE,
                },
                components: vec![],
                children: vec![
                    GameObjectData {
                        name: "Child1".into(),
                        tag: "".into(),
                        layer: 0,
                        active: true,
                        transform: TransformData {
                            local_position: Vec3::ZERO,
                            local_rotation: Quat::IDENTITY,
                            local_scale: Vec3::ONE,
                        },
                        components: vec![],
                        children: vec![],
                    },
                    GameObjectData {
                        name: "Child2".into(),
                        tag: "".into(),
                        layer: 0,
                        active: true,
                        transform: TransformData {
                            local_position: Vec3::ZERO,
                            local_rotation: Quat::IDENTITY,
                            local_scale: Vec3::ONE,
                        },
                        components: vec![],
                        children: vec![],
                    },
                ],
            }],
        };

        let s = SceneSerializer::new();
        let handles = s.Load(&scene, &mut world);

        assert_eq!(handles.len(), 1);
        assert_eq!(world.GetName(handles[0]), "Root");
        assert_eq!(world.GetChildren(handles[0]).len(), 2);

        let children = world.GetChildren(handles[0]);
        assert_eq!(world.GetName(children[0]), "Child1");
        assert_eq!(world.GetName(children[1]), "Child2");
    }

    #[test]
    fn test_roundtrip_json() {
        let mut world = World::new();
        let root = world.CreateGameObject("Player");
        let child = world.CreateGameObject("Gun");
        world.SetParent(child, Some(root));

        let json = SaveSceneJson(&world, "GameScene").unwrap();
        assert!(json.contains("Player"));
        assert!(json.contains("Gun"));

        let mut world2 = World::new();
        let handles = LoadSceneJson(&json, &mut world2).unwrap();

        assert_eq!(handles.len(), 1);
        assert_eq!(world2.GetName(handles[0]), "Player");
        assert_eq!(world2.GetChildren(handles[0]).len(), 1);

        let children = world2.GetChildren(handles[0]);
        assert_eq!(world2.GetName(children[0]), "Gun");
    }

    #[test]
    fn test_save_preserves_transform() {
        let mut world = World::new();
        let handle = world.CreateGameObject("Player");
        let _ = world.with_transform_mut(handle, |t| {
            t.SetLocalPosition(Vec3::new(1.0, 2.0, 3.0));
            t.SetLocalRotation(Quat::from_rotation_y(1.57));
            t.SetLocalScale(Vec3::new(2.0, 2.0, 2.0));
        });

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "Test");

        let data = &scene.game_objects[0];
        assert_eq!(data.transform.local_position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(data.transform.local_scale, Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_default_impl() {
        let s = SceneSerializer::default();
        assert_eq!(s.formatters.len(), 7);
        assert_eq!(s.deserializers.len(), 7);
    }

    #[test]
    fn test_physics_audio_light_camera_roundtrip() {
        use crate::components::{AudioSource, Camera, Light, LightType, Rigidbody};

        let mut world = World::new();
        let go = world.CreateGameObject("Prop");
        world.AddComponent(
            go,
            Rigidbody {
                mass: 2.5,
                use_gravity: true,
                is_kinematic: false,
                ..Default::default()
            },
        );
        world.AddComponent(
            go,
            AudioSource {
                clip: "sfx/hit.wav".into(),
                volume: 0.7,
                play_on_awake: true,
                ..Default::default()
            },
        );
        world.AddComponent(
            go,
            Light {
                light_type: LightType::Directional,
                color: [1.0, 0.9, 0.8],
                intensity: 2.0,
                range: 50.0,
                shadows: true,
                ..Default::default()
            },
        );
        world.AddComponent(go, Camera::default());

        let s = SceneSerializer::new();
        let scene = s.Save(&world, "Props");
        assert_eq!(scene.game_objects[0].components.len(), 4);

        let mut world2 = World::new();
        s.Load(&scene, &mut world2);
        let go2 = world2.Find("Prop").unwrap();
        let rb = world2.GetComponent::<Rigidbody>(go2).unwrap();
        assert!((rb.mass - 2.5).abs() < 1e-5);
        let au = world2.GetComponent::<AudioSource>(go2).unwrap();
        assert_eq!(au.clip, "sfx/hit.wav");
        let li = world2.GetComponent::<Light>(go2).unwrap();
        assert_eq!(li.light_type, LightType::Directional);
        assert!((li.intensity - 2.0).abs() < 1e-5);
        assert!(world2.GetComponent::<Camera>(go2).is_some());
    }
}
