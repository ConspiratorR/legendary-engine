//! Asset preview and thumbnail generation.
//!
//! Provides thumbnail generation for textures and a preview registry
//! for caching generated previews. Integrates with the editor's
//! resource browser for asset visualization.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Maximum thumbnail dimension (width or height) in pixels.
const DEFAULT_THUMB_SIZE: u32 = 128;

/// Error type for preview generation.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("Unsupported asset type for preview: {0}")]
    UnsupportedType(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image processing error: {0}")]
    Image(String),
    #[error("Asset not found: {0}")]
    NotFound(String),
}

/// A generated thumbnail image in RGBA8 format.
#[derive(Debug, Clone)]
pub struct Thumbnail {
    /// Pixel data in RGBA8 format.
    pub pixels: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Thumbnail {
    /// Create a new thumbnail.
    pub fn new(pixels: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            pixels,
            width,
            height,
        }
    }

    /// Create a placeholder thumbnail (checkerboard pattern).
    pub fn placeholder(size: u32) -> Self {
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                let checker = ((x / 8) + (y / 8)) % 2 == 0;
                let c = if checker { 200u8 } else { 100u8 };
                pixels.extend_from_slice(&[c, c, c, 255]);
            }
        }
        Self {
            pixels,
            width: size,
            height: size,
        }
    }

    /// Create an error/missing thumbnail (red cross pattern).
    pub fn error(size: u32) -> Self {
        let mut pixels = vec![60u8; (size * size * 4) as usize];
        let thickness = (size / 16).max(2);

        for y in 0..size {
            for x in 0..size {
                let idx = ((y * size + x) * 4) as usize;
                let on_diag1 = (x as i32 - y as i32).unsigned_abs() < thickness;
                let on_diag2 = (x as i32 + y as i32 - size as i32 + 1).unsigned_abs() < thickness;

                if on_diag1 || on_diag2 {
                    pixels[idx] = 200;
                    pixels[idx + 1] = 50;
                    pixels[idx + 2] = 50;
                    pixels[idx + 3] = 255;
                }
            }
        }

        Self {
            pixels,
            width: size,
            height: size,
        }
    }

    /// Raw pixel data size in bytes.
    pub fn data_size(&self) -> usize {
        self.pixels.len()
    }
}

/// Generates thumbnails for different asset types.
pub struct ThumbnailGenerator {
    /// Target thumbnail size in pixels.
    thumb_size: u32,
}

impl ThumbnailGenerator {
    /// Create a new thumbnail generator with default settings.
    pub fn new() -> Self {
        Self {
            thumb_size: DEFAULT_THUMB_SIZE,
        }
    }

    /// Create with a custom thumbnail size.
    pub fn with_size(thumb_size: u32) -> Self {
        Self { thumb_size }
    }

    /// Generate a thumbnail for the asset at the given path.
    ///
    /// Dispatches to the appropriate generator based on file extension.
    pub fn generate(&self, path: &Path) -> Result<Thumbnail, PreviewError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "bmp" | "tga" | "hdr" => self.generate_texture_thumbnail(path),
            _ => Err(PreviewError::UnsupportedType(ext)),
        }
    }

    /// Generate a thumbnail from raw RGBA8 pixel data.
    pub fn generate_from_rgba(&self, pixels: &[u8], src_width: u32, src_height: u32) -> Thumbnail {
        if src_width <= self.thumb_size && src_height <= self.thumb_size {
            return Thumbnail::new(pixels.to_vec(), src_width, src_height);
        }

        let (dst_w, dst_h) = compute_thumb_dims(src_width, src_height, self.thumb_size);
        let resized = resize_nearest_neighbor(pixels, src_width, src_height, dst_w, dst_h);
        Thumbnail::new(resized, dst_w, dst_h)
    }

    /// Generate a thumbnail for a texture file.
    fn generate_texture_thumbnail(&self, path: &Path) -> Result<Thumbnail, PreviewError> {
        let img =
            image::open(path).map_err(|e| PreviewError::Image(format!("Load failed: {e}")))?;

        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();

        Ok(self.generate_from_rgba(&rgba, w, h))
    }

    /// Target thumbnail size.
    pub fn thumb_size(&self) -> u32 {
        self.thumb_size
    }
}

