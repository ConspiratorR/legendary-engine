//! Hot reload system for assets and shaders.
//!
//! Watches file system changes and triggers recompilation/reload
//! of modified assets, shaders, and scripts.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::Context;
use engine_asset::types::ResourceType;
use log::{info, warn};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebouncedEvent, Debouncer, new_debouncer};

/// Notification for hot-reloaded assets.
#[derive(Debug, Clone)]
pub struct ReloadNotification {
    pub message: String,
    pub timestamp: Instant,
    pub level: ReloadLevel,
}

/// Severity level of a reload notification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReloadLevel {
    Info,
    Warning,
    Error,
}

/// Hot reload manager with notifications.
pub struct HotReloadManager {
    pub notifications: VecDeque<ReloadNotification>,
    pub max_notifications: usize,
}

impl HotReloadManager {
    pub fn new() -> Self {
        Self {
            notifications: VecDeque::with_capacity(20),
            max_notifications: 20,
        }
    }

    pub fn notify(&mut self, message: String, level: ReloadLevel) {
        self.notifications.push_back(ReloadNotification {
            message,
            timestamp: Instant::now(),
            level,
        });
        while self.notifications.len() > self.max_notifications {
            self.notifications.pop_front();
        }
    }

    pub fn clear_old(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.notifications
            .retain(|n| now.duration_since(n.timestamp) < max_age);
    }

    pub fn latest(&self) -> Option<&ReloadNotification> {
        self.notifications.back()
    }
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Reload a texture from disk.
pub fn reload_texture(
    path: &str,
    queue: &wgpu::Queue,
    device: &wgpu::Device,
) -> anyhow::Result<()> {
    let img = image::open(path).context("Failed to load image")?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(path),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba.as_raw(),
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    Ok(())
}

/// Reload a material from disk.
pub fn reload_material(path: &str) -> anyhow::Result<serde_json::Value> {
    let content = std::fs::read_to_string(path).context("Failed to read material file")?;

    let material_data: serde_json::Value =
        serde_json::from_str(&content).context("Failed to parse material JSON")?;

    Ok(material_data)
}

/// Draw hot reload notifications on screen.
pub fn draw_notifications(
    manager: &HotReloadManager,
    painter: &egui::Painter,
    screen_rect: egui::Rect,
) {
    let start_y = screen_rect.top() + 10.0;
    let mut y = start_y;

    for notification in manager.notifications.iter().rev() {
        let age = notification.timestamp.elapsed();
        if age > Duration::from_secs(5) {
            continue;
        }

        let alpha = if age > Duration::from_secs(3) {
            1.0 - (age.as_secs_f32() - 3.0) / 2.0
        } else {
            1.0
        };

        let color = match notification.level {
            ReloadLevel::Info => {
                egui::Color32::from_rgba_premultiplied(100, 200, 100, (alpha * 255.0) as u8)
            }
            ReloadLevel::Warning => {
                egui::Color32::from_rgba_premultiplied(200, 200, 100, (alpha * 255.0) as u8)
            }
            ReloadLevel::Error => {
                egui::Color32::from_rgba_premultiplied(200, 100, 100, (alpha * 255.0) as u8)
            }
        };

        let prefix = match notification.level {
            ReloadLevel::Info => "[INFO] ",
            ReloadLevel::Warning => "[WARN] ",
            ReloadLevel::Error => "[ERROR] ",
        };

        painter.text(
            egui::Pos2::new(screen_rect.left() + 10.0, y),
            egui::Align2::LEFT_CENTER,
            format!("{prefix}{}", notification.message),
            egui::FontId::proportional(12.0),
            color,
        );

        y += 20.0;
    }
}

/// A pending reload request for a changed file.
#[derive(Debug, Clone)]
pub struct ReloadRequest {
    pub path: PathBuf,
    pub resource_type: ResourceType,
    pub timestamp: Instant,
}

/// Watches directories for file changes using a debounced file system watcher.
pub struct FileWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
    receiver: Receiver<Result<Vec<DebouncedEvent>, notify::Error>>,
    watched_paths: Vec<PathBuf>,
    pending_reload: Vec<ReloadRequest>,
}

impl FileWatcher {
    /// Creates a new `FileWatcher` with a 500ms debounce delay.
    pub fn new() -> anyhow::Result<Self> {
        let (tx, receiver) = std::sync::mpsc::channel();
        let debouncer = new_debouncer(Duration::from_millis(500), tx)
            .context("Failed to create file debouncer")?;

        Ok(Self {
            _debouncer: debouncer,
            receiver,
            watched_paths: Vec::new(),
            pending_reload: Vec::new(),
        })
    }

    /// Watches a directory recursively for file changes.
    pub fn watch(&mut self, path: &Path) -> anyhow::Result<()> {
        self._debouncer
            .watcher()
            .watch(path, RecursiveMode::Recursive)
            .context(format!("Failed to watch path {}", path.display()))?;
        self.watched_paths.push(path.to_path_buf());
        Ok(())
    }

    /// Polls for file change events and queues reload requests.
    pub fn poll(&mut self) {
        while let Ok(Ok(events)) = self.receiver.try_recv() {
            for event in events {
                if let Some(resource_type) = Self::detect_resource_type(&event.path) {
                    self.pending_reload.push(ReloadRequest {
                        path: event.path,
                        resource_type,
                        timestamp: Instant::now(),
                    });
                }
            }
        }
    }

