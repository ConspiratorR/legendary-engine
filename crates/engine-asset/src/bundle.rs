//! Asset bundle system for packing, compressing, and loading asset packs.
//!
//! Bundles pack multiple assets into a single file with compression,
//! enabling faster loading and on-demand asset retrieval.
//!
//! ## Format
//!
//! ```text
//! [Magic: b"REBUN\0\0\0"]  (8 bytes)
//! [Version: u32 LE]        (4 bytes)
//! [Entry Count: u32 LE]    (4 bytes)
//! [Entries...]             (variable)
//! [Data...]                (variable)
//! ```
//!
//! Each entry:
//! ```text
//! [Path Len: u16 LE]       (2 bytes)
//! [Path: UTF-8 bytes]      (variable)
//! [Offset: u64 LE]         (8 bytes)
//! [Original Size: u64 LE]  (8 bytes)
//! [Compressed Size: u64 LE](8 bytes)
//! [Flags: u8]              (1 byte) — bit 0: compressed
//! ```

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Magic bytes identifying an asset bundle.
const BUNDLE_MAGIC: &[u8; 8] = b"REBUN\0\0\0";

/// Current bundle format version.
const BUNDLE_VERSION: u32 = 1;

/// Bundle compression flags.
const FLAG_COMPRESSED: u8 = 1 << 0;

/// Error type for bundle operations.
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("Invalid bundle magic bytes")]
    InvalidMagic,
    #[error("Unsupported bundle version: {0}")]
    UnsupportedVersion(u32),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Compression error: {0}")]
    Compression(String),
    #[error("Entry not found: {0}")]
    EntryNotFound(String),
    #[error("Corrupt entry: {0}")]
    CorruptEntry(String),
}

/// Metadata for a single asset entry in a bundle.
#[derive(Debug, Clone)]
pub struct BundleEntry {
    /// Original path of the asset (relative to project root).
    pub path: PathBuf,
    /// Byte offset of the data within the bundle.
    pub offset: u64,
    /// Original (uncompressed) size in bytes.
    pub original_size: u64,
    /// Size in bytes within the bundle (compressed if applicable).
    pub stored_size: u64,
    /// Whether the entry data is compressed.
    pub compressed: bool,
}

/// An asset bundle containing packed, optionally compressed assets.
pub struct Bundle {
    /// Bundle file path.
    path: PathBuf,
    /// Entries indexed by path.
    entries: HashMap<PathBuf, BundleEntry>,
    /// Raw entry list (for serialization).
    entry_list: Vec<BundleEntry>,
}

impl Bundle {
    /// Create an empty bundle builder.
    pub fn builder() -> BundleBuilder {
        BundleBuilder::new()
    }

    /// Open an existing bundle file for reading.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, BundleError> {
        let path = path.into();
        let data = std::fs::read(&path)?;
        let entry_list = parse_bundle_index(&data)?;
        let entries: HashMap<PathBuf, BundleEntry> = entry_list
            .iter()
            .map(|e| (e.path.clone(), e.clone()))
            .collect();

        Ok(Self {
            path,
            entries,
            entry_list,
        })
    }

    /// List all entries in the bundle.
    pub fn entries(&self) -> &[BundleEntry] {
        &self.entry_list
    }

    /// Check if an asset exists in the bundle.
    pub fn contains(&self, path: &Path) -> bool {
        self.entries.contains_key(path)
    }

    /// Get metadata for an asset.
    pub fn get_entry(&self, path: &Path) -> Option<&BundleEntry> {
        self.entries.get(path)
    }

    /// Extract an asset from the bundle.
    ///
    /// Returns the raw (decompressed) bytes.
    pub fn extract(&self, path: &Path) -> Result<Vec<u8>, BundleError> {
        let entry = self
            .entries
            .get(path)
            .ok_or_else(|| BundleError::EntryNotFound(path.display().to_string()))?;

        let mut file = std::fs::File::open(&self.path)?;
        std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(entry.offset))?;

        let mut buf = vec![0u8; entry.stored_size as usize];
        std::io::Read::read_exact(&mut file, &mut buf)?;

        if entry.compressed {
            decompress(&buf).map_err(BundleError::Compression)
        } else {
            Ok(buf)
        }
    }

    /// Number of entries in the bundle.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the bundle is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Builder for creating asset bundles.
///
/// # Example
///
/// ```no_run
/// use engine_asset::bundle::Bundle;
///
/// let bundle_path = Bundle::builder()
///     .add_file("assets/textures/wood.png", "textures/wood.png")
///     .add_file("assets/textures/stone.png", "textures/stone.png")
///     .compress(true)
///     .build("output.rebundle")
///     .unwrap();
/// ```
pub struct BundleBuilder {
    entries: Vec<BuilderEntry>,
    compress: bool,
}

struct BuilderEntry {
    /// Path stored in the bundle (relative).
    bundle_path: PathBuf,
    /// Raw data to pack.
    data: Vec<u8>,
}

