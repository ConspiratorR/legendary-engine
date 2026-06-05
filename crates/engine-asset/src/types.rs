//! Resource type definitions.
//!
//! Concrete asset types used throughout the engine: [`Texture`],
//! [`AudioClip`], [`Mesh`], [`Material`], [`Script`], and [`SceneAsset`].

use crate::asset::Asset;
use std::path::PathBuf;

/// Image texture asset.
///
/// Stores decoded pixel data in RGBA8 format along with dimensions.
#[derive(Debug, Clone)]
pub struct Texture {
    /// Unique identifier (typically the source path).
    pub id: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Raw pixel data in RGBA8 format.
    pub data: Vec<u8>,
    /// Number of color channels (always 4 for RGBA8).
    pub channels: u8,
    /// Original file path.
    pub asset_path: PathBuf,
}

impl Asset for Texture {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// Audio asset.
///
/// Stores raw audio data with format information and estimated duration.
#[derive(Debug, Clone)]
pub struct AudioClip {
    /// Unique identifier (typically the source path).
    pub id: String,
    /// Raw audio data (encoded or PCM depending on format).
    pub data: Vec<u8>,
    /// Audio format codec.
    pub format: AudioFormat,
    /// Estimated duration in seconds.
    pub duration: f32,
}

/// Audio codec format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// WAV (uncompressed PCM).
    Wav,
    /// Ogg Vorbis.
    Ogg,
    /// MP3.
    Mp3,
    /// FLAC (lossless).
    Flac,
}

impl Asset for AudioClip {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// 3D mesh asset.
///
/// Contains vertex and index data for rendering a triangle mesh.
#[derive(Debug, Clone)]
pub struct Mesh {
    /// Unique identifier.
    pub id: String,
    /// Vertex data (position, normal, UV).
    pub vertices: Vec<Vertex>,
    /// Triangle indices.
    pub indices: Vec<u32>,
}

/// A single vertex with position, normal, and texture coordinates.
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    /// XYZ position.
    pub position: [f32; 3],
    /// XYZ normal vector.
    pub normal: [f32; 3],
    /// UV texture coordinates.
    pub tex_coord: [f32; 2],
}

impl Asset for Mesh {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// Material asset.
///
/// Defines surface appearance properties for PBR rendering.
#[derive(Debug, Clone)]
pub struct Material {
    /// Unique identifier.
    pub id: String,
    /// RGBA base color (linear space).
    pub base_color: [f32; 4],
    /// Metallic factor (0.0 = dielectric, 1.0 = metal).
    pub metallic: f32,
    /// Roughness factor (0.0 = smooth, 1.0 = rough).
    pub roughness: f32,
    /// Optional path to a base color texture.
    pub base_color_texture: Option<String>,
    /// Optional path to a normal map texture.
    pub normal_texture: Option<String>,
}

impl Asset for Material {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// Script asset.
///
/// Stores source code and language information for executable scripts.
#[derive(Debug, Clone)]
pub struct Script {
    /// Unique identifier.
    pub id: String,
    /// Source code text.
    pub source: String,
    /// Programming language of the script.
    pub language: ScriptLanguage,
}

/// Supported script languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptLanguage {
    /// Lua scripting language.
    Lua,
    /// Rust source.
    Rust,
    /// Python scripting language.
    Python,
}

impl Asset for Script {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// Scene asset.
///
/// Stores serialized scene data (entity hierarchy, transforms, components).
#[derive(Debug, Clone)]
pub struct SceneAsset {
    /// Unique identifier.
    pub id: String,
    /// Serialized scene data.
    pub data: Vec<u8>,
}

impl Asset for SceneAsset {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// Resource metadata.
///
/// Describes a discovered file or directory in the asset tree.
#[derive(Debug, Clone)]
pub struct ResourceMeta {
    /// Full file system path.
    pub path: PathBuf,
    /// File or directory name.
    pub name: String,
    /// Detected resource type.
    pub file_type: ResourceType,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Last modification time (Unix timestamp).
    pub last_modified: u64,
}

/// Type of resource detected by file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// Unknown or unrecognized file type.
    Unknown,
    /// Directory.
    Directory,
    /// Image texture.
    Texture,
    /// Audio file.
    Audio,
    /// 3D model mesh.
    Mesh,
    /// Material definition.
    Material,
    /// Script file.
    Script,
    /// Scene file.
    Scene,
}

impl ResourceType {
    /// Determine the resource type from a file extension (case-insensitive).
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

    /// Get an emoji icon representing this resource type.
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
