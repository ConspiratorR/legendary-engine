//! Theme management for the GUI system.
//!
//! Provides [`Theme`] enumeration, [`ThemeManager`] for runtime theme switching,
//! and CSS-like style cascading with [`resolve_style`]. Supports dark/light
//! built-in themes, custom user themes, and transition hooks for cross-fade
//! animation.

use std::collections::HashMap;

use egui::{Color32, Margin, Rounding};

use crate::skin::{ColorBlock, GuiSkin, GuiStyle};

// ---------------------------------------------------------------------------
// Theme enum
// ---------------------------------------------------------------------------

/// Identifies a theme by kind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Theme {
    /// Built-in dark theme.
    Dark,
    /// Built-in light theme.
    Light,
    /// User-registered custom theme by name.
    Custom(String),
}

impl Theme {
    /// Returns the canonical key used inside [`ThemeManager`].
    pub fn key(&self) -> String {
        match self {
            Theme::Dark => "dark".into(),
            Theme::Light => "light".into(),
            Theme::Custom(name) => name.clone(),
        }
    }
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Theme::Dark => write!(f, "Dark"),
            Theme::Light => write!(f, "Light"),
            Theme::Custom(name) => write!(f, "{name}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Theme transition
// ---------------------------------------------------------------------------

/// Describes an in-progress cross-fade between two themes.
///
/// The renderer can read `progress` (0.0 → 1.0) each frame and blend
/// between the two skins accordingly.
#[derive(Debug, Clone)]
pub struct ThemeTransition {
    /// The theme we are transitioning away from.
    pub from: Theme,
    /// The theme we are transitioning towards.
    pub to: Theme,
    /// Blend factor in `0.0 ..= 1.0`.  `0.0` = fully `from`, `1.0` = fully `to`.
    pub progress: f32,
    /// Duration of the transition in seconds.
    pub duration_secs: f32,
}

impl ThemeTransition {
    /// Advance the transition by `dt` seconds.  Clamps to `1.0`.
    pub fn advance(&mut self, dt: f32) {
        if self.duration_secs > 0.0 {
            self.progress = (self.progress + dt / self.duration_secs).min(1.0);
        } else {
            self.progress = 1.0;
        }
    }

    /// Returns `true` when the transition has fully completed.
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }
}

// ---------------------------------------------------------------------------
// Style override key
// ---------------------------------------------------------------------------

/// Identifies a single widget whose style should be overridden.
///
/// Used as the key in [`ThemeManager::style_overrides`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WidgetStyleKey {
    /// Optional widget instance name (e.g. `"my_button"`).
    pub name: Option<String>,
    /// Widget type (e.g. `"button"`, `"label"`).
    pub widget_type: String,
}

