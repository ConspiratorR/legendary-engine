use engine_ui::gui::Gui;
use engine_ui::layout::GuiLayout;
use engine_ui::retained::{LayoutType, UiTree, WidgetKind};
use engine_ui::skin::GuiSkin;
use engine_ui::theme::{Theme, ThemeManager};
use egui::{Pos2, Rect};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run_in_ui(mut f: impl FnMut(&mut Gui)) {
    let ctx = egui::Context::default();
    let skin = GuiSkin::default();
    let f_ref = &mut f;
    let _ = ctx.run(egui::RawInput::default(), move |ctx| {
        egui::Area::new(egui::Id::new("test_area")).show(ctx, |ui| {
            let mut gui = Gui::new(ui, &skin);
            f_ref(&mut gui);
        });
    });
}

fn run_layout(mut f: impl FnMut(&mut GuiLayout)) {
    let ctx = egui::Context::default();
    let skin = GuiSkin::default();
    let f_ref = &mut f;
    let _ = ctx.run(egui::RawInput::default(), move |ctx| {
        let mut layout = GuiLayout::new(ctx, &skin);
        f_ref(&mut layout);
    });
}

// ---------------------------------------------------------------------------
// Label creation
// ---------------------------------------------------------------------------

#[test]
fn label_draws_without_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(100.0, 20.0));
        gui.label(rect, "Hello");
    });
}

#[test]
fn colored_label_draws_without_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(100.0, 20.0));
        gui.colored_label(rect, "Colored", egui::Color32::RED);
    });
}

#[test]
fn status_item_draws_without_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(150.0, 20.0));
        gui.status_item(rect, "Online", egui::Color32::GREEN);
    });
}

// ---------------------------------------------------------------------------
// Button creation
// ---------------------------------------------------------------------------

#[test]
fn button_returns_false_without_click() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(100.0, 20.0));
        assert!(!gui.button(rect, "Click"));
    });
}

#[test]
fn repeat_button_returns_false_without_input() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(100.0, 20.0));
        assert!(!gui.repeat_button(rect, "Hold"));
    });
}

#[test]
fn tool_button_returns_false_without_click() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(32.0, 32.0));
        assert!(!gui.tool_button(rect, "Tool", false));
    });
}

#[test]
fn tab_returns_false_without_click() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(60.0, 32.0));
        assert!(!gui.tab(rect, "Tab", false));
    });
}

// ---------------------------------------------------------------------------
// Panel layout
// ---------------------------------------------------------------------------

#[test]
fn panel_header_returns_content_rect() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(200.0, 36.0));
        let content = gui.panel_header(rect, "Header");
        assert!(content.top() >= rect.bottom());
    });
}

#[test]
fn separator_draws_without_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(100.0, 4.0));
        gui.separator(rect);
    });
}

#[test]
fn separator_h_draws_without_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(100.0, 4.0));
        gui.separator_h(rect);
    });
}

#[test]
fn separator_v_draws_without_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(4.0, 100.0));
        gui.separator_v(rect);
    });
}

#[test]
fn menu_bar_returns_none_without_click() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(400.0, 32.0));
        assert!(gui.menu_bar(rect, &["File", "Edit"]).is_none());
    });
}

#[test]
fn toolbar_empty_no_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(300.0, 24.0));
        let mut sel = 0;
        gui.toolbar(rect, &mut sel, &[]);
        assert_eq!(sel, 0);
    });
}

#[test]
fn selection_grid_empty_no_panic() {
    run_in_ui(|gui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(200.0, 100.0));
        let mut sel = 0;
        gui.selection_grid(rect, &mut sel, &[], 2);
        assert_eq!(sel, 0);
    });
}

// ---------------------------------------------------------------------------
// UI builder (GuiLayout)
// ---------------------------------------------------------------------------

#[test]
fn gui_layout_constructs() {
    let ctx = egui::Context::default();
    let skin = GuiSkin::default();
    let _layout = GuiLayout::new(&ctx, &skin);
}

#[test]
fn horizontal_scope_label_does_not_panic() {
    run_layout(|layout| {
        layout.horizontal(|h| {
            h.label("Hello");
            h.label("World");
        });
    });
}

#[test]
fn vertical_scope_label_does_not_panic() {
    run_layout(|layout| {
        layout.vertical(|v| {
            v.label("Hello");
            v.label("World");
        });
    });
}