impl Default for ThumbnailGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for caching generated thumbnails.
///
/// Thumbnails are keyed by asset path and invalidated when the
/// source asset changes.
pub struct PreviewRegistry {
    /// Cached thumbnails indexed by asset path.
    thumbnails: HashMap<PathBuf, Thumbnail>,
    /// Content hash of the source asset at thumbnail generation time.
    hashes: HashMap<PathBuf, u64>,
    /// Maximum number of cached thumbnails.
    max_entries: usize,
    /// Total memory used by cached thumbnails (bytes).
    memory_used: usize,
}

impl PreviewRegistry {
    /// Create a new preview registry.
    pub fn new() -> Self {
        Self {
            thumbnails: HashMap::new(),
            hashes: HashMap::new(),
            max_entries: 1024,
            memory_used: 0,
        }
    }

    /// Create with a maximum number of cached entries.
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            max_entries,
            ..Self::new()
        }
    }

    /// Store a thumbnail for an asset path.
    pub fn insert(&mut self, path: PathBuf, thumbnail: Thumbnail, hash: u64) {
        // Remove old entry if exists
        if let Some(old) = self.thumbnails.get(&path) {
            self.memory_used = self.memory_used.saturating_sub(old.data_size());
        }

        self.memory_used += thumbnail.data_size();
        self.thumbnails.insert(path.clone(), thumbnail);
        self.hashes.insert(path, hash);

        // Evict if over limit
        self.evict_if_needed();
    }

    /// Get a cached thumbnail for an asset path.
    pub fn get(&self, path: &Path) -> Option<&Thumbnail> {
        self.thumbnails.get(path)
    }

    /// Check if a cached thumbnail is still valid (hash matches).
    pub fn is_valid(&self, path: &Path, current_hash: u64) -> bool {
        self.hashes
            .get(path)
            .is_some_and(|&cached_hash| cached_hash == current_hash)
    }

    /// Remove a cached thumbnail.
    pub fn remove(&mut self, path: &Path) -> Option<Thumbnail> {
        self.hashes.remove(path);
        let thumb = self.thumbnails.remove(path);
        if let Some(ref t) = thumb {
            self.memory_used = self.memory_used.saturating_sub(t.data_size());
        }
        thumb
    }

    /// Clear all cached thumbnails.
    pub fn clear(&mut self) {
        self.thumbnails.clear();
        self.hashes.clear();
        self.memory_used = 0;
    }

    /// Number of cached thumbnails.
    pub fn len(&self) -> usize {
        self.thumbnails.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.thumbnails.is_empty()
    }

    /// Total memory used by cached thumbnails (bytes).
    pub fn memory_used(&self) -> usize {
        self.memory_used
    }

    /// Evict oldest entries if over the max entry count.
    fn evict_if_needed(&mut self) {
        while self.thumbnails.len() > self.max_entries {
            // Remove an arbitrary entry (HashMap order is not guaranteed,
            // but this provides a simple eviction strategy)
            if let Some(key) = self.thumbnails.keys().next().cloned() {
                self.remove(&key);
            } else {
                break;
            }
        }
    }
}

impl Default for PreviewRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute thumbnail dimensions maintaining aspect ratio.
fn compute_thumb_dims(src_w: u32, src_h: u32, max_size: u32) -> (u32, u32) {
    if src_w == 0 || src_h == 0 {
        return (max_size, max_size);
    }

    let ratio = src_w as f32 / src_h as f32;
    if ratio >= 1.0 {
        let w = max_size;
        let h = (max_size as f32 / ratio).max(1.0) as u32;
        (w, h)
    } else {
        let h = max_size;
        let w = (max_size as f32 * ratio).max(1.0) as u32;
        (w, h)
    }
}

