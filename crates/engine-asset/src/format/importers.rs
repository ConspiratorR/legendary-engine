//! Concrete [`AssetImporter`] implementations for common formats.
//!
//! These importers convert raw file bytes into engine asset types
//! and can be registered with [`ImportPipeline`](crate::pipeline::ImportPipeline).

use crate::pipeline::{AssetImporter, ImportContext, ImportError};
use crate::types::{AudioClip, AudioFormat, Material, Mesh, Script, ScriptLanguage, Texture};

/// Importer for image files (PNG, JPG, BMP, TGA, HDR).
///
/// Produces [`Texture`] assets with RGBA8 pixel data.
pub struct ImageImporter;

impl AssetImporter for ImageImporter {
    type Asset = Texture;

    fn extensions(&self) -> &[&str] {
        &["png", "jpg", "jpeg", "bmp", "tga", "hdr"]
    }

    fn import(&self, data: &[u8], ctx: &mut ImportContext) -> Result<Texture, ImportError> {
        let img = image::load_from_memory(data)
            .map_err(|e| ImportError::Format(format!("Image decode failed: {e}")))?;

        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let pixels = rgba.into_raw();

        Ok(Texture {
            id: ctx.source_path().display().to_string(),
            width,
            height,
            data: pixels,
            channels: 4,
            asset_path: ctx.source_path().to_path_buf(),
        })
    }
}

/// Collection of meshes from a single glTF/GLB file.
///
/// A glTF file can contain multiple meshes — this wrapper holds them all
/// and implements [`Asset`] so it can be stored in the registry.
#[derive(Debug, Clone)]
pub struct MeshCollection {
    pub id: String,
    pub meshes: Vec<Mesh>,
}

impl crate::asset::Asset for MeshCollection {
    type Id = str;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

/// Importer for glTF/GLB 3D model files.
///
/// Produces [`MeshCollection`] containing all meshes from the file.
pub struct GltfImporter;

impl AssetImporter for GltfImporter {
    type Asset = MeshCollection;

    fn extensions(&self) -> &[&str] {
        &["gltf", "glb"]
    }

    fn import(&self, data: &[u8], ctx: &mut ImportContext) -> Result<MeshCollection, ImportError> {
        // Write to a temp file since gltf::import requires a file path
        let tmp_path = std::env::temp_dir().join(format!(
            "rustengine_gltf_{}.glb",
            ctx.source_path()
                .display()
                .to_string()
                .replace(['\\', '/'], "_")
        ));
        std::fs::write(&tmp_path, data).map_err(ImportError::Io)?;

        let result = (|| -> Result<MeshCollection, ImportError> {
            let (document, buffers, _images) = gltf::import(&tmp_path)
                .map_err(|e| ImportError::Format(format!("glTF parse: {e}")))?;

            let mut meshes = Vec::new();

            for mesh in document.meshes() {
                let mesh_name = mesh.name().unwrap_or("unnamed").to_string();

                for primitive in mesh.primitives() {
                    let reader =
                        primitive.reader(|buffer| buffers.get(buffer.index()).map(|d| &**d));

                    let positions = reader
                        .read_positions()
                        .ok_or(ImportError::Format("Missing position accessor".into()))?;

                    let normals: Vec<[f32; 3]> = match reader.read_normals() {
                        Some(n) => n.collect(),
                        None => positions.clone().map(|_| [0.0, 1.0, 0.0]).collect(),
                    };

                    let tex_coords: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
                        Some(tc) => tc.into_f32().collect(),
                        None => positions.clone().map(|_| [0.0, 0.0]).collect(),
                    };

                    let vertices: Vec<crate::types::Vertex> = positions
                        .zip(normals)
                        .zip(tex_coords)
                        .map(|((pos, n), uv)| crate::types::Vertex {
                            position: pos,
                            normal: n,
                            tex_coord: uv,
                        })
                        .collect();

                    let indices: Vec<u32> = match reader.read_indices() {
                        Some(gltf::mesh::util::ReadIndices::U8(iter)) => {
                            iter.map(|i| i as u32).collect()
                        }
                        Some(gltf::mesh::util::ReadIndices::U16(iter)) => {
                            iter.map(|i| i as u32).collect()
                        }
                        Some(gltf::mesh::util::ReadIndices::U32(iter)) => iter.collect(),
                        None => (0..vertices.len() as u32).collect(),
                    };

                    meshes.push(Mesh {
                        id: format!(
                            "{}_{}",
                            ctx.source_path().display(),
                            if mesh_name.is_empty() {
                                meshes.len().to_string()
                            } else {
                                mesh_name.clone()
                            }
                        ),
                        vertices,
                        indices,
                    });
                }
            }

            Ok(MeshCollection {
                id: ctx.source_path().display().to_string(),
                meshes,
            })
        })();