#[test]
fn vertical_scope_button_returns_false_without_click() {
    run_layout(|layout| {
        layout.vertical(|v| {
            assert!(!v.button("OK"));
        });
    });
}

#[test]
fn vertical_scope_box_draws_without_panic() {
    run_layout(|layout| {
        layout.vertical(|v| {
            v.box_("Content", 200.0, 100.0);
        });
    });
}

#[test]
fn vertical_scope_separator_draws_without_panic() {
    run_layout(|layout| {
        layout.vertical(|v| {
            v.separator();
        });
    });
}

#[test]
fn window_creates_scope() {
    run_layout(|layout| {
        let mut rect = Rect::from_min_size(Pos2::new(10.0, 10.0), egui::vec2(200.0, 300.0));
        layout.window("Window", &mut rect, |_v| {});
    });
}

#[test]
fn nested_horizontal_in_vertical() {
    run_layout(|layout| {
        layout.vertical(|v| {
            v.label("Top");
            v.horizontal(|h| {
                h.label("Left");
                h.label("Right");
            });
            v.label("Bottom");
        });
    });
}

// ---------------------------------------------------------------------------
// Retained-mode UiTree
// ---------------------------------------------------------------------------

#[test]
fn ui_tree_create_and_access_widget() {
    let mut tree = UiTree::new();
    let id = tree.create_widget(WidgetKind::Label("test".into()));
    let w = tree.get(id).unwrap();
    assert_eq!(w.kind, WidgetKind::Label("test".into()));
    assert!(w.visible);
}

#[test]
fn ui_tree_parent_child_relationship() {
    let mut tree = UiTree::new();
    let root = tree.create_widget(WidgetKind::Container);
    let child = tree.create_widget(WidgetKind::Button("OK".into()));
    tree.set_root(root);
    tree.add_child(root, child);

    let root_w = tree.get(root).unwrap();
    assert_eq!(root_w.children.len(), 1);
    assert_eq!(tree.get(child).unwrap().parent, Some(root));
}

#[test]
fn ui_tree_remove_widget() {
    let mut tree = UiTree::new();
    let root = tree.create_widget(WidgetKind::Container);
    let child = tree.create_widget(WidgetKind::Label("x".into()));
    tree.set_root(root);
    tree.add_child(root, child);
    tree.remove_widget(child);

    assert!(tree.get(child).is_none());
    assert_eq!(tree.get(root).unwrap().children.len(), 0);
}

#[test]
fn ui_tree_vertical_layout() {
    let mut tree = UiTree::new();
    let root = tree.create_widget(WidgetKind::Container);
    let c1 = tree.create_widget(WidgetKind::Label("A".into()));
    let c2 = tree.create_widget(WidgetKind::Label("B".into()));
    tree.set_root(root);
    tree.add_child(root, c1);
    tree.add_child(root, c2);
    tree.layout(egui::vec2(400.0, 300.0));

    let b1 = tree.get(c1).unwrap().bounds;
    let b2 = tree.get(c2).unwrap().bounds;
    assert!(b1.min.y < b2.min.y);
}

#[test]
fn ui_tree_horizontal_layout() {
    let mut tree = UiTree::new();
    let root = tree.create_widget(WidgetKind::Container);
    tree.get_mut(root).unwrap().layout = LayoutType::Horizontal;
    let c1 = tree.create_widget(WidgetKind::Label("A".into()));
    let c2 = tree.create_widget(WidgetKind::Label("B".into()));
    tree.set_root(root);
    tree.add_child(root, c1);
    tree.add_child(root, c2);
    tree.layout(egui::vec2(400.0, 300.0));

    let b1 = tree.get(c1).unwrap().bounds;
    let b2 = tree.get(c2).unwrap().bounds;
    assert!(b1.min.x < b2.min.x);
}

#[test]
fn ui_tree_grid_layout() {
    let mut tree = UiTree::new();
    let root = tree.create_widget(WidgetKind::Container);
    tree.get_mut(root).unwrap().layout = LayoutType::Grid { columns: 2 };
    let ids: Vec<_> = (0..4)
        .map(|_| tree.create_widget(WidgetKind::Label("cell".into())))
        .collect();
    tree.set_root(root);
    for &id in &ids {
        tree.add_child(root, id);
    }
    tree.layout(egui::vec2(200.0, 200.0));

    let b0 = tree.get(ids[0]).unwrap().bounds;
    let b1 = tree.get(ids[1]).unwrap().bounds;
    let b2 = tree.get(ids[2]).unwrap().bounds;
    assert!(b0.min.x < b1.min.x);
    assert!(b0.min.y < b2.min.y);
}