/// Simple nearest-neighbor resize for RGBA8 data.
fn resize_nearest_neighbor(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_w * dst_h * 4) as usize];

    for dy in 0..dst_h {
        for dx in 0..dst_w {
            let sx = (dx as f32 * src_w as f32 / dst_w as f32) as u32;
            let sy = (dy as f32 * src_h as f32 / dst_h as f32) as u32;
            let sx = sx.min(src_w - 1);
            let sy = sy.min(src_h - 1);

            let src_idx = ((sy * src_w + sx) * 4) as usize;
            let dst_idx = ((dy * dst_w + dx) * 4) as usize;

            dst[dst_idx..dst_idx + 4].copy_from_slice(&src[src_idx..src_idx + 4]);
        }
    }

    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_placeholder() {
        let thumb = Thumbnail::placeholder(64);
        assert_eq!(thumb.width, 64);
        assert_eq!(thumb.height, 64);
        assert_eq!(thumb.pixels.len(), 64 * 64 * 4);
    }

    #[test]
    fn test_thumbnail_error() {
        let thumb = Thumbnail::error(32);
        assert_eq!(thumb.width, 32);
        assert_eq!(thumb.height, 32);
        // Check that some red pixels exist
        let has_red = thumb.pixels.chunks(4).any(|px| px[0] > 150 && px[1] < 100);
        assert!(has_red);
    }

    #[test]
    fn test_thumbnail_generator_from_rgba() {
        let generator = ThumbnailGenerator::with_size(64);
        let pixels = vec![255u8; 256 * 256 * 4]; // Large white image
        let thumb = generator.generate_from_rgba(&pixels, 256, 256);
        assert!(thumb.width <= 64);
        assert!(thumb.height <= 64);
    }

    #[test]
    fn test_thumbnail_generator_small_image() {
        let generator = ThumbnailGenerator::with_size(128);
        let pixels = vec![128u8; 32 * 32 * 4]; // Small image
        let thumb = generator.generate_from_rgba(&pixels, 32, 32);
        assert_eq!(thumb.width, 32);
        assert_eq!(thumb.height, 32);
    }

    #[test]
    fn test_compute_thumb_dims_landscape() {
        let (w, h) = compute_thumb_dims(200, 100, 64);
        assert_eq!(w, 64);
        assert_eq!(h, 32);
    }

    #[test]
    fn test_compute_thumb_dims_portrait() {
        let (w, h) = compute_thumb_dims(100, 200, 64);
        assert_eq!(w, 32);
        assert_eq!(h, 64);
    }

    #[test]
    fn test_compute_thumb_dims_square() {
        let (w, h) = compute_thumb_dims(200, 200, 64);
        assert_eq!(w, 64);
        assert_eq!(h, 64);
    }

    #[test]
    fn test_preview_registry_insert_get() {
        let mut reg = PreviewRegistry::new();
        let thumb = Thumbnail::placeholder(32);
        let path = PathBuf::from("test.png");

        reg.insert(path.clone(), thumb, 12345);
        assert!(reg.get(&path).is_some());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn test_preview_registry_hash_validity() {
        let mut reg = PreviewRegistry::new();
        let thumb = Thumbnail::placeholder(32);
        let path = PathBuf::from("test.png");

        reg.insert(path.clone(), thumb, 12345);
        assert!(reg.is_valid(&path, 12345));
        assert!(!reg.is_valid(&path, 99999));
    }

    #[test]
    fn test_preview_registry_remove() {
        let mut reg = PreviewRegistry::new();
        let thumb = Thumbnail::placeholder(32);
        let path = PathBuf::from("test.png");

        reg.insert(path.clone(), thumb, 12345);
        reg.remove(&path);
        assert!(reg.is_empty());
        assert_eq!(reg.memory_used(), 0);
    }

    #[test]
    fn test_preview_registry_eviction() {
        let mut reg = PreviewRegistry::with_max_entries(2);

        reg.insert(PathBuf::from("a.png"), Thumbnail::placeholder(16), 1);
        reg.insert(PathBuf::from("b.png"), Thumbnail::placeholder(16), 2);
        reg.insert(PathBuf::from("c.png"), Thumbnail::placeholder(16), 3);

        // Should have evicted one entry
        assert!(reg.len() <= 2);
    }

    #[test]
    fn test_preview_registry_memory_tracking() {
        let mut reg = PreviewRegistry::new();
        let thumb = Thumbnail::placeholder(32);
        let expected_size = thumb.data_size();

        reg.insert(PathBuf::from("test.png"), thumb, 12345);
        assert_eq!(reg.memory_used(), expected_size);
    }

    #[test]
    fn test_resize_nearest_neighbor() {
        let src = vec![255u8; 4 * 4 * 4]; // 4x4 white
        let dst = resize_nearest_neighbor(&src, 4, 4, 2, 2);
        assert_eq!(dst.len(), 2 * 2 * 4);
        assert_eq!(dst, vec![255u8; 2 * 2 * 4]);
    }
}
