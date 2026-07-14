//! File/asset browser panel.
//!
//! Unity Reference: https://docs.unity3d.com/Manual/ProjectView.html
//! Uses IMGUI wrapper (engine_ui::imgui) for Unity-style layout.

use crate::state::EditorState;
use egui::Color32;
use engine_asset::types::ResourceType;

/// Resource browser panel state — displays project assets in a file-list view.
#[derive(Debug, Clone)]
pub struct ResourceBrowser {
    pub current_path: String,
    pub entries: Vec<ResourceEntry>,
    pub selected_entry: Option<usize>,
}

/// A single entry in the resource browser (file or directory).
#[derive(Debug, Clone)]
pub struct ResourceEntry {
    pub name: String,
    pub file_type: ResourceType,
    pub size: u64,
    pub is_directory: bool,
}

impl Default for ResourceBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceBrowser {
    /// Creates a new resource browser with default demo entries.
    pub fn new() -> Self {
        let mut browser = Self {
            current_path: "Assets".into(),
            entries: Vec::new(),
            selected_entry: None,
        };
        // Try to scan real filesystem; fall back to demo entries if Assets/ doesn't exist
        browser.refresh();
        if browser.entries.is_empty() {
            browser.current_path = ".".into();
            browser.refresh();
        }
        if browser.entries.is_empty() {
            browser.entries = Self::demo_entries();
        }
        browser
    }

    /// Returns hardcoded demo entries as fallback.
    fn demo_entries() -> Vec<ResourceEntry> {
        vec![
            ResourceEntry {
                name: "Images".into(),
                file_type: ResourceType::Directory,
                size: 0,
                is_directory: true,
            },
            ResourceEntry {
                name: "Audio".into(),
                file_type: ResourceType::Directory,
                size: 0,
                is_directory: true,
            },
            ResourceEntry {
                name: "Models".into(),
                file_type: ResourceType::Directory,
                size: 0,
                is_directory: true,
            },
            ResourceEntry {
                name: "Scripts".into(),
                file_type: ResourceType::Directory,
                size: 0,
                is_directory: true,
            },
            ResourceEntry {
                name: "Materials".into(),
                file_type: ResourceType::Directory,
                size: 0,
                is_directory: true,
            },
            ResourceEntry {
                name: "player.png".into(),
                file_type: ResourceType::Texture,
                size: 245760,
                is_directory: false,
            },
            ResourceEntry {
                name: "background.png".into(),
                file_type: ResourceType::Texture,
                size: 1048576,
                is_directory: false,
            },
            ResourceEntry {
                name: "sound_bg.wav".into(),
                file_type: ResourceType::Audio,
                size: 5242880,
                is_directory: false,
            },
            ResourceEntry {
                name: "jump.wav".into(),
                file_type: ResourceType::Audio,
                size: 65536,
                is_directory: false,
            },
            ResourceEntry {
                name: "character.gltf".into(),
                file_type: ResourceType::Mesh,
                size: 2097152,
                is_directory: false,
            },
            ResourceEntry {
                name: "enemy.gltf".into(),
                file_type: ResourceType::Mesh,
                size: 1572864,
                is_directory: false,
            },
            ResourceEntry {
                name: "player.lua".into(),
                file_type: ResourceType::Script,
                size: 4096,
                is_directory: false,
            },
            ResourceEntry {
                name: "game_logic.lua".into(),
                file_type: ResourceType::Script,
                size: 8192,
                is_directory: false,
            },
            ResourceEntry {
                name: "default.mat".into(),
                file_type: ResourceType::Material,
                size: 512,
                is_directory: false,
            },
            ResourceEntry {
                name: "metal.mat".into(),
                file_type: ResourceType::Material,
                size: 768,
                is_directory: false,
            },
        ]
    }

    /// Returns the icon emoji for a given resource type.
    pub fn get_icon(&self, file_type: &ResourceType) -> &'static str {
        file_type.icon()
    }

    /// Refresh the resource browser by scanning the filesystem.
    pub fn refresh(&mut self) {
        self.entries.clear();
        self.selected_entry = None;

        let path = std::path::Path::new(&self.current_path);
        if !path.exists() {
            return;
        }

        if let Ok(read_dir) = std::fs::read_dir(path) {
            for entry in read_dir.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with('.') {
                    continue;
                }
                let metadata = entry.metadata().ok();
                let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let file_type = if is_dir {
                    ResourceType::Directory
                } else {
                    let ext = std::path::Path::new(&file_name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    ResourceType::from_extension(ext)
                };
                self.entries.push(ResourceEntry {
                    name: file_name,
                    file_type,
                    size,
                    is_directory: is_dir,
                });
            }
            self.entries.sort_by(|a, b| {
                b.is_directory
                    .cmp(&a.is_directory)
                    .then(a.name.cmp(&b.name))
            });
        }
    }

    fn format_size(&self, size: u64) -> String {
        if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f32 / 1024.0)
        } else if size < 1024 * 1024 * 1024 {
            format!("{:.1} MB", size as f32 / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size as f32 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

pub fn draw(state: &mut EditorState, ui: &mut egui::Ui) {
    // Path bar
    ui.horizontal(|ui| {
        ui.label("路径:");
        let current_path = state.resource_browser.current_path.clone();
        let path_parts: Vec<_> = current_path.split('/').collect();
        for (i, part) in path_parts.iter().enumerate() {
            if ui.link(*part).clicked() {
                let new_path: String = path_parts[..=i].join("/");
                state.resource_browser.current_path = new_path;
                state.resource_browser.refresh();
            }
            if i < path_parts.len() - 1 {
                ui.label("/");
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("刷新").clicked() {
                state.resource_browser.refresh();
            }
        });
    });

    ui.separator();

    // File list
    let mut navigate_to: Option<String> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (i, entry) in state.resource_browser.entries.iter().enumerate() {
                let selected = state.resource_browser.selected_entry == Some(i);
                let icon = state.resource_browser.get_icon(&entry.file_type);

                let resp = ui.horizontal(|ui| {
                    ui.label(icon);
                    let name_btn = ui.selectable_label(selected, &entry.name);
                    if !entry.is_directory {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(state.resource_browser.format_size(entry.size))
                                    .color(Color32::from_gray(120)),
                            );
                        });
                    }
                    name_btn
                });

                if resp.inner.clicked() {
                    state.resource_browser.selected_entry = Some(i);
                }
                if resp.inner.double_clicked() && entry.is_directory {
                    navigate_to = Some(format!(
                        "{}/{}",
                        state.resource_browser.current_path, entry.name
                    ));
                }
            }
        });
    if let Some(path) = navigate_to {
        state.resource_browser.current_path = path;
        state.resource_browser.refresh();
    }
}