#[test]
fn ui_tree_hit_test() {
    let mut tree = UiTree::new();
    let root = tree.create_widget(WidgetKind::Container);
    let child = tree.create_widget(WidgetKind::Button("OK".into()));
    tree.set_root(root);
    tree.add_child(root, child);
    tree.layout(egui::vec2(400.0, 300.0));

    let child_bounds = tree.get(child).unwrap().bounds;
    let hit = tree.hit_test(child_bounds.center());
    assert_eq!(hit, Some(child));
}

#[test]
fn ui_tree_hit_test_outside_returns_none() {
    let mut tree = UiTree::new();
    let root = tree.create_widget(WidgetKind::Container);
    tree.set_root(root);
    tree.layout(egui::vec2(400.0, 300.0));

    assert!(tree.hit_test(Pos2::new(500.0, 500.0)).is_none());
}

#[test]
fn ui_tree_len_and_is_empty() {
    let mut tree = UiTree::new();
    assert!(tree.is_empty());
    let root = tree.create_widget(WidgetKind::Container);
    assert_eq!(tree.len(), 1);
    tree.remove_widget(root);
    assert!(tree.is_empty());
}

#[test]
fn ui_tree_focus() {
    let mut tree = UiTree::new();
    let id = tree.create_widget(WidgetKind::TextField {
        text: String::new(),
        placeholder: "type".into(),
    });
    assert!(tree.focused().is_none());
    tree.set_focus(Some(id));
    assert_eq!(tree.focused(), Some(id));
    tree.set_focus(None);
    assert!(tree.focused().is_none());
}

// ---------------------------------------------------------------------------
// Theme manager
// ---------------------------------------------------------------------------

#[test]
fn theme_manager_has_builtins() {
    let mgr = ThemeManager::new();
    assert!(mgr.has_theme(&Theme::Dark));
    assert!(mgr.has_theme(&Theme::Light));
}

#[test]
fn theme_manager_switches_theme() {
    let mut mgr = ThemeManager::new();
    assert_eq!(*mgr.active_theme(), Theme::Dark);
    mgr.set_active_theme(Theme::Light, 0.0);
    assert_eq!(*mgr.active_theme(), Theme::Light);
}

#[test]
fn theme_manager_transition_lifecycle() {
    let mut mgr = ThemeManager::new();
    mgr.set_active_theme(Theme::Light, 1.0);
    assert!(mgr.transition().is_some());

    assert!(!mgr.update_transition(0.5));
    assert!(mgr.update_transition(0.5));
    assert!(mgr.transition().is_none());
}

#[test]
fn theme_manager_register_custom() {
    let mut mgr = ThemeManager::new();
    mgr.register_theme("ocean", GuiSkin::default());
    assert!(mgr.has_theme(&Theme::Custom("ocean".into())));
}

#[test]
fn theme_manager_resolve_style_cascade() {
    use engine_ui::skin::{ColorBlock, GuiStyle};
    use engine_ui::theme::WidgetStyleKey;

    let mut mgr = ThemeManager::new();

    let red = egui::Color32::from_rgb(200, 0, 0);
    mgr.set_style_override(
        WidgetStyleKey::type_only("button"),
        GuiStyle {
            normal: ColorBlock {
                background: red,
                text: egui::Color32::WHITE,
                border: None,
            },
            ..GuiStyle::default()
        },
    );

    let resolved = mgr.resolve_style(None, "button");
    assert_eq!(resolved.normal.background, red);
}

// ---------------------------------------------------------------------------
// Skin
// ---------------------------------------------------------------------------

#[test]
fn gui_skin_default_exists() {
    let skin = GuiSkin::default();
    assert_eq!(skin.button.normal.text, egui::Color32::WHITE);
}

#[test]
fn gui_style_default_font_size() {
    let style = engine_ui::skin::GuiStyle::default();
    assert!(style.font_size > 0.0);
}