impl WidgetStyleKey {
    /// Override for a specific named widget instance.
    pub fn named(name: impl Into<String>, widget_type: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            widget_type: widget_type.into(),
        }
    }

    /// Override for all widgets of a given type.
    pub fn type_only(widget_type: impl Into<String>) -> Self {
        Self {
            name: None,
            widget_type: widget_type.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// ThemeManager
// ---------------------------------------------------------------------------

/// Central registry for GUI themes.
///
/// Ships with two built-in themes (`"dark"` and `"light"`).  Additional
/// themes can be registered at runtime via [`register_theme`](Self::register_theme).
///
/// # Cascading order (highest → lowest priority)
///
/// 1. Per-widget instance override (`name + widget_type`)
/// 2. Widget-type override (`widget_type` only)
/// 3. Active theme skin default
pub struct ThemeManager {
    themes: HashMap<String, GuiSkin>,
    active_theme: Theme,
    previous_theme: Option<Theme>,
    transition: Option<ThemeTransition>,
    /// Per-widget / per-type style overrides layered on top of the active skin.
    style_overrides: HashMap<WidgetStyleKey, GuiStyle>,
}

impl ThemeManager {
    /// Create a new manager with the built-in dark and light themes pre-loaded.
    /// Dark theme is active by default.
    pub fn new() -> Self {
        let mut themes = HashMap::new();
        themes.insert("dark".into(), dark_theme());
        themes.insert("light".into(), light_theme());

        Self {
            themes,
            active_theme: Theme::Dark,
            previous_theme: None,
            transition: None,
            style_overrides: HashMap::new(),
        }
    }

    // -- theme registration -------------------------------------------------

    /// Register (or overwrite) a named custom theme.
    pub fn register_theme(&mut self, name: impl Into<String>, skin: GuiSkin) {
        self.themes.insert(name.into(), skin);
    }

    /// Returns `true` if a theme with the given key exists.
    pub fn has_theme(&self, theme: &Theme) -> bool {
        self.themes.contains_key(&theme.key())
    }

    /// List all registered theme names (including built-ins).
    pub fn theme_names(&self) -> Vec<String> {
        self.themes.keys().cloned().collect()
    }

    // -- active theme -------------------------------------------------------

    /// Switch to `theme`, optionally starting a cross-fade transition.
    ///
    /// If `transition_secs` is greater than zero a [`ThemeTransition`] is
    /// created that can be advanced each frame via
    /// [`update_transition`](Self::update_transition).
    pub fn set_active_theme(&mut self, theme: Theme, transition_secs: f32) {
        if self.active_theme == theme {
            return;
        }
        if !self.has_theme(&theme) {
            return;
        }
        self.previous_theme = Some(self.active_theme.clone());
        self.active_theme = theme.clone();

        if transition_secs > 0.0 {
            self.transition = Some(ThemeTransition {
                from: self.previous_theme.clone().unwrap(),
                to: theme,
                progress: 0.0,
                duration_secs: transition_secs,
            });
        } else {
            self.transition = None;
        }
    }

    /// Borrow the [`GuiSkin`] for the currently active theme.
    pub fn active_skin(&self) -> &GuiSkin {
        self.themes
            .get(&self.active_theme.key())
            .expect("active theme must be registered")
    }

    /// Returns the current active [`Theme`] identifier.
    pub fn active_theme(&self) -> &Theme {
        &self.active_theme
    }

    // -- transition ---------------------------------------------------------

    /// Advance the current transition by `dt` seconds.
    ///
    /// Returns `true` when the transition just completed this call.
    pub fn update_transition(&mut self, dt: f32) -> bool {
        if let Some(ref mut t) = self.transition {
            t.advance(dt);
            if t.is_complete() {
                self.transition = None;
                return true;
            }
        }
        false
    }

    /// Borrow the in-progress transition, if any.
    pub fn transition(&self) -> Option<&ThemeTransition> {
        self.transition.as_ref()
    }

    // -- style overrides ----------------------------------------------------

    /// Insert a per-widget or per-widget-type style override.
    pub fn set_style_override(&mut self, key: WidgetStyleKey, style: GuiStyle) {
        self.style_overrides.insert(key, style);
    }

    /// Remove a previously set override.
    pub fn clear_style_override(&mut self, key: &WidgetStyleKey) {
        self.style_overrides.remove(key);
    }

    /// Remove all overrides.
    pub fn clear_all_overrides(&mut self) {
        self.style_overrides.clear();
    }

    /// Resolve the effective style for a widget, applying the cascade:
    ///
    /// 1. Per-widget instance override (`name + widget_type`)
    /// 2. Widget-type override (`widget_type` only)
    /// 3. Theme default from the active skin
    ///
    /// Returns a *merged* [`GuiStyle`] — only the fields present in the
    /// override are replaced; the rest come from the theme default.
    pub fn resolve_style(&self, widget_name: Option<&str>, widget_type: &str) -> GuiStyle {
        let base = self.style_for_widget_type(widget_type);

        // Layer 2: widget-type override (name = None)
        let type_key = WidgetStyleKey::type_only(widget_type);
        let base = if let Some(type_override) = self.style_overrides.get(&type_key) {
            merge_styles(&base, type_override)
        } else {
            base
        };

        // Layer 1: per-widget instance override (name = Some)
        if let Some(name) = widget_name {
            let instance_key = WidgetStyleKey::named(name, widget_type);
            if let Some(instance_override) = self.style_overrides.get(&instance_key) {
                return merge_styles(&base, instance_override);
            }
        }

        base
    }

    /// Look up the default style for a widget type from the active skin.
    fn style_for_widget_type(&self, widget_type: &str) -> GuiStyle {
        let skin = self.active_skin();
        match widget_type {
            "label" => skin.label.clone(),
            "button" => skin.button.clone(),
            "box" | "box_" => skin.box_.clone(),
            "text_field" => skin.text_field.clone(),
            "toggle" => skin.toggle.clone(),
            "window" => skin.window.clone(),
            "slider" => skin.slider.clone(),
            "toolbar" => skin.toolbar.clone(),
            "selection_grid" => skin.selection_grid.clone(),
            _ => GuiStyle::default(),
        }
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Merge helper — overlay non-default values from `override_` onto `base`.
// ---------------------------------------------------------------------------

/// Merge two styles: `base` supplies defaults, `override_` replaces any
/// field whose value differs from the `GuiStyle::default()` for that field.
///
/// This is a "sparse merge" — only explicitly changed fields in `override_`
/// take effect.
fn merge_styles(base: &GuiStyle, override_: &GuiStyle) -> GuiStyle {
    let defaults = GuiStyle::default();

    GuiStyle {
        normal: merge_color_block(&base.normal, &override_.normal, &defaults.normal),
        hover: merge_color_block(&base.hover, &override_.hover, &defaults.hover),
        active: merge_color_block(&base.active, &override_.active, &defaults.active),
        focused: merge_color_block(&base.focused, &override_.focused, &defaults.focused),
        border: if override_.border != defaults.border {
            override_.border
        } else {
            base.border
        },
        margins: if override_.margins != defaults.margins {
            override_.margins
        } else {
            base.margins
        },
        font_size: if (override_.font_size - defaults.font_size).abs() > f32::EPSILON {
            override_.font_size
        } else {
            base.font_size
        },
    }
}

fn merge_color_block(
    base: &ColorBlock,
    override_: &ColorBlock,
    defaults: &ColorBlock,
) -> ColorBlock {
    ColorBlock {
        background: if override_.background != defaults.background {
            override_.background
        } else {
            base.background
        },
        text: if override_.text != defaults.text {
            override_.text
        } else {
            base.text
        },
        border: if override_.border != defaults.border {
            override_.border
        } else {
            base.border
        },
    }
}

// ---------------------------------------------------------------------------
// Built-in dark theme
// ---------------------------------------------------------------------------

/// Produce the built-in dark theme skin (mirrors the existing `GuiSkin::default`).
pub fn dark_theme() -> GuiSkin {
    GuiSkin::default()
}

// ---------------------------------------------------------------------------
// Built-in light theme
// ---------------------------------------------------------------------------

/// Produce the built-in light theme skin — white/light-gray palette with blue accents.
pub fn light_theme() -> GuiSkin {
    let accent = Color32::from_rgb(50, 110, 200);
    let accent_hover = Color32::from_rgb(70, 130, 220);

    let default_style = GuiStyle {
        normal: ColorBlock {
            background: Color32::from_rgb(245, 245, 245),
            text: Color32::from_rgb(30, 30, 30),
            border: None,
        },
        hover: ColorBlock {
            background: Color32::from_rgb(230, 230, 230),
            text: Color32::from_rgb(20, 20, 20),
            border: None,
        },
        active: ColorBlock {
            background: Color32::from_rgb(210, 210, 210),
            text: Color32::from_rgb(10, 10, 10),
            border: None,
        },
        focused: ColorBlock {
            background: Color32::from_rgb(235, 235, 235),
            text: Color32::from_rgb(20, 20, 20),
            border: None,
        },
        border: Rounding::same(2.0),
        margins: Margin::symmetric(4.0, 2.0),
        font_size: 14.0,
    };

    GuiSkin {
        label: default_style.clone(),
        button: GuiStyle {
            normal: ColorBlock {
                background: accent,
                text: Color32::WHITE,
                border: None,
            },
            hover: ColorBlock {
                background: accent_hover,
                text: Color32::WHITE,
                border: None,
            },
            active: ColorBlock {
                background: Color32::from_rgb(40, 95, 180),
                text: Color32::WHITE,
                border: None,
            },
            focused: ColorBlock {
                background: accent,
                text: Color32::WHITE,
                border: None,
            },
            ..default_style.clone()
        },
        box_: GuiStyle {
            normal: ColorBlock {
                background: Color32::from_rgb(250, 250, 250),
                text: Color32::from_rgb(30, 30, 30),
                border: Some(Color32::from_rgb(200, 200, 200)),
            },
            ..default_style.clone()
        },
        text_field: GuiStyle {
            normal: ColorBlock {
                background: Color32::WHITE,
                text: Color32::from_rgb(30, 30, 30),
                border: Some(Color32::from_rgb(190, 190, 190)),
            },
            focused: ColorBlock {
                background: Color32::WHITE,
                text: Color32::from_rgb(20, 20, 20),
                border: Some(accent),
            },
            ..default_style.clone()
        },
        toggle: default_style.clone(),
        window: GuiStyle {
            normal: ColorBlock {
                background: Color32::from_rgb(240, 240, 240),
                text: Color32::from_rgb(30, 30, 30),
                border: Some(Color32::from_rgb(200, 200, 200)),
            },
            ..default_style.clone()
        },
        slider: GuiStyle {
            normal: ColorBlock {
                background: Color32::from_rgb(220, 220, 220),
                text: Color32::from_rgb(30, 30, 30),
                border: None,
            },
            hover: ColorBlock {
                background: Color32::from_rgb(200, 200, 200),
                text: Color32::from_rgb(20, 20, 20),
                border: None,
            },
            active: ColorBlock {
                background: accent,
                text: Color32::WHITE,
                border: None,
            },
            focused: ColorBlock {
                background: Color32::from_rgb(210, 210, 210),
                text: Color32::from_rgb(20, 20, 20),
                border: None,
            },
            ..default_style.clone()
        },
        toolbar: GuiStyle {
            normal: ColorBlock {
                background: Color32::from_rgb(235, 235, 235),
                text: Color32::from_rgb(80, 80, 80),
                border: None,
            },
            active: ColorBlock {
                background: accent,
                text: Color32::WHITE,
                border: None,
            },
            ..default_style.clone()
        },
        selection_grid: GuiStyle {
            normal: ColorBlock {
                background: Color32::from_rgb(240, 240, 240),
                text: Color32::from_rgb(80, 80, 80),
                border: Some(Color32::from_rgb(200, 200, 200)),
            },
            active: ColorBlock {
                background: accent,
                text: Color32::WHITE,
                border: Some(accent_hover),
            },
            ..default_style
        },
        font: egui::FontId::proportional(14.0),
        cursor: None,
    }
}

// ---------------------------------------------------------------------------
// Serde support (feature-gated)
// ---------------------------------------------------------------------------

/// Serializable mirror of [`ColorBlock`].
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SerdeColorBlock {
    background: [u8; 4],
    text: [u8; 4],
    border: Option<[u8; 4]>,
}

#[cfg(feature = "serde")]
impl From<&ColorBlock> for SerdeColorBlock {
    fn from(cb: &ColorBlock) -> Self {
        Self {
            background: cb.background.to_array(),
            text: cb.text.to_array(),
            border: cb.border.map(|c| c.to_array()),
        }
    }
}

#[cfg(feature = "serde")]
impl SerdeColorBlock {
    fn to_color_block(&self) -> ColorBlock {
        ColorBlock {
            background: Color32::from_rgba_premultiplied(
                self.background[0],
                self.background[1],
                self.background[2],
                self.background[3],
            ),
            text: Color32::from_rgba_premultiplied(
                self.text[0],
                self.text[1],
                self.text[2],
                self.text[3],
            ),
            border: self
                .border
                .map(|c| Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3])),
        }
    }
}

/// Serializable mirror of [`GuiStyle`].
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SerdeGuiStyle {
    normal: SerdeColorBlock,
    hover: SerdeColorBlock,
    active: SerdeColorBlock,
    focused: SerdeColorBlock,
    border_radius: f32,
    margin_x: f32,
    margin_y: f32,
    font_size: f32,
}

#[cfg(feature = "serde")]
impl From<&GuiStyle> for SerdeGuiStyle {
    fn from(gs: &GuiStyle) -> Self {
        Self {
            normal: SerdeColorBlock::from(&gs.normal),
            hover: SerdeColorBlock::from(&gs.hover),
            active: SerdeColorBlock::from(&gs.active),
            focused: SerdeColorBlock::from(&gs.focused),
            border_radius: gs.border.ne,
            margin_x: gs.margins.left,
            margin_y: gs.margins.top,
            font_size: gs.font_size,
        }
    }
}

#[cfg(feature = "serde")]
impl SerdeGuiStyle {
    fn to_gui_style(&self) -> GuiStyle {
        GuiStyle {
            normal: self.normal.to_color_block(),
            hover: self.hover.to_color_block(),
            active: self.active.to_color_block(),
            focused: self.focused.to_color_block(),
            border: Rounding::same(self.border_radius),
            margins: Margin::symmetric(self.margin_x, self.margin_y),
            font_size: self.font_size,
        }
    }
}

/// Serializable full theme configuration that can be saved/loaded.
///
/// Enable with the `serde` feature.
#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ThemeConfig {
    pub name: String,
    pub label: SerdeGuiStyle,
    pub button: SerdeGuiStyle,
    pub box_: SerdeGuiStyle,
    pub text_field: SerdeGuiStyle,
    pub toggle: SerdeGuiStyle,
    pub window: SerdeGuiStyle,
    pub slider: SerdeGuiStyle,
    pub toolbar: SerdeGuiStyle,
    pub selection_grid: SerdeGuiStyle,
    pub font_size: f32,
}

#[cfg(feature = "serde")]
impl ThemeConfig {
    /// Serialize the current active skin of a [`ThemeManager`] to a config.
    pub fn from_skin(name: impl Into<String>, skin: &GuiSkin) -> Self {
        Self {
            name: name.into(),
            label: SerdeGuiStyle::from(&skin.label),
            button: SerdeGuiStyle::from(&skin.button),
            box_: SerdeGuiStyle::from(&skin.box_),
            text_field: SerdeGuiStyle::from(&skin.text_field),
            toggle: SerdeGuiStyle::from(&skin.toggle),
            window: SerdeGuiStyle::from(&skin.window),
            slider: SerdeGuiStyle::from(&skin.slider),
            toolbar: SerdeGuiStyle::from(&skin.toolbar),
            selection_grid: SerdeGuiStyle::from(&skin.selection_grid),
            font_size: skin.font.size,
        }
    }

    /// Convert this config back into a [`GuiSkin`].
    pub fn to_skin(&self) -> GuiSkin {
        GuiSkin {
            label: self.label.to_gui_style(),
            button: self.button.to_gui_style(),
            box_: self.box_.to_gui_style(),
            text_field: self.text_field.to_gui_style(),
            toggle: self.toggle.to_gui_style(),
            window: self.window.to_gui_style(),
            slider: self.slider.to_gui_style(),
            toolbar: self.toolbar.to_gui_style(),
            selection_grid: self.selection_grid.to_gui_style(),
            font: egui::FontId::proportional(self.font_size),
            cursor: None,
        }
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_key_variants() {
        assert_eq!(Theme::Dark.key(), "dark");
        assert_eq!(Theme::Light.key(), "light");
        assert_eq!(Theme::Custom("ocean".into()).key(), "ocean");
    }

    #[test]
    fn test_theme_display() {
        assert_eq!(Theme::Dark.to_string(), "Dark");
        assert_eq!(Theme::Light.to_string(), "Light");
        assert_eq!(Theme::Custom("solarized".into()).to_string(), "solarized");
    }

    #[test]
    fn test_manager_has_builtins() {
        let mgr = ThemeManager::new();
        assert!(mgr.has_theme(&Theme::Dark));
        assert!(mgr.has_theme(&Theme::Light));
        assert!(!mgr.has_theme(&Theme::Custom("nope".into())));
    }

    #[test]
    fn test_active_theme_switching() {
        let mut mgr = ThemeManager::new();
        assert_eq!(*mgr.active_theme(), Theme::Dark);

        mgr.set_active_theme(Theme::Light, 0.0);
        assert_eq!(*mgr.active_theme(), Theme::Light);
        // Previous theme was stored but transition duration was 0 — no transition object.
        assert!(mgr.transition().is_none());
    }

    #[test]
    fn test_transition_lifecycle() {
        let mut mgr = ThemeManager::new();
        mgr.set_active_theme(Theme::Light, 1.0);

        let t = mgr.transition().expect("transition should exist");
        assert_eq!(t.from, Theme::Dark);
        assert_eq!(t.to, Theme::Light);
        assert!((t.progress - 0.0).abs() < f32::EPSILON);

        // Advance half-way.
        assert!(!mgr.update_transition(0.5));
        let t = mgr.transition().unwrap();
        assert!((t.progress - 0.5).abs() < 0.01);

        // Advance to completion.
        assert!(mgr.update_transition(0.5));
        assert!(mgr.transition().is_none());
    }

    #[test]
    fn test_resolve_style_cascade() {
        let mut mgr = ThemeManager::new();

        // Override all buttons: red background.
        let red_bg = Color32::from_rgb(200, 0, 0);
        let type_override = GuiStyle {
            normal: ColorBlock {
                background: red_bg,
                text: Color32::WHITE,
                border: None,
            },
            ..GuiStyle::default()
        };
        mgr.set_style_override(WidgetStyleKey::type_only("button"), type_override);

        // Without a name, the type override applies.
        let resolved = mgr.resolve_style(None, "button");
        assert_eq!(resolved.normal.background, red_bg);

        // Named instance override: green background.
        let green_bg = Color32::from_rgb(0, 180, 0);
        let instance_override = GuiStyle {
            normal: ColorBlock {
                background: green_bg,
                text: Color32::WHITE,
                border: None,
            },
            ..GuiStyle::default()
        };
        mgr.set_style_override(WidgetStyleKey::named("ok_btn", "button"), instance_override);

        // Named widget wins over type-only override.
        let resolved = mgr.resolve_style(Some("ok_btn"), "button");
        assert_eq!(resolved.normal.background, green_bg);

        // Other buttons still get the type override.
        let resolved = mgr.resolve_style(Some("cancel_btn"), "button");
        assert_eq!(resolved.normal.background, red_bg);
    }

    #[test]
    fn test_register_custom_theme() {
        let mut mgr = ThemeManager::new();
        let skin = light_theme(); // reuse light as a base
        mgr.register_theme("my_theme", skin);
        assert!(mgr.has_theme(&Theme::Custom("my_theme".into())));

        mgr.set_active_theme(Theme::Custom("my_theme".into()), 0.0);
        assert_eq!(*mgr.active_theme(), Theme::Custom("my_theme".into()));
    }

    #[test]
    fn test_light_theme_distinct_colors() {
        let light = light_theme();
        let dark = dark_theme();

        // Light theme windows should be lighter than dark theme windows.
        assert!(
            light.window.normal.background.to_array()[0]
                > dark.window.normal.background.to_array()[0]
        );
        // Light theme button text is white, but background is blue-ish (not gray).
        let l_btn = light.button.normal.background.to_array();
        assert!(l_btn[2] > l_btn[0], "light button should be blue-tinted");
    }

    #[test]
    fn test_clear_overrides() {
        let mut mgr = ThemeManager::new();
        let override_style = GuiStyle {
            font_size: 24.0,
            ..GuiStyle::default()
        };
        let key = WidgetStyleKey::named("big", "label");
        mgr.set_style_override(key.clone(), override_style);

        let resolved = mgr.resolve_style(Some("big"), "label");
        assert!((resolved.font_size - 24.0).abs() < f32::EPSILON);

        mgr.clear_style_override(&key);
        let resolved = mgr.resolve_style(Some("big"), "label");
        assert!((resolved.font_size - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_transition_same_theme_is_noop() {
        let mut mgr = ThemeManager::new();
        mgr.set_active_theme(Theme::Dark, 1.0);
        // Setting the same theme should not create a transition.
        assert!(mgr.transition().is_none());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_theme_config_roundtrip() {
        let skin = light_theme();
        let config = ThemeConfig::from_skin("light_copy", &skin);
        let json = config.to_json().expect("serialize");
        let restored = ThemeConfig::from_json(&json).expect("deserialize");
        assert_eq!(restored.name, "light_copy");

        let restored_skin = restored.to_skin();
        // Spot-check a value survived the round-trip.
        assert_eq!(restored_skin.button.normal.text, skin.button.normal.text);
    }
}
