use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Scene {
    pub name: String,
    pub entities: Vec<SceneEntity>,
    pub settings: SceneSettings,
}

#[derive(Debug, Clone)]
pub struct SceneEntity {
    pub id: u64,
    pub name: String,
    pub components: Vec<ComponentData>,
    pub children: Vec<u64>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct ComponentData {
    pub type_name: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SceneSettings {
    pub ambient_color: [f32; 4],
    pub fog_enabled: bool,
    pub fog_color: [f32; 4],
    pub fog_near: f32,
    pub fog_far: f32,
}

impl Default for SceneSettings {
    fn default() -> Self {
        Self {
            ambient_color: [0.2, 0.2, 0.2, 1.0],
            fog_enabled: false,
            fog_color: [0.5, 0.5, 0.5, 1.0],
            fog_near: 10.0,
            fog_far: 100.0,
        }
    }
}

impl Scene {
    pub fn new(name: String) -> Self {
        Self {
            name,
            entities: Vec::new(),
            settings: SceneSettings::default(),
        }
    }

    pub fn add_entity(&mut self, entity: SceneEntity) {
        self.entities.push(entity);
    }

    pub fn remove_entity(&mut self, id: u64) -> Option<SceneEntity> {
        self.entities.retain(|e| e.id != id);
        self.entities
            .iter()
            .position(|e| e.id == id)
            .map(|pos| self.entities.remove(pos))
    }

    pub fn get_entity(&self, id: u64) -> Option<&SceneEntity> {
        self.entities.iter().find(|e| e.id == id)
    }

    pub fn get_entity_mut(&mut self, id: u64) -> Option<&mut SceneEntity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    pub fn to_string_pretty(&self) -> String {
        let mut output = format!("Scene: {}\n", self.name);
        output += "Settings:\n";
        output += &format!("  Ambient Color: {:?}\n", self.settings.ambient_color);
        output += &format!("  Fog Enabled: {}\n", self.settings.fog_enabled);
        output += &format!("\nEntities ({}):\n", self.entities.len());

        for entity in &self.entities {
            output += &format!(
                "  Entity {}: {} (active: {})\n",
                entity.id, entity.name, entity.active
            );
            if !entity.components.is_empty() {
                output += "    Components:\n";
                for component in &entity.components {
                    output += &format!("      - {}\n", component.type_name);
                }
            }
            if !entity.children.is_empty() {
                output += &format!("    Children: {:?}\n", entity.children);
            }
        }

        output
    }
}

impl SceneEntity {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            components: Vec::new(),
            children: Vec::new(),
            active: true,
        }
    }

    pub fn add_component(&mut self, component: ComponentData) {
        self.components.push(component);
    }

    pub fn remove_component(&mut self, type_name: &str) -> Option<ComponentData> {
        self.components
            .iter()
            .position(|c| c.type_name == type_name)
            .map(|pos| self.components.remove(pos))
    }
}

impl ComponentData {
    pub fn new(type_name: String) -> Self {
        Self {
            type_name,
            properties: HashMap::new(),
        }
    }

    pub fn with_property(mut self, key: &str, value: String) -> Self {
        self.properties.insert(key.to_string(), value);
        self
    }
}

pub struct SceneManager {
    current_scene: Option<Scene>,
    scene_path: Option<String>,
    is_modified: bool,
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            current_scene: None,
            scene_path: None,
            is_modified: false,
        }
    }

    pub fn create_scene(&mut self, name: String) {
        self.current_scene = Some(Scene::new(name));
        self.scene_path = None;
        self.is_modified = false;
    }

    pub fn current_scene(&self) -> Option<&Scene> {
        self.current_scene.as_ref()
    }

    pub fn current_scene_mut(&mut self) -> Option<&mut Scene> {
        self.is_modified = true;
        self.current_scene.as_mut()
    }

    pub fn scene_path(&self) -> Option<&str> {
        self.scene_path.as_deref()
    }

    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    pub fn mark_modified(&mut self) {
        self.is_modified = true;
    }

    pub fn mark_saved(&mut self) {
        self.is_modified = false;
    }

    pub fn new_entity(&mut self, name: String) -> Option<u64> {
        if let Some(ref mut scene) = self.current_scene {
            let id = scene.entities.len() as u64 + 1;
            scene.add_entity(SceneEntity::new(id, name));
            self.is_modified = true;
            Some(id)
        } else {
            None
        }
    }

    pub fn print_scene(&self) {
        if let Some(ref scene) = self.current_scene {
            println!("{}", scene.to_string_pretty());
        }
    }
}

impl Default for SceneManager {
    fn default() -> Self {
        Self::new()
    }
}
