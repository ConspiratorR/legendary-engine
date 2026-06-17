//! Unified asset import pipeline.
//!
//! Provides [`AssetImporter`] trait for format-specific importers,
//! [`ImportContext`] for passing parameters and collecting dependencies,
//! and [`ImportPipeline`] for registering and running importers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::asset::Asset;

/// Error type for asset import operations.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("No importer registered for extension '{0}'")]
    NoImporter(String),
    #[error("Import failed for '{path}': {message}")]
    Failed { path: String, message: String },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Format error: {0}")]
    Format(String),
    #[error("Missing dependency: {0}")]
    MissingDependency(String),
}

/// Result of an asset import.
pub struct ImportResult {
    /// The imported asset data (type-erased).
    pub asset: Box<dyn std::any::Any + Send + Sync>,
    /// Assets this import depends on (relative paths).
    pub dependencies: Vec<PathBuf>,
    /// Content hash for cache invalidation.
    pub content_hash: u64,
}

impl std::fmt::Debug for ImportResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportResult")
            .field("dependencies", &self.dependencies)
            .field("content_hash", &self.content_hash)
            .finish()
    }
}

/// Context passed to importers during import.
///
/// Collects dependency declarations and provides import parameters.
pub struct ImportContext {
    /// Import parameters (key-value).
    params: HashMap<String, ImportParam>,
    /// Collected dependency paths.
    dependencies: Vec<PathBuf>,
    /// The source path being imported.
    source_path: PathBuf,
}

/// Supported import parameter types.
#[derive(Debug, Clone)]
pub enum ImportParam {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl ImportContext {
    /// Create a new import context for the given source path.
    pub fn new(source_path: impl Into<PathBuf>) -> Self {
        Self {
            params: HashMap::new(),
            dependencies: Vec::new(),
            source_path: source_path.into(),
        }
    }

    /// Create a context with initial parameters.
    pub fn with_params(
        source_path: impl Into<PathBuf>,
        params: HashMap<String, ImportParam>,
    ) -> Self {
        Self {
            params,
            dependencies: Vec::new(),
            source_path: source_path.into(),
        }
    }

    /// The source file path being imported.
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    /// Declare a dependency on another asset file.
    pub fn add_dependency(&mut self, path: impl Into<PathBuf>) {
        self.dependencies.push(path.into());
    }

    /// Get all collected dependencies.
    pub fn dependencies(&self) -> &[PathBuf] {
        &self.dependencies
    }

    /// Get a boolean parameter.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.params.get(key)? {
            ImportParam::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Get an integer parameter.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.params.get(key)? {
            ImportParam::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// Get a float parameter.
    pub fn get_float(&self, key: &str) -> Option<f64> {
        match self.params.get(key)? {
            ImportParam::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// Get a string parameter.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.params.get(key)? {
            ImportParam::String(v) => Some(v.as_str()),
            _ => None,
        }
    }

    /// Take the collected dependencies, leaving the context empty.
    pub fn take_dependencies(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.dependencies)
    }
}

/// Trait for format-specific asset importers.
///
/// Each importer handles one or more file extensions and converts
/// raw bytes into a concrete asset type.
pub trait AssetImporter: Send + Sync {
    /// The asset type this importer produces.
    type Asset: Asset + Send + Sync;

    /// File extensions this importer handles (without the dot).
    fn extensions(&self) -> &[&str];

    /// Import an asset from raw bytes.
    ///
    /// The `ctx` is used to declare dependencies and read parameters.
    fn import(&self, data: &[u8], ctx: &mut ImportContext) -> Result<Self::Asset, ImportError>;

    /// Whether this importer supports hot-reload.
    ///
    /// Default is `true`. Override to `false` for assets that cannot
    /// be safely replaced at runtime (e.g., GPU pipeline objects).
    fn supports_hot_reload(&self) -> bool {
        true
    }
}

/// Type-erased wrapper for [`AssetImporter`].
trait DynImporter: Send + Sync {
    fn extensions(&self) -> &[&str];
    fn import_boxed(
        &self,
        data: &[u8],
        ctx: &mut ImportContext,
    ) -> Result<ImportResult, ImportError>;
    fn supports_hot_reload(&self) -> bool;
}

struct ImporterWrapper<I: AssetImporter> {
    inner: I,
}

impl<I: AssetImporter> DynImporter for ImporterWrapper<I> {
    fn extensions(&self) -> &[&str] {
        self.inner.extensions()
    }

