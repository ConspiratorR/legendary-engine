use crate::state::EditorState;
use egui::{Color32, FontId, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use engine_asset::types::ResourceType;
use engine_ui::Gui;

#[derive(Debug, Clone)]
pub struct ResourceBrowser {
    pub current_path: String,
    pub entries: Vec<ResourceEntry>,
    pub selected_entry: Option<usize>,
}

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
    pub fn new() -> Self {
        let entries = vec![
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
                size: 128 * 1024,
                is_directory: false,
            },
            ResourceEntry {
                name: "background.png".into(),
                file_type: ResourceType::Texture,
                size: 512 * 1024,
                is_directory: false,
            },
            ResourceEntry {
                name: "sound_bg.wav".into(),
                file_type: ResourceType::Audio,
                size: 2_300_000,
                is_directory: false,
            },
            ResourceEntry {
                name: "jump.wav".into(),
                file_type: ResourceType::Audio,
                size: 86 * 1024,
                is_directory: false,
            },
            ResourceEntry {
                name: "player.glb".into(),
                file_type: ResourceType::Mesh,
                size: 5_200_000,
                is_directory: false,
            },
            ResourceEntry {
                name: "player_move.lua".into(),
                file_type: ResourceType::Script,
                size: 1500,
                is_directory: false,
            },
            ResourceEntry {
                name: "wood.mat".into(),
                file_type: ResourceType::Material,
                size: 320,
                is_directory: false,
            },
        ];

        Self {
            current_path: "Assets".into(),
            entries,
            selected_entry: None,
        }
    }

    pub fn get_icon(&self, file_type: &ResourceType) -> &'static str {
        file_type.icon()
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

pub fn draw(state: &mut EditorState, gui: &mut Gui, rect: Rect) {
    let h_scale = gui.ui.ctx().screen_rect().height() / 1080.0;
    let w_scale = gui.ui.ctx().screen_rect().width() / 1920.0;

    let painter = gui.ui.painter_at(rect);
    painter.add(Shape::rect_filled(
        rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.left(), rect.bottom()),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let header_h = 36.0 * h_scale;
    let header_rect = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), header_h));
    painter.add(Shape::rect_filled(
        header_rect,
        Rounding::ZERO,
        Color32::from_rgb(22, 22, 25),
    ));
    painter.text(
        egui::pos2(rect.left() + 12.0 * w_scale, header_rect.center().y),
        egui::Align2::LEFT_CENTER,
        "项目",
        FontId::proportional(12.0 * h_scale),
        Color32::from_gray(90),
    );

    let btn_size = 24.0 * h_scale;
    let refresh_rect = Rect::from_min_size(
        Pos2::new(
            rect.right() - btn_size - 8.0 * w_scale,
            header_rect.top() + (header_h - btn_size) / 2.0,
        ),
        Vec2::new(btn_size, btn_size),
    );
    let refresh_id = egui::Id::new("refresh_resources");
    let refresh_response = gui
        .ui
        .interact(refresh_rect, refresh_id, egui::Sense::click());
    if refresh_response.hovered() {
        painter.add(Shape::rect_filled(
            refresh_rect,
            Rounding::same(4.0 * h_scale),
            Color32::from_rgb(30, 30, 34),
        ));
    }
    painter.text(
        refresh_rect.center(),
        egui::Align2::CENTER_CENTER,
        "🔄",
        FontId::proportional(12.0 * h_scale),
        Color32::from_gray(90),
    );

    let line_y = header_rect.bottom() - 1.0;
    painter.add(Shape::line(
        vec![
            Pos2::new(rect.left(), line_y),
            Pos2::new(rect.right(), line_y),
        ],
        Stroke::new(1.0_f32, Color32::from_rgb(45, 45, 53)),
    ));

    let path_bar_h = 28.0 * h_scale;
    let path_bar_rect = Rect::from_min_size(
        Pos2::new(rect.left(), line_y),
        Vec2::new(rect.width(), path_bar_h),
    );
    painter.add(Shape::rect_filled(
        path_bar_rect,
        Rounding::ZERO,
        Color32::from_rgb(26, 26, 29),
    ));

    let path_parts: Vec<_> = state.resource_browser.current_path.split('/').collect();
    let mut path_x = 12.0 * w_scale;
    for (i, part) in path_parts.iter().enumerate() {
        let text_w = painter
            .layout(
                part.to_string(),
                FontId::proportional(11.0 * h_scale),
                Color32::from_rgb(0, 212, 170),
                f32::INFINITY,
            )
            .rect
            .width();

        painter.text(
            egui::pos2(rect.left() + path_x, path_bar_rect.center().y),
            egui::Align2::LEFT_CENTER,
            part,
            FontId::proportional(11.0 * h_scale),
            Color32::from_rgb(0, 212, 170),
        );
        path_x += text_w + 8.0 * w_scale;

        if i < path_parts.len() - 1 {
            painter.text(
                egui::pos2(rect.left() + path_x, path_bar_rect.center().y),
                egui::Align2::LEFT_CENTER,
                "/",
                FontId::proportional(11.0 * h_scale),
                Color32::from_gray(60),
            );
            path_x += 8.0 * w_scale;
        }
    }

    let content_top = path_bar_rect.bottom();
    let content_rect = Rect::from_min_size(
        Pos2::new(rect.left(), content_top),
        Vec2::new(rect.width(), rect.bottom() - content_top),
    );

    let file_h = 40.0 * h_scale;
    let file_pad = 8.0 * w_scale;
    let mut y = content_rect.top() + 4.0 * h_scale;

    for (i, entry) in state.resource_browser.entries.iter().enumerate() {
        let file_rect = Rect::from_min_size(
            Pos2::new(content_rect.left() + file_pad, y),
            Vec2::new(content_rect.width() - file_pad * 2.0, file_h),
        );

        let id = egui::Id::new("res_file").with(i as u64);
        let response = gui.ui.interact(file_rect, id, egui::Sense::click());

        if state.resource_browser.selected_entry == Some(i) || response.hovered() {
            painter.add(Shape::rect_filled(
                file_rect,
                Rounding::same(4.0 * h_scale),
                if state.resource_browser.selected_entry == Some(i) {
                    Color32::from_rgb(0, 110, 210)
                } else {
                    Color32::from_rgb(30, 30, 34)
                },
            ));
        }

        if response.clicked() {
            state.resource_browser.selected_entry = Some(i);
        }

        let icon = state.resource_browser.get_icon(&entry.file_type);
        painter.text(
            egui::pos2(file_rect.left() + 12.0 * w_scale, file_rect.center().y),
            egui::Align2::LEFT_CENTER,
            icon,
            FontId::proportional(16.0 * h_scale),
            Color32::from_gray(200),
        );

        painter.text(
            egui::pos2(file_rect.left() + 44.0 * w_scale, file_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &entry.name,
            FontId::proportional(11.0 * h_scale),
            Color32::from_rgb(232, 232, 236),
        );

        if !entry.is_directory {
            let size_str = state.resource_browser.format_size(entry.size);
            painter.text(
                egui::pos2(
                    file_rect.right() - file_pad - 8.0 * w_scale,
                    file_rect.center().y,
                ),
                egui::Align2::RIGHT_CENTER,
                &size_str,
                FontId::proportional(10.0 * h_scale),
                Color32::from_gray(90),
            );
        }

        y += file_h + 2.0 * h_scale;
    }
}
