//! Resource type definitions.
use crate::asset::Asset;
use std::path::PathBuf;

/// Image texture asset.
#[derive(Debug, Clone)]
pub struct Texture {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub channels: u8,
}

impl Asset for Texture {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// Audio asset.
#[derive(Debug, Clone)]
pub struct AudioClip {
    pub id: String,
    pub data: Vec<u8>,
    pub format: AudioFormat,
    pub duration: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Wav,
    Ogg,
    Mp3,
    Flac,
}

impl Asset for AudioClip {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// 3D mesh asset.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub id: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl Asset for Mesh {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// Material asset.
#[derive(Debug, Clone)]
pub struct Material {
    pub id: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub base_color_texture: Option<String>,
    pub normal_texture: Option<String>,
}

impl Asset for Material {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// Script asset.
#[derive(Debug, Clone)]
pub struct Script {
    pub id: String,
    pub source: String,
    pub language: ScriptLanguage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptLanguage {
    Lua,
    Rust,
    Python,
}

impl Asset for Script {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// Scene asset.
#[derive(Debug, Clone)]
pub struct SceneAsset {
    pub id: String,
    pub data: Vec<u8>,
}

impl Asset for SceneAsset {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// Resource metadata.
#[derive(Debug, Clone)]
pub struct ResourceMeta {
    pub path: PathBuf,
    pub name: String,
    pub file_type: ResourceType,
    pub size: u64,
    pub last_modified: u64,
}

/// Type of resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Unknown,
    Directory,
    Texture,
    Audio,
    Mesh,
    Material,
    Script,
    Scene,
}

impl ResourceType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "bmp" | "tga" | "hdr" => Self::Texture,
            "wav" | "ogg" | "mp3" | "flac" => Self::Audio,
            "gltf" | "glb" | "obj" | "fbx" => Self::Mesh,
            "mat" | "material" => Self::Material,
            "lua" | "rs" | "py" => Self::Script,
            "scene" | "json" => Self::Scene,
            _ => Self::Unknown,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Unknown => "📄",
            Self::Directory => "📁",
            Self::Texture => "🖼️",
            Self::Audio => "🎵",
            Self::Mesh => "🧊",
            Self::Material => "🎨",
            Self::Script => "📝",
            Self::Scene => "🎬",
        }
    }
}
