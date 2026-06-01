use crate::scene_serializer::{ComponentData, PropertyValue, Scene, SceneEntity, TransformData};
use anyhow::{Context, Result};
use engine_ecs::entity::Entity;
use engine_ecs::world::World;
use engine_math::{Quat, Vec3};
use engine_scene::transform::Transform;
use std::collections::HashMap;

/// Trait for serializing/deserializing a specific component type.
pub trait ComponentSerializer: Send + Sync {
    /// Unique type name used in scene files (e.g. "Transform", "Camera").
    fn type_name(&self) -> &str;

    /// Extract component data from a world entity.
    /// Returns `None` if the entity doesn't have this component.
    fn extract(&self, world: &World, entity: Entity) -> Option<ComponentData>;

    /// Apply component data to a world entity.
    fn apply(&self, world: &mut World, entity: Entity, data: &ComponentData) -> Result<()>;
}

/// Built-in serializer for `Transform`.
struct TransformSerializer;

impl ComponentSerializer for TransformSerializer {
    fn type_name(&self) -> &str {
        "Transform"
    }

    fn extract(&self, world: &World, entity: Entity) -> Option<ComponentData> {
        let t = world.get::<Transform>(entity)?;
        Some(
            ComponentData::new("Transform".to_string())
                .with_property(
                    "translation",
                    PropertyValue::Vec3([t.translation.x, t.translation.y, t.translation.z]),
                )
                .with_property(
                    "rotation",
                    PropertyValue::Vec4([t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w]),
                )
                .with_property(
                    "scale",
                    PropertyValue::Vec3([t.scale.x, t.scale.y, t.scale.z]),
                ),
        )
    }

    fn apply(&self, world: &mut World, entity: Entity, data: &ComponentData) -> Result<()> {
        let t = transform_from_properties(&data.properties);
        world.add_component(entity, t);
        Ok(())
    }
}

/// Built-in serializer for `Camera` (from engine-render).
struct CameraSerializer;

impl ComponentSerializer for CameraSerializer {
    fn type_name(&self) -> &str {
        "Camera"
    }

    fn extract(&self, world: &World, entity: Entity) -> Option<ComponentData> {
        let cam = world.get::<engine_render::camera::Camera>(entity)?;
        let mut props = HashMap::new();
        props.insert(
            "priority".to_string(),
            PropertyValue::Int(cam.priority as i64),
        );
        props.insert("is_active".to_string(), PropertyValue::Bool(cam.is_active));
        if let Some(ref cc) = cam.clear_color {
            props.insert(
                "clear_color".to_string(),
                PropertyValue::Color([cc.r, cc.g, cc.b, cc.a]),
            );
        }
        // Serialize projection type
        match &cam.projection {
            engine_render::camera::Projection::Perspective { fov_y, near, far } => {
                props.insert(
                    "projection_type".to_string(),
                    PropertyValue::String("perspective".to_string()),
                );
                props.insert("fov_y".to_string(), PropertyValue::Float(*fov_y));
                props.insert("near".to_string(), PropertyValue::Float(*near));
                props.insert("far".to_string(), PropertyValue::Float(*far));
            }
            engine_render::camera::Projection::Orthographic {
                left,
                right,
                bottom,
                top,
                near,
                far,
            } => {
                props.insert(
                    "projection_type".to_string(),
                    PropertyValue::String("orthographic".to_string()),
                );
                props.insert("left".to_string(), PropertyValue::Float(*left));
                props.insert("right".to_string(), PropertyValue::Float(*right));
                props.insert("bottom".to_string(), PropertyValue::Float(*bottom));
                props.insert("top".to_string(), PropertyValue::Float(*top));
                props.insert("near".to_string(), PropertyValue::Float(*near));
                props.insert("far".to_string(), PropertyValue::Float(*far));
            }
        }
        Some(ComponentData {
            type_name: "Camera".to_string(),
            properties: props,
        })
    }