    fn import_boxed(
        &self,
        data: &[u8],
        ctx: &mut ImportContext,
    ) -> Result<ImportResult, ImportError> {
        let asset = self.inner.import(data, ctx)?;
        let dependencies = ctx.take_dependencies();
        let content_hash = hash_bytes(data);

        Ok(ImportResult {
            asset: Box::new(asset),
            dependencies,
            content_hash,
        })
    }

    fn supports_hot_reload(&self) -> bool {
        self.inner.supports_hot_reload()
    }
}

/// Central import pipeline that manages importers and runs imports.
pub struct ImportPipeline {
    /// Registered importers indexed by file extension.
    importers: HashMap<String, Arc<dyn DynImporter>>,
}

impl ImportPipeline {
    /// Create an empty pipeline.
    pub fn new() -> Self {
        Self {
            importers: HashMap::new(),
        }
    }

    /// Register an importer. Each extension it handles gets a mapping.
    pub fn register<I: AssetImporter + 'static>(&mut self, importer: I) {
        let wrapper = Arc::new(ImporterWrapper { inner: importer });
        for ext in wrapper.extensions() {
            self.importers.insert(ext.to_string(), wrapper.clone());
        }
    }

    /// Import an asset from a file path.
    ///
    /// Reads the file, determines the importer by extension, and runs it.
    pub fn import_file(&self, path: &Path) -> Result<ImportResult, ImportError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| ImportError::Failed {
                path: path.display().to_string(),
                message: "File has no extension".to_string(),
            })?;

        let importer = self
            .importers
            .get(ext)
            .ok_or_else(|| ImportError::NoImporter(ext.to_string()))?;

        let data = std::fs::read(path)?;
        let mut ctx = ImportContext::new(path);
        importer.import_boxed(&data, &mut ctx)
    }

    /// Import from raw bytes with an explicit extension hint.
    pub fn import_bytes(
        &self,
        data: &[u8],
        extension: &str,
        source_path: &Path,
    ) -> Result<ImportResult, ImportError> {
        let importer = self
            .importers
            .get(extension)
            .ok_or_else(|| ImportError::NoImporter(extension.to_string()))?;

        let mut ctx = ImportContext::new(source_path);
        importer.import_boxed(data, &mut ctx)
    }

    /// Check if an importer is registered for the given extension.
    pub fn has_importer(&self, extension: &str) -> bool {
        self.importers.contains_key(extension)
    }

    /// List all registered extensions.
    pub fn registered_extensions(&self) -> Vec<&str> {
        self.importers.keys().map(|s| s.as_str()).collect()
    }

    /// Check if the importer for the given extension supports hot-reload.
    pub fn supports_hot_reload(&self, extension: &str) -> bool {
        self.importers
            .get(extension)
            .map(|i| i.supports_hot_reload())
            .unwrap_or(false)
    }
}

impl Default for ImportPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute a fast hash of byte content for cache invalidation.
pub fn hash_bytes(data: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

/// Compute hash of a file's contents.
pub fn hash_file(path: &Path) -> Result<u64, ImportError> {
    let data = std::fs::read(path)?;
    Ok(hash_bytes(&data))
}

/// Metadata about an imported asset for cache tracking and .meta files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetMeta {
    /// Stable unique identifier for this asset (GUID).
    pub guid: String,
    /// The file path of the source asset.
    pub source_path: PathBuf,
    /// Content hash at time of import.
    pub content_hash: u64,
    /// File modification time (seconds since epoch).
    pub modified_at_secs: u64,
    /// Paths of dependency assets.
    pub dependencies: Vec<PathBuf>,
    /// Whether the importer supports hot-reload.
    pub hot_reloadable: bool,
    /// Import settings for this asset type.
    pub import_settings: ImportSettings,
}

/// Per-asset import settings.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub enum ImportSettings {
    /// Texture import settings.
    Texture {
        max_size: u32,
        generate_mipmaps: bool,
        compression: String,
    },
    /// Mesh import settings.
    Mesh { generate_lod: bool, scale: f32 },
    /// Audio import settings.
    Audio { sample_rate: u32, streaming: bool },
    /// Default settings.
    #[default]
    Default,
}

impl AssetMeta {
    /// Create a new asset meta with a generated GUID.
    pub fn new(source_path: PathBuf) -> Self {
        Self {
            guid: Self::generate_guid(),
            source_path,
            content_hash: 0,
            modified_at_secs: 0,
            dependencies: Vec::new(),
            hot_reloadable: false,
            import_settings: ImportSettings::default(),
        }
    }