    /// Drains and returns all pending reload requests.
    pub fn take_pending(&mut self) -> Vec<ReloadRequest> {
        std::mem::take(&mut self.pending_reload)
    }

    /// Maps a file extension to a `ResourceType`.
    pub fn detect_resource_type(path: &Path) -> Option<ResourceType> {
        let ext = path.extension()?.to_str()?;
        Some(ResourceType::from_extension(ext))
    }
}

/// Manages hot reload lifecycle: watching, polling, and logging.
pub struct ReloadManager {
    file_watcher: FileWatcher,
    reload_log: Vec<String>,
    start_time: Instant,
}

impl ReloadManager {
    /// Creates a new `ReloadManager` that watches the given path.
    pub fn new(watch_path: &Path) -> anyhow::Result<Self> {
        let mut file_watcher = FileWatcher::new()?;
        file_watcher.watch(watch_path)?;
        info!("Hot reload watching: {}", watch_path.display());

        Ok(Self {
            file_watcher,
            reload_log: Vec::new(),
            start_time: Instant::now(),
        })
    }

    /// Polls for changes and logs any reload events.
    /// Returns the most recent reload log entry, if any.
    pub fn latest_reload_log(&self) -> Option<String> {
        self.reload_log.last().cloned()
    }

    pub fn update(&mut self) {
        // Don't poll here - callers should use process_pending() which handles polling
        let requests = self.file_watcher.take_pending();

        for req in &requests {
            let elapsed = self.start_time.elapsed();
            let hours = elapsed.as_secs() / 3600;
            let minutes = (elapsed.as_secs() % 3600) / 60;
            let seconds = elapsed.as_secs() % 60;
            let ts = format!("{hours:02}:{minutes:02}:{seconds:02}");
            let entry = format!(
                "[{ts}] Reload {:?}: {}",
                req.resource_type,
                req.path.display()
            );
            info!("{}", entry);
            self.reload_log.push(entry);
        }

        if !requests.is_empty() {
            warn!("Queued {} resource reload(s)", requests.len());
        }
    }

    /// Process all pending reload requests via a user-provided callback.
    pub fn process_pending<F>(&mut self, mut reload_fn: F)
    where
        F: FnMut(&Path, &str),
    {
        self.file_watcher.poll();
        let requests = self.file_watcher.take_pending();
        for req in &requests {
            let ext = req
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            reload_fn(&req.path, &ext);
        }
        if !requests.is_empty() {
            self.reload_log
                .push(format!("已重载 {} 个资源", requests.len()));
            warn!("已重载 {} 个资源", requests.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_resource_type() {
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("texture.png")),
            Some(ResourceType::Texture)
        ));
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("sound.wav")),
            Some(ResourceType::Audio)
        ));
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("model.gltf")),
            Some(ResourceType::Mesh)
        ));
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("mat.mat")),
            Some(ResourceType::Material)
        ));
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("script.lua")),
            Some(ResourceType::Script)
        ));
        assert!(matches!(
            FileWatcher::detect_resource_type(Path::new("scene.scene")),
            Some(ResourceType::Scene)
        ));
        assert!(FileWatcher::detect_resource_type(Path::new("noext")).is_none());
        assert!(FileWatcher::detect_resource_type(Path::new("unknown.xyz")).is_some());
    }

    #[test]
    fn test_file_watcher_creation() {
        let watcher = FileWatcher::new();
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_hot_reload_manager() {
        let mut manager = HotReloadManager::new();
        assert!(manager.latest().is_none());

        manager.notify("Test reload".into(), ReloadLevel::Info);
        assert_eq!(manager.notifications.len(), 1);
        assert_eq!(manager.latest().unwrap().message, "Test reload");
        assert_eq!(manager.latest().unwrap().level, ReloadLevel::Info);

        manager.notify("Warning msg".into(), ReloadLevel::Warning);
        assert_eq!(manager.notifications.len(), 2);
        assert_eq!(manager.latest().unwrap().level, ReloadLevel::Warning);
    }

    #[test]
    fn test_hot_reload_manager_max_notifications() {
        let mut manager = HotReloadManager::new();
        manager.max_notifications = 3;

        for i in 0..5 {
            manager.notify(format!("msg {i}"), ReloadLevel::Info);
        }

        assert_eq!(manager.notifications.len(), 3);
        assert_eq!(manager.notifications.front().unwrap().message, "msg 2");
        assert_eq!(manager.notifications.back().unwrap().message, "msg 4");
    }

    #[test]
    fn test_hot_reload_manager_clear_old() {
        let mut manager = HotReloadManager::new();
        manager.notify("msg".into(), ReloadLevel::Info);

        // Should not clear anything since the notification was just created
        manager.clear_old(Duration::from_secs(10));
        assert_eq!(manager.notifications.len(), 1);
    }

    #[test]
    fn test_reload_level_eq() {
        assert_eq!(ReloadLevel::Info, ReloadLevel::Info);
        assert_ne!(ReloadLevel::Info, ReloadLevel::Error);
    }
}