    fn apply(&self, world: &mut World, entity: Entity, data: &ComponentData) -> Result<()> {
        let proj_type = data
            .properties
            .get("projection_type")
            .and_then(|v| match v {
                PropertyValue::String(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("perspective");

        let projection = if proj_type == "orthographic" {
            engine_render::camera::Projection::orthographic(
                get_float(&data.properties, "left", 0.0),
                get_float(&data.properties, "right", 800.0),
                get_float(&data.properties, "bottom", 600.0),
                get_float(&data.properties, "top", 0.0),
                get_float(&data.properties, "near", -1.0),
                get_float(&data.properties, "far", 1.0),
            )
        } else {
            engine_render::camera::Projection::perspective(
                get_float(&data.properties, "fov_y", std::f32::consts::FRAC_PI_3),
                get_float(&data.properties, "near", 0.1),
                get_float(&data.properties, "far", 100.0),
            )
        };

        let mut cam = engine_render::camera::Camera::new(projection);
        cam.priority = get_int(&data.properties, "priority", 0) as i32;
        cam.is_active = get_bool(&data.properties, "is_active", true);
        if let Some(PropertyValue::Color(c)) = data.properties.get("clear_color") {
            cam.clear_color = Some(engine_render::camera::Color::new(c[0], c[1], c[2], c[3]));
        }
        world.add_component(entity, cam);
        Ok(())
    }
}

/// Bridge between ECS World and Scene serialization.
pub struct SceneBridge {
    serializers: HashMap<String, Box<dyn ComponentSerializer>>,
}

impl Default for SceneBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneBridge {
    pub fn new() -> Self {
        let mut bridge = Self {
            serializers: HashMap::new(),
        };
        bridge.register(Box::new(TransformSerializer));
        bridge.register(Box::new(CameraSerializer));
        bridge
    }

    /// Register a custom component serializer.
    pub fn register(&mut self, serializer: Box<dyn ComponentSerializer>) {
        self.serializers
            .insert(serializer.type_name().to_string(), serializer);
    }

    /// Export all entities with registered components from a World to a Scene.
    pub fn export_world(&self, world: &World, scene_name: &str) -> Scene {
        let mut scene = Scene::new(scene_name.to_string());
        let mut entity_id: u64 = 0;

        // Collect all entities that have at least one registered component.
        let mut entity_set: Vec<Entity> = Vec::new();

        // Find entities with Transform
        for idx in world.component_entities::<Transform>() {
            let e = Entity::new(idx, 0);
            if !entity_set.contains(&e) {
                entity_set.push(e);
            }
        }

        // Find entities with Camera
        for idx in world.component_entities::<engine_render::camera::Camera>() {
            let e = Entity::new(idx, 0);
            if !entity_set.contains(&e) {
                entity_set.push(e);
            }
        }

        for entity in entity_set {
            let mut scene_entity =
                SceneEntity::new(entity_id, format!("Entity_{}", entity.index()));

            // Extract transform data for the SceneEntity transform field
            if let Some(t) = world.get::<Transform>(entity) {
                scene_entity.transform = TransformData {
                    translation: [t.translation.x, t.translation.y, t.translation.z],
                    rotation: [t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w],
                    scale: [t.scale.x, t.scale.y, t.scale.z],
                };
            }

            // Extract all registered components
            for serializer in self.serializers.values() {
                if let Some(comp_data) = serializer.extract(world, entity) {
                    // Skip Transform as it's stored in the entity's transform field
                    if comp_data.type_name != "Transform" {
                        scene_entity.add_component(comp_data);
                    }
                }
            }

            scene.add_entity(scene_entity);
            entity_id += 1;
        }

        scene
    }

    /// Import a Scene into a World, creating entities and adding components.
    pub fn import_world(&self, scene: &Scene, world: &mut World) -> Result<Vec<Entity>> {
        let mut entities: Vec<Entity> = Vec::new();

        for scene_entity in &scene.entities {
            let entity = world.spawn();

            // Always add Transform from the entity's transform data
            let transform = transform_from_data(&scene_entity.transform);
            world.add_component(entity, transform);

            // Apply registered components
            for comp_data in &scene_entity.components {
                if let Some(serializer) = self.serializers.get(&comp_data.type_name) {
                    serializer
                        .apply(world, entity, comp_data)
                        .with_context(|| {
                            format!(
                                "Failed to apply component '{}' to entity {}",
                                comp_data.type_name, scene_entity.id
                            )
                        })?;
                }
            }

            entities.push(entity);
        }

        Ok(entities)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn transform_from_data(data: &TransformData) -> Transform {
    Transform {
        translation: Vec3::new(
            data.translation[0],
            data.translation[1],
            data.translation[2],
        ),
        rotation: Quat::from_xyzw(
            data.rotation[0],
            data.rotation[1],
            data.rotation[2],
            data.rotation[3],
        ),
        scale: Vec3::new(data.scale[0], data.scale[1], data.scale[2]),
    }
}

fn transform_from_properties(props: &HashMap<String, PropertyValue>) -> Transform {
    let translation = match props.get("translation") {
        Some(PropertyValue::Vec3(v)) => Vec3::new(v[0], v[1], v[2]),
        _ => Vec3::ZERO,
    };
    let rotation = match props.get("rotation") {
        Some(PropertyValue::Vec4(v)) => Quat::from_xyzw(v[0], v[1], v[2], v[3]),
        _ => Quat::IDENTITY,
    };
    let scale = match props.get("scale") {
        Some(PropertyValue::Vec3(v)) => Vec3::new(v[0], v[1], v[2]),
        _ => Vec3::ONE,
    };
    Transform {
        translation,
        rotation,
        scale,
    }
}

fn get_float(props: &HashMap<String, PropertyValue>, key: &str, default: f32) -> f32 {
    match props.get(key) {
        Some(PropertyValue::Float(v)) => *v,
        _ => default,
    }
}

fn get_int(props: &HashMap<String, PropertyValue>, key: &str, default: i64) -> i64 {
    match props.get(key) {
        Some(PropertyValue::Int(v)) => *v,
        _ => default,
    }
}

fn get_bool(props: &HashMap<String, PropertyValue>, key: &str, default: bool) -> bool {
    match props.get(key) {
        Some(PropertyValue::Bool(v)) => *v,
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_empty_world() {
        let world = World::new();
        let bridge = SceneBridge::new();
        let scene = bridge.export_world(&world, "Empty");
        assert_eq!(scene.name, "Empty");
        assert!(scene.entities.is_empty());
    }

    #[test]
    fn test_export_entity_with_transform() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Transform::from_xyz(1.0, 2.0, 3.0));

        let bridge = SceneBridge::new();
        let scene = bridge.export_world(&world, "Test");
        assert_eq!(scene.entities.len(), 1);
        assert_eq!(scene.entities[0].transform.translation, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_roundtrip_transform() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Transform::from_xyz(5.0, 10.0, 15.0));

        let bridge = SceneBridge::new();
        let scene = bridge.export_world(&world, "Roundtrip");

        // Import into a new world
        let mut world2 = World::new();
        let entities = bridge.import_world(&scene, &mut world2).unwrap();
        assert_eq!(entities.len(), 1);

        let t = world2.get::<Transform>(entities[0]).unwrap();
        assert!((t.translation.x - 5.0).abs() < 1e-6);
        assert!((t.translation.y - 10.0).abs() < 1e-6);
        assert!((t.translation.z - 15.0).abs() < 1e-6);
    }

    #[test]
    fn test_roundtrip_camera() {
        let mut world = World::new();
        let e = world.spawn();
        let mut cam = engine_render::camera::Camera::perspective(1.0472, 0.1, 100.0);
        cam.priority = 5;
        world.add_component(e, cam);
        world.add_component(e, Transform::from_xyz(0.0, 0.0, 0.0));

        let bridge = SceneBridge::new();
        let scene = bridge.export_world(&world, "CamTest");

        // Find the Camera component in the exported scene
        let cam_comp = scene.entities[0]
            .components
            .iter()
            .find(|c| c.type_name == "Camera");
        assert!(cam_comp.is_some());

        // Import into a new world
        let mut world2 = World::new();
        let entities = bridge.import_world(&scene, &mut world2).unwrap();
        let cam2 = world2
            .get::<engine_render::camera::Camera>(entities[0])
            .unwrap();
        assert_eq!(cam2.priority, 5);
    }

    #[test]
    fn test_import_scene_with_multiple_entities() {
        let mut scene = Scene::new("Multi".to_string());
        scene.add_entity(SceneEntity::new(0, "A".to_string()));
        scene.add_entity(SceneEntity::new(1, "B".to_string()));
        scene.add_entity(SceneEntity::new(2, "C".to_string()));

        let bridge = SceneBridge::new();
        let mut world = World::new();
        let entities = bridge.import_world(&scene, &mut world).unwrap();
        assert_eq!(entities.len(), 3);

        // All should have Transform
        for e in &entities {
            assert!(world.get::<Transform>(*e).is_some());
        }
    }

    #[test]
    fn test_roundtrip_via_json() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Transform::from_xyz(1.0, 2.0, 3.0));

        let bridge = SceneBridge::new();
        let scene = bridge.export_world(&world, "JsonTest");

        // Serialize to JSON and back
        let json = serde_json::to_string_pretty(&scene).unwrap();
        let loaded: Scene = serde_json::from_str(&json).unwrap();

        // Import into new world
        let mut world2 = World::new();
        let entities = bridge.import_world(&loaded, &mut world2).unwrap();
        let t = world2.get::<Transform>(entities[0]).unwrap();
        assert!((t.translation.x - 1.0).abs() < 1e-6);
    }
}