    /// Save this meta to a `.meta` file alongside the source asset.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let meta_path = Self::meta_path_for(&self.source_path);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&meta_path, json)
    }

    /// Load a `.meta` file for the given asset path, if it exists.
    pub fn load(source_path: &Path) -> Option<Self> {
        let meta_path = Self::meta_path_for(source_path);
        if !meta_path.exists() {
            return None;
        }
        let json = std::fs::read_to_string(&meta_path).ok()?;
        serde_json::from_str(&json).ok()
    }

    /// Get the `.meta` file path for a given asset path.
    pub fn meta_path_for(asset_path: &Path) -> PathBuf {
        let mut meta_path = asset_path.to_path_buf();
        let extension = meta_path
            .extension()
            .map(|e| format!("{}.meta", e.to_string_lossy()))
            .unwrap_or_else(|| "meta".to_string());
        meta_path.set_extension(extension);
        meta_path
    }

    /// Generate a simple GUID (hex string based on timestamp + counter).
    fn generate_guid() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{:032x}", t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestAsset(String);

    impl Asset for TestAsset {
        type Id = str;
        fn id(&self) -> &Self::Id {
            &self.0
        }
    }

    struct TestImporter;

    impl AssetImporter for TestImporter {
        type Asset = TestAsset;

        fn extensions(&self) -> &[&str] {
            &["test"]
        }

        fn import(&self, data: &[u8], _ctx: &mut ImportContext) -> Result<TestAsset, ImportError> {
            let s =
                String::from_utf8(data.to_vec()).map_err(|e| ImportError::Format(e.to_string()))?;
            Ok(TestAsset(s))
        }
    }

    #[test]
    fn test_pipeline_register_and_import() {
        let mut pipeline = ImportPipeline::new();
        pipeline.register(TestImporter);

        assert!(pipeline.has_importer("test"));
        assert!(!pipeline.has_importer("unknown"));

        let result = pipeline
            .import_bytes(b"hello", "test", Path::new("test.test"))
            .unwrap();
        let asset = result.asset.downcast_ref::<TestAsset>().unwrap();
        assert_eq!(&asset.0, "hello");
    }

    #[test]
    fn test_pipeline_extensions() {
        let mut pipeline = ImportPipeline::new();
        pipeline.register(TestImporter);

        let mut exts = pipeline.registered_extensions();
        exts.sort();
        assert_eq!(exts, vec!["test"]);
    }

    #[test]
    fn test_import_context_params() {
        let ctx = ImportContext::with_params(
            "test.txt",
            HashMap::from([
                ("flag".to_string(), ImportParam::Bool(true)),
                ("count".to_string(), ImportParam::Int(42)),
                ("rate".to_string(), ImportParam::Float(0.5)),
                ("name".to_string(), ImportParam::String("foo".to_string())),
            ]),
        );

        assert_eq!(ctx.get_bool("flag"), Some(true));
        assert_eq!(ctx.get_int("count"), Some(42));
        assert_eq!(ctx.get_float("rate"), Some(0.5));
        assert_eq!(ctx.get_string("name"), Some("foo"));
        assert_eq!(ctx.get_bool("missing"), None);
    }

    #[test]
    fn test_import_context_dependencies() {
        let mut ctx = ImportContext::new("main.gltf");
        ctx.add_dependency("texture.png");
        ctx.add_dependency("material.mat");

        assert_eq!(ctx.dependencies().len(), 2);
        let deps = ctx.take_dependencies();
        assert_eq!(deps.len(), 2);
        assert!(ctx.dependencies().is_empty());
    }

    #[test]
    fn test_hash_bytes_deterministic() {
        let h1 = hash_bytes(b"test data");
        let h2 = hash_bytes(b"test data");
        assert_eq!(h1, h2);

        let h3 = hash_bytes(b"different data");
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_import_result_content_hash() {
        let mut pipeline = ImportPipeline::new();
        pipeline.register(TestImporter);

        let r1 = pipeline
            .import_bytes(b"same", "test", Path::new("a.test"))
            .unwrap();
        let r2 = pipeline
            .import_bytes(b"same", "test", Path::new("b.test"))
            .unwrap();
        assert_eq!(r1.content_hash, r2.content_hash);
    }

    #[test]
    fn test_no_importer_error() {
        let pipeline = ImportPipeline::new();
        let result = pipeline.import_bytes(b"data", "xyz", Path::new("file.xyz"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ImportError::NoImporter(_)));
    }
}