        // Clean up temp file
        let _ = std::fs::remove_file(&tmp_path);

        result
    }
}

/// Importer for audio files (WAV, OGG, MP3, FLAC).
///
/// Produces [`AudioClip`] assets. Duration is estimated from file size
/// for formats where precise duration requires full decode.
pub struct AudioImporter;

impl AssetImporter for AudioImporter {
    type Asset = AudioClip;

    fn extensions(&self) -> &[&str] {
        &["wav", "ogg", "mp3", "flac"]
    }

    fn import(&self, data: &[u8], ctx: &mut ImportContext) -> Result<AudioClip, ImportError> {
        let ext = ctx
            .source_path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let format = match ext.to_lowercase().as_str() {
            "wav" => AudioFormat::Wav,
            "ogg" => AudioFormat::Ogg,
            "mp3" => AudioFormat::Mp3,
            "flac" => AudioFormat::Flac,
            _ => return Err(ImportError::Format(format!("Unknown audio format: {ext}"))),
        };

        // Estimate duration from data size and format
        let duration = estimate_audio_duration(data, format);

        Ok(AudioClip {
            id: ctx.source_path().display().to_string(),
            data: data.to_vec(),
            format,
            duration,
        })
    }
}

/// Importer for material definition files.
///
/// Produces [`Material`] assets from JSON material definitions.
pub struct MaterialImporter;

impl AssetImporter for MaterialImporter {
    type Asset = Material;

    fn extensions(&self) -> &[&str] {
        &["mat"]
    }

    fn import(&self, data: &[u8], ctx: &mut ImportContext) -> Result<Material, ImportError> {
        let text = std::str::from_utf8(data)
            .map_err(|e| ImportError::Format(format!("Invalid UTF-8 in material: {e}")))?;

        // Simple key-value material format
        let mut base_color = [1.0f32; 4];
        let mut metallic = 0.0f32;
        let mut roughness = 0.5f32;
        let mut base_color_texture = None;
        let mut normal_texture = None;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();

            match key {
                "base_color" => {
                    if let Ok(arr) = parse_vec4(value) {
                        base_color = arr;
                    }
                }
                "metallic" => {
                    metallic = value.parse().unwrap_or(0.0);
                }
                "roughness" => {
                    roughness = value.parse().unwrap_or(0.5);
                }
                "base_color_texture" => {
                    base_color_texture = Some(value.to_string());
                    ctx.add_dependency(value);
                }
                "normal_texture" => {
                    normal_texture = Some(value.to_string());
                    ctx.add_dependency(value);
                }
                _ => {}
            }
        }

        Ok(Material {
            id: ctx.source_path().display().to_string(),
            base_color,
            metallic,
            roughness,
            base_color_texture,
            normal_texture,
        })
    }
}

/// Importer for script files (Lua, Rust, Python).
///
/// Produces [`Script`] assets.
pub struct ScriptImporter;

impl AssetImporter for ScriptImporter {
    type Asset = Script;

    fn extensions(&self) -> &[&str] {
        &["lua", "rs", "py"]
    }

    fn import(&self, data: &[u8], ctx: &mut ImportContext) -> Result<Script, ImportError> {
        let ext = ctx
            .source_path()
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let language = match ext.to_lowercase().as_str() {
            "lua" => ScriptLanguage::Lua,
            "rs" => ScriptLanguage::Rust,
            "py" => ScriptLanguage::Python,
            _ => return Err(ImportError::Format(format!("Unknown script lang: {ext}"))),
        };

        let source = String::from_utf8(data.to_vec())
            .map_err(|e| ImportError::Format(format!("Invalid UTF-8 in script: {e}")))?;

        Ok(Script {
            id: ctx.source_path().display().to_string(),
            source,
            language,
        })
    }

    fn supports_hot_reload(&self) -> bool {
        true
    }
}

/// Estimate audio duration from raw data size and format.
fn estimate_audio_duration(data: &[u8], format: AudioFormat) -> f32 {
    match format {
        // WAV: typically 16-bit PCM at 44100 Hz stereo
        AudioFormat::Wav => {
            // Skip WAV header (44 bytes), data is samples * channels * bytes_per_sample
            let data_size = data.len().saturating_sub(44) as f32;
            // Assume 44100 Hz, 16-bit, stereo (4 bytes per sample frame)
            data_size / (44100.0 * 4.0)
        }
        // Compressed formats: rough estimate based on typical bitrates
        AudioFormat::Mp3 => data.len() as f32 / (128_000.0 / 8.0), // 128 kbps
        AudioFormat::Ogg => data.len() as f32 / (112_000.0 / 8.0), // 112 kbps
        AudioFormat::Flac => data.len() as f32 / (800_000.0 / 8.0), // ~800 kbps
    }
}