impl BundleBuilder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            compress: false,
        }
    }

    /// Enable or disable compression for all entries.
    pub fn compress(mut self, compress: bool) -> Self {
        self.compress = compress;
        self
    }

    /// Add a file from disk to the bundle.
    ///
    /// `source_path` is the file on disk, `bundle_path` is the path stored in the bundle.
    pub fn add_file(self, source_path: impl AsRef<Path>, bundle_path: impl Into<PathBuf>) -> Self {
        let data = std::fs::read(source_path.as_ref())
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", source_path.as_ref().display()));
        self.add_bytes(bundle_path, data)
    }

    /// Add raw bytes to the bundle.
    pub fn add_bytes(mut self, bundle_path: impl Into<PathBuf>, data: Vec<u8>) -> Self {
        self.entries.push(BuilderEntry {
            bundle_path: bundle_path.into(),
            data,
        });
        self
    }

    /// Build the bundle and write it to `output_path`.
    pub fn build(self, output_path: impl AsRef<Path>) -> Result<PathBuf, BundleError> {
        let mut out = std::fs::File::create(output_path.as_ref())?;

        // Write header placeholder
        out.write_all(BUNDLE_MAGIC)?;
        out.write_all(&BUNDLE_VERSION.to_le_bytes())?;
        out.write_all(&(self.entries.len() as u32).to_le_bytes())?;

        // First pass: compress data and calculate offsets
        let mut processed: Vec<(PathBuf, Vec<u8>, bool)> = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            let (data, compressed) = if self.compress && entry.data.len() > 64 {
                let compressed = compress(&entry.data).map_err(BundleError::Compression)?;
                if compressed.len() < entry.data.len() {
                    (compressed, true)
                } else {
                    (entry.data.clone(), false)
                }
            } else {
                (entry.data.clone(), false)
            };
            processed.push((entry.bundle_path.clone(), data, compressed));
        }

        // Calculate data offset (after all index entries)
        let index_size: u64 = processed
            .iter()
            .map(|(path, _data, _compressed)| {
                let path_bytes = path.to_string_lossy().len();
                2 + path_bytes as u64 + 8 + 8 + 8 + 1 // path_len + path + offset + orig_size + stored_size + flags
            })
            .sum();

        let header_size: u64 = 8 + 4 + 4; // magic + version + count
        let mut data_offset = header_size + index_size;

        // Write index entries
        let mut entries = Vec::new();
        for (path, data, compressed) in &processed {
            let path_bytes = path.to_string_lossy();
            let path_bytes = path_bytes.as_bytes();

            out.write_all(&(path_bytes.len() as u16).to_le_bytes())?;
            out.write_all(path_bytes)?;
            out.write_all(&data_offset.to_le_bytes())?;

            let original_size = self
                .entries
                .iter()
                .find(|e| &e.bundle_path == path)
                .map(|e| e.data.len() as u64)
                .unwrap_or(0);

            out.write_all(&original_size.to_le_bytes())?;
            out.write_all(&(data.len() as u64).to_le_bytes())?;

            let flags: u8 = if *compressed { FLAG_COMPRESSED } else { 0 };
            out.write_all(&[flags])?;

            entries.push(BundleEntry {
                path: path.clone(),
                offset: data_offset,
                original_size,
                stored_size: data.len() as u64,
                compressed: *compressed,
            });

            data_offset += data.len() as u64;
        }

        // Write data
        for (_, data, _) in &processed {
            out.write_all(data)?;
        }

        Ok(output_path.as_ref().to_path_buf())
    }
}

impl Default for BundleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse the index from a bundle file's raw bytes.
fn parse_bundle_index(data: &[u8]) -> Result<Vec<BundleEntry>, BundleError> {
    if data.len() < 16 {
        return Err(BundleError::InvalidMagic);
    }

    // Check magic
    if &data[0..8] != BUNDLE_MAGIC {
        return Err(BundleError::InvalidMagic);
    }

    let version = u32::from_le_bytes(
        data[8..12]
            .try_into()
            .map_err(|_| BundleError::CorruptEntry("Invalid version bytes".into()))?,
    );
    if version != BUNDLE_VERSION {
        return Err(BundleError::UnsupportedVersion(version));
    }

    let entry_count = u32::from_le_bytes(
        data[12..16]
            .try_into()
            .map_err(|_| BundleError::CorruptEntry("Invalid entry count bytes".into()))?,
    ) as usize;
    let mut offset = 16usize;
    let mut entries = Vec::with_capacity(entry_count);

    for _ in 0..entry_count {
        if offset + 2 > data.len() {
            return Err(BundleError::CorruptEntry("Truncated index".into()));
        }

        let path_len = u16::from_le_bytes(
            data[offset..offset + 2]
                .try_into()
                .map_err(|_| BundleError::CorruptEntry("Invalid path length bytes".into()))?,
        ) as usize;
        offset += 2;

        if offset + path_len > data.len() {
            return Err(BundleError::CorruptEntry("Truncated path".into()));
        }
        let path = String::from_utf8(data[offset..offset + path_len].to_vec())
            .map_err(|_| BundleError::CorruptEntry("Invalid UTF-8 path".into()))?;
        offset += path_len;

        if offset + 25 > data.len() {
            return Err(BundleError::CorruptEntry("Truncated entry header".into()));
        }

        let entry_offset = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| BundleError::CorruptEntry("Invalid offset bytes".into()))?,
        );
        offset += 8;

        let original_size = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| BundleError::CorruptEntry("Invalid original size bytes".into()))?,
        );
        offset += 8;

        let stored_size = u64::from_le_bytes(
            data[offset..offset + 8]
                .try_into()
                .map_err(|_| BundleError::CorruptEntry("Invalid stored size bytes".into()))?,
        );
        offset += 8;

        let flags = data[offset];
        offset += 1;

        entries.push(BundleEntry {
            path: PathBuf::from(path),
            offset: entry_offset,
            original_size,
            stored_size,
            compressed: flags & FLAG_COMPRESSED != 0,
        });
    }

    Ok(entries)
}

