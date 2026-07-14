//! Panel system for IMGUI layout (matches Unity's EditorWindow layout).
//!
//! Unity Reference: https://docs.unity3d.com/ScriptReference/EditorWindow.html

use egui::Context;

/// Side panel position (matches Unity's EditorWindow side panels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// Top/Bottom panel position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopBottom {
    Top,
    Bottom,
}

/// Panel system for IMGUI layout.
pub struct Panels {
    ctx: Context,
}

impl Panels {
    pub fn new(ctx: &Context) -> Self {
        Self { ctx: ctx.clone() }
    }

    /// Create a side panel (matches Unity's EditorWindow side panels).
    pub fn SidePanel(&self, side: Side, id: &str) -> SidePanelBuilder {
        let id_str = id.to_string();
        let panel = match side {
            Side::Left => egui::SidePanel::left(id_str.clone()),
            Side::Right => egui::SidePanel::right(id_str),
        };
        SidePanelBuilder {
            panel: Some(panel),
            ctx: self.ctx.clone(),
        }
    }

    /// Create a top panel (matches Unity's top toolbar area).
    pub fn TopPanel(&self, id: &str) -> TopBottomPanelBuilder {
        TopBottomPanelBuilder {
            panel: Some(egui::TopBottomPanel::top(id.to_string())),
            ctx: self.ctx.clone(),
        }
    }

    /// Create a bottom panel (matches Unity's bottom status area).
    pub fn BottomPanel(&self, id: &str) -> TopBottomPanelBuilder {
        TopBottomPanelBuilder {
            panel: Some(egui::TopBottomPanel::bottom(id.to_string())),
            ctx: self.ctx.clone(),
        }
    }

    /// Create a central panel (matches Unity's main content area).
    pub fn CentralPanel(&self) -> CentralPanelBuilder {
        CentralPanelBuilder {
            ctx: self.ctx.clone(),
        }
    }
}

pub struct SidePanelBuilder {
    panel: Option<egui::SidePanel>,
    ctx: Context,
}

impl SidePanelBuilder {
    pub fn Resizable(mut self, resizable: bool) -> Self {
        if let Some(ref mut p) = self.panel {
            let tmp = egui::SidePanel::left("tmp");
            *p = std::mem::replace(p, tmp).resizable(resizable);
        }
        self
    }

    pub fn DefaultWidth(self, _width: f32) -> Self {
        self
    }

    pub fn Show(self, f: impl FnOnce(&mut egui::Ui)) {
        if let Some(panel) = self.panel {
            panel.show(&self.ctx, |ui| f(ui));
        }
    }
}

pub struct TopBottomPanelBuilder {
    panel: Option<egui::TopBottomPanel>,
    ctx: Context,
}

impl TopBottomPanelBuilder {
    pub fn Show(self, f: impl FnOnce(&mut egui::Ui)) {
        if let Some(panel) = self.panel {
            panel.show(&self.ctx, |ui| f(ui));
        }
    }
}

pub struct CentralPanelBuilder {
    ctx: Context,
}

impl CentralPanelBuilder {
    pub fn Show(self, f: impl FnOnce(&mut egui::Ui)) {
        egui::CentralPanel::default().show(&self.ctx, |ui| f(ui));
    }
}