/// Parse a "r g b a" or "r,g,b,a" string into [f32; 4].
fn parse_vec4(s: &str) -> Result<[f32; 4], ()> {
    let parts: Vec<&str> = if s.contains(',') {
        s.split(',').collect()
    } else {
        s.split_whitespace().collect()
    };
    if parts.len() != 4 {
        return Err(());
    }
    let mut arr = [0.0f32; 4];
    for (i, p) in parts.iter().enumerate() {
        arr[i] = p.trim().parse().map_err(|_| ())?;
    }
    Ok(arr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::Asset;
    use std::path::Path;

    #[test]
    fn test_image_importer_extensions() {
        let imp = ImageImporter;
        assert!(imp.extensions().contains(&"png"));
        assert!(imp.extensions().contains(&"jpg"));
        assert!(imp.extensions().contains(&"hdr"));
    }

    #[test]
    fn test_gltf_importer_extensions() {
        let imp = GltfImporter;
        assert!(imp.extensions().contains(&"gltf"));
        assert!(imp.extensions().contains(&"glb"));
    }

    #[test]
    fn test_audio_importer_extensions() {
        let imp = AudioImporter;
        assert!(imp.extensions().contains(&"wav"));
        assert!(imp.extensions().contains(&"ogg"));
        assert!(imp.extensions().contains(&"mp3"));
        assert!(imp.extensions().contains(&"flac"));
    }

    #[test]
    fn test_script_importer_extensions() {
        let imp = ScriptImporter;
        assert!(imp.extensions().contains(&"lua"));
        assert!(imp.extensions().contains(&"rs"));
        assert!(imp.extensions().contains(&"py"));
    }

    #[test]
    fn test_audio_format_detection() {
        let ctx = ImportContext::new("test.wav");
        assert_eq!(
            ctx.source_path()
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or(""),
            "wav"
        );
    }

    #[test]
    fn test_parse_vec4() {
        assert_eq!(parse_vec4("1.0 0.5 0.3 1.0"), Ok([1.0, 0.5, 0.3, 1.0]));
        assert_eq!(parse_vec4("1.0,0.5,0.3,1.0"), Ok([1.0, 0.5, 0.3, 1.0]));
        assert!(parse_vec4("1.0 0.5").is_err());
    }

    #[test]
    fn test_estimate_audio_duration_wav() {
        // 44-byte header + 44100 * 4 bytes = 1 second of stereo 16-bit audio
        let data = vec![0u8; 44 + 44100 * 4];
        let dur = estimate_audio_duration(&data, AudioFormat::Wav);
        assert!((dur - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_material_importer() {
        let mat_data = b"base_color = 1.0 0.0 0.0 1.0\nmetallic = 0.5\nroughness = 0.8\n";
        let imp = MaterialImporter;
        let mut ctx = ImportContext::new("test.mat");
        let mat = imp.import(mat_data, &mut ctx).unwrap();
        assert_eq!(mat.base_color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(mat.metallic, 0.5);
        assert_eq!(mat.roughness, 0.8);
    }

    #[test]
    fn test_material_with_dependency() {
        let mat_data = b"base_color_texture = textures/wood.png\n";
        let imp = MaterialImporter;
        let mut ctx = ImportContext::new("test.mat");
        let _mat = imp.import(mat_data, &mut ctx).unwrap();
        assert_eq!(ctx.dependencies().len(), 1);
        assert_eq!(
            ctx.dependencies()[0],
            Path::new("textures/wood.png").to_path_buf()
        );
    }

    #[test]
    fn test_script_importer() {
        let script_data = b"print('hello world')";
        let imp = ScriptImporter;
        let mut ctx = ImportContext::new("test.lua");
        let script = imp.import(script_data, &mut ctx).unwrap();
        assert_eq!(script.language, ScriptLanguage::Lua);
        assert_eq!(script.source, "print('hello world')");
    }

    #[test]
    fn test_mesh_collection_asset() {
        let mc = MeshCollection {
            id: "test".to_string(),
            meshes: vec![],
        };
        assert_eq!(mc.id(), "test");
    }

    #[test]
    fn test_image_importer_invalid_data() {
        let imp = ImageImporter;
        let mut ctx = ImportContext::new("test.png");
        let result = imp.import(b"not an image", &mut ctx);
        assert!(result.is_err());
    }
}