/// Compress data using deflate.
fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    use flate2::Compression;
    use flate2::write::DeflateEncoder;

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(data)
        .map_err(|e| format!("Compress: {e}"))?;
    encoder
        .finish()
        .map_err(|e| format!("Compress finish: {e}"))
}

/// Decompress deflate-compressed data.
fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    use flate2::read::DeflateDecoder;

    let mut decoder = DeflateDecoder::new(data);
    let mut result = Vec::new();
    decoder
        .read_to_end(&mut result)
        .map_err(|e| format!("Decompress: {e}"))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_builder_and_extract() {
        let dir = tempfile::tempdir().unwrap();
        let bundle_path = dir.path().join("test.rebundle");

        Bundle::builder()
            .add_bytes("textures/wood.png", vec![1, 2, 3, 4, 5])
            .add_bytes("models/cube.obj", vec![10, 20, 30])
            .build(&bundle_path)
            .unwrap();

        let bundle = Bundle::open(&bundle_path).unwrap();
        assert_eq!(bundle.len(), 2);
        assert!(bundle.contains(Path::new("textures/wood.png")));
        assert!(bundle.contains(Path::new("models/cube.obj")));

        let data = bundle.extract(Path::new("textures/wood.png")).unwrap();
        assert_eq!(data, vec![1, 2, 3, 4, 5]);

        let data = bundle.extract(Path::new("models/cube.obj")).unwrap();
        assert_eq!(data, vec![10, 20, 30]);
    }

    #[test]
    fn test_bundle_compressed() {
        let dir = tempfile::tempdir().unwrap();
        let bundle_path = dir.path().join("compressed.rebundle");

        // Large enough data to benefit from compression
        let large_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

        Bundle::builder()
            .compress(true)
            .add_bytes("data.bin", large_data.clone())
            .build(&bundle_path)
            .unwrap();

        let bundle = Bundle::open(&bundle_path).unwrap();
        let entry = bundle.get_entry(Path::new("data.bin")).unwrap();
        // Compressed should be smaller than original (for repetitive data)
        assert!(entry.stored_size <= entry.original_size);

        let extracted = bundle.extract(Path::new("data.bin")).unwrap();
        assert_eq!(extracted, large_data);
    }

    #[test]
    fn test_bundle_empty() {
        let dir = tempfile::tempdir().unwrap();
        let bundle_path = dir.path().join("empty.rebundle");

        Bundle::builder().build(&bundle_path).unwrap();

        let bundle = Bundle::open(&bundle_path).unwrap();
        assert_eq!(bundle.len(), 0);
        assert!(bundle.is_empty());
    }

    #[test]
    fn test_bundle_nonexistent_entry() {
        let dir = tempfile::tempdir().unwrap();
        let bundle_path = dir.path().join("test.rebundle");

        Bundle::builder()
            .add_bytes("a.txt", vec![1, 2, 3])
            .build(&bundle_path)
            .unwrap();

        let bundle = Bundle::open(&bundle_path).unwrap();
        assert!(bundle.extract(Path::new("missing.txt")).is_err());
    }

    #[test]
    fn test_bundle_entries_list() {
        let dir = tempfile::tempdir().unwrap();
        let bundle_path = dir.path().join("test.rebundle");

        Bundle::builder()
            .add_bytes("a.txt", vec![1])
            .add_bytes("b.txt", vec![2])
            .add_bytes("c.txt", vec![3])
            .build(&bundle_path)
            .unwrap();

        let bundle = Bundle::open(&bundle_path).unwrap();
        let entries = bundle.entries();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let original = b"Hello, world! This is a test of the compression system. \
                         Repeating data helps compression: AAAAAAAAAAAABBBBBBBBBBBBCCCCCCCCCCCC";
        let compressed = compress(original).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }
}
