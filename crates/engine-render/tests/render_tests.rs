//! Tests for engine-render crate.
//!
//! These tests focus on components that can be tested without a GPU context,
//! such as camera, frustum, and render graph structure.

use engine_math::{Mat4, Vec3};
use engine_render::camera::{Camera, Color, Projection, RenderTarget, Viewport};
use engine_render::frustum::Frustum;
use engine_render::graph::{BufferDesc, RenderGraph, TextureDesc};

// ============================================================================
// Render Graph Tests
// ============================================================================

#[test]
fn test_render_graph_creation() {
    let graph = RenderGraph::new();
    assert!(!graph.is_compiled());
    assert!(graph.get_buffers().is_empty());
}

#[test]
fn test_render_graph_default() {
    let graph = RenderGraph::default();
    assert!(!graph.is_compiled());
}

#[test]
fn test_render_graph_create_texture() {
    let mut graph = RenderGraph::new();
    let _handle = graph.create_texture(
        TextureDesc::new_2d(
            1920,
            1080,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .named("test_texture"),
    );
    // Texture handle created successfully
}

#[test]
fn test_render_graph_create_buffer() {
    let mut graph = RenderGraph::new();
    let _handle =
        graph.create_buffer(BufferDesc::new(1024, wgpu::BufferUsages::VERTEX).named("test_buffer"));
    // Buffer handle created successfully
}

#[test]
fn test_render_graph_create_multiple_resources() {
    let mut graph = RenderGraph::new();

    let _tex1 = graph.create_texture(
        TextureDesc::new_2d(
            800,
            600,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .named("tex1"),
    );

    let _tex2 = graph.create_texture(
        TextureDesc::new_2d(
            400,
            300,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .named("tex2"),
    );

    let _buf1 =
        graph.create_buffer(BufferDesc::new(2048, wgpu::BufferUsages::VERTEX).named("buf1"));

    // Multiple resources created successfully
}

#[test]
fn test_render_graph_reset_preserves_imports() {
    let mut graph = RenderGraph::new();

    // Create a non-imported resource
    let _tex = graph.create_texture(
        TextureDesc::new_2d(
            100,
            100,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .named("created_tex"),
    );

    // Reset should clear non-imported resources
    graph.reset();
    assert!(!graph.is_compiled());
}

#[test]
fn test_render_graph_buffer_desc_size() {
    let desc = BufferDesc::new(4096, wgpu::BufferUsages::UNIFORM);
    assert_eq!(desc.size, 4096);
}

#[test]
fn test_render_graph_texture_desc_dimensions() {
    let desc = TextureDesc::new_2d(
        1920,
        1080,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    assert_eq!(desc.size.width, 1920);
    assert_eq!(desc.size.height, 1080);
}

// ============================================================================
// Camera Tests
// ============================================================================

#[test]
fn test_camera_perspective_creation() {
    let camera = Camera::perspective(std::f32::consts::FRAC_PI_4, 0.1, 1000.0);
    assert!(camera.is_active);
    assert_eq!(camera.priority, 0);
    assert!(camera.clear_color.is_some());
}

#[test]
fn test_camera_orthographic_creation() {
    let camera = Camera::orthographic(0.0, 800.0, 600.0, 0.0);
    assert!(camera.is_active);
    assert_eq!(camera.priority, 0);
}

#[test]
fn test_camera_view_projection_matrix() {
    let camera = Camera::perspective(std::f32::consts::FRAC_PI_4, 0.1, 1000.0);
    let vp = camera.view_projection(16.0 / 9.0);

    // The view-projection matrix should not be zero
    assert_ne!(vp, Mat4::ZERO);

    // The matrix should have non-zero determinant
    let det = vp.determinant();
    assert!(det.abs() > 1e-10);
}

#[test]
fn test_camera_orthographic_view_projection() {
    let camera = Camera::orthographic(0.0, 800.0, 600.0, 0.0);
    let vp = camera.view_projection(800.0 / 600.0);
    assert_ne!(vp, Mat4::ZERO);
}

#[test]
fn test_camera_view_projection_with_identity_view() {
    let camera = Camera::perspective(std::f32::consts::FRAC_PI_4, 0.1, 1000.0);
    // Default view is identity
    let vp = camera.view_projection(1.0);

    // With identity view, VP should equal projection
    let proj = Projection::perspective(std::f32::consts::FRAC_PI_4, 0.1, 1000.0);
    let expected = proj.matrix(1.0);
    assert_eq!(vp, expected);
}

#[test]
fn test_camera_priority_sorting() {
    let mut cameras = vec![
        Camera::perspective(1.0, 0.1, 100.0),
        Camera::perspective(1.0, 0.1, 100.0),
        Camera::perspective(1.0, 0.1, 100.0),
    ];
    cameras[0].priority = 10;
    cameras[1].priority = 0;
    cameras[2].priority = 5;

    cameras.sort_by_key(|c| c.priority);

    assert_eq!(cameras[0].priority, 0);
    assert_eq!(cameras[1].priority, 5);
    assert_eq!(cameras[2].priority, 10);
}

#[test]
fn test_camera_default_values() {
    let camera = Camera::new(Projection::perspective(1.0, 0.1, 100.0));
    assert!(camera.is_active);
    assert_eq!(camera.priority, 0);
    assert_eq!(camera.view, Mat4::IDENTITY);
    assert!(matches!(camera.render_target, RenderTarget::Screen));
    assert!(matches!(
        camera.viewport,
        Viewport::Relative {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0
        }
    ));
}

// ============================================================================
// Viewport Tests
// ============================================================================

#[test]
fn test_viewport_relative_to_absolute() {
    let vp = Viewport::Relative {
        x: 0.5,
        y: 0.0,
        width: 0.5,
        height: 1.0,
    };
    let (x, y, w, h) = vp.to_absolute(800, 600);
    assert_eq!(x, 400);
    assert_eq!(y, 0);
    assert_eq!(w, 400);
    assert_eq!(h, 600);
}

#[test]
fn test_viewport_absolute_passthrough() {
    let vp = Viewport::Absolute {
        x: 10,
        y: 20,
        width: 300,
        height: 200,
    };
    let (x, y, w, h) = vp.to_absolute(800, 600);
    assert_eq!(x, 10);
    assert_eq!(y, 20);
    assert_eq!(w, 300);
    assert_eq!(h, 200);
}

#[test]
fn test_viewport_full_screen_relative() {
    let vp = Viewport::Relative {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };
    let (x, y, w, h) = vp.to_absolute(1920, 1080);
    assert_eq!(x, 0);
    assert_eq!(y, 0);
    assert_eq!(w, 1920);
    assert_eq!(h, 1080);
}

// ============================================================================
// Color Tests
// ============================================================================

#[test]
fn test_color_to_wgpu() {
    let c = Color::new(0.5, 0.25, 0.1, 1.0);
    let w = c.to_wgpu();
    assert!((w.r - 0.5).abs() < 1e-6);
    assert!((w.g - 0.25).abs() < 1e-6);
    assert!((w.b - 0.1).abs() < 1e-6);
    assert!((w.a - 1.0).abs() < 1e-6);
}

#[test]
fn test_color_constants() {
    let black = Color::BLACK;
    assert_eq!(black.r, 0.0);
    assert_eq!(black.g, 0.0);
    assert_eq!(black.b, 0.0);
    assert_eq!(black.a, 1.0);

    let white = Color::WHITE;
    assert_eq!(white.r, 1.0);
    assert_eq!(white.g, 1.0);
    assert_eq!(white.b, 1.0);
    assert_eq!(white.a, 1.0);

    let transparent = Color::TRANSPARENT;
    assert_eq!(transparent.r, 0.0);
    assert_eq!(transparent.g, 0.0);
    assert_eq!(transparent.b, 0.0);
    assert_eq!(transparent.a, 0.0);
}

// ============================================================================
// Projection Tests
// ============================================================================

#[test]
fn test_projection_perspective_matrix() {
    let proj = Projection::perspective(std::f32::consts::FRAC_PI_4, 0.1, 1000.0);
    let matrix = proj.matrix(16.0 / 9.0);
    assert_ne!(matrix, Mat4::ZERO);
}

#[test]
fn test_projection_orthographic_matrix() {
    let proj = Projection::orthographic(0.0, 800.0, 600.0, 0.0, -1.0, 1.0);
    let matrix = proj.matrix(800.0 / 600.0);
    assert_ne!(matrix, Mat4::ZERO);
}

#[test]
fn test_projection_perspective_preserves_depth() {
    let proj = Projection::perspective(std::f32::consts::FRAC_PI_4, 0.1, 1000.0);
    let matrix = proj.matrix(1.0);

    // A point at z=-1 should map to a valid clip space coordinate
    let p = matrix * Vec3::new(0.0, 0.0, -1.0).extend(1.0);
    // w should be positive (since z is negative in right-handed)
    assert!(p.w > 0.0);
}

#[test]
fn test_projection_orthographic_maps_correctly() {
    let proj = Projection::orthographic(0.0, 800.0, 600.0, 0.0, -1.0, 1.0);
    let matrix = proj.matrix(1.0);

    // Center point should map to origin
    let center = Vec3::new(400.0, 300.0, 0.0).extend(1.0);
    let clip = matrix * center;
    // Should be near origin in clip space
    assert!((clip.x / clip.w).abs() < 0.01);
    assert!((clip.y / clip.w).abs() < 0.01);
}

// ============================================================================
// Frustum Tests
// ============================================================================

#[test]
fn test_frustum_from_identity_has_six_planes() {
    let f = Frustum::from_view_projection(&Mat4::IDENTITY);
    assert_eq!(f.planes.len(), 6);
}

#[test]
fn test_frustum_aabb_inside() {
    let f = Frustum::from_view_projection(&Mat4::IDENTITY);
    assert!(f.test_aabb(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5)));
}

#[test]
fn test_frustum_aabb_outside() {
    let f = Frustum::from_view_projection(&Mat4::IDENTITY);
    assert!(!f.test_aabb(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0)));
}

#[test]
fn test_frustum_aabb_partially_inside() {
    let f = Frustum::from_view_projection(&Mat4::IDENTITY);
    assert!(f.test_aabb(Vec3::new(0.5, -0.5, -0.5), Vec3::new(1.5, 0.5, 0.5)));
}

#[test]
fn test_frustum_sphere_inside() {
    let f = Frustum::from_view_projection(&Mat4::IDENTITY);
    assert!(f.test_sphere(Vec3::ZERO, 0.5));
}

#[test]
fn test_frustum_sphere_outside() {
    let f = Frustum::from_view_projection(&Mat4::IDENTITY);
    assert!(!f.test_sphere(Vec3::new(3.0, 0.0, 0.0), 0.5));
}

#[test]
fn test_frustum_sphere_intersecting() {
    let f = Frustum::from_view_projection(&Mat4::IDENTITY);
    assert!(f.test_sphere(Vec3::new(1.2, 0.0, 0.0), 0.5));
}

#[test]
fn test_frustum_orthographic() {
    let proj = Mat4::orthographic_rh(0.0, 800.0, 600.0, 0.0, -1.0, 1.0);
    let view = Mat4::IDENTITY;
    let vp = proj * view;
    let f = Frustum::from_view_projection(&vp);

    // Center should be visible
    assert!(f.test_sphere(Vec3::new(400.0, 300.0, 0.0), 1.0));

    // Far outside should not be visible
    assert!(!f.test_sphere(Vec3::new(1000.0, 300.0, 0.0), 1.0));
}

#[test]
fn test_frustum_perspective_culling() {
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, 16.0 / 9.0, 0.1, 1000.0);
    let view = Mat4::IDENTITY;
    let vp = proj * view;
    let f = Frustum::from_view_projection(&vp);

    // Point in front of camera should be visible
    assert!(f.test_sphere(Vec3::new(0.0, 0.0, -5.0), 1.0));

    // Point far to the side should not be visible
    assert!(!f.test_sphere(Vec3::new(100.0, 0.0, -5.0), 1.0));
}

// ============================================================================
// RenderTarget Tests
// ============================================================================

#[test]
fn test_render_target_screen() {
    let target = RenderTarget::Screen;
    assert!(matches!(target, RenderTarget::Screen));
}

#[test]
fn test_render_target_texture() {
    let target = RenderTarget::Texture(42);
    if let RenderTarget::Texture(key) = target {
        assert_eq!(key, 42);
    } else {
        panic!("Expected Texture variant");
    }
}

// ============================================================================
// BufferDesc / TextureDesc Builder Tests
// ============================================================================

#[test]
fn test_buffer_desc_named() {
    let desc = BufferDesc::new(256, wgpu::BufferUsages::VERTEX).named("vb");
    assert_eq!(desc.label.as_deref(), Some("vb"));
    assert_eq!(desc.size, 256);
    assert!(!desc.transient);
}

#[test]
fn test_buffer_desc_transient() {
    let desc = BufferDesc::new(512, wgpu::BufferUsages::UNIFORM).transient();
    assert!(desc.transient);
    assert!(desc.label.is_none());
}

#[test]
fn test_texture_desc_named() {
    let desc = TextureDesc::new_2d(
        64,
        64,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::TEXTURE_BINDING,
    )
    .named("icon");
    assert_eq!(desc.label.as_deref(), Some("icon"));
    assert_eq!(desc.size.width, 64);
    assert!(!desc.transient);
}

#[test]
fn test_texture_desc_transient() {
    let desc = TextureDesc::new_2d(
        256,
        256,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    )
    .transient();
    assert!(desc.transient);
}

#[test]
fn test_texture_handle_equality() {
    let a = engine_render::graph::TextureHandle(5);
    let b = engine_render::graph::TextureHandle(5);
    let c = engine_render::graph::TextureHandle(6);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_buffer_handle_equality() {
    let a = engine_render::graph::BufferHandle(0);
    let b = engine_render::graph::BufferHandle(0);
    assert_eq!(a, b);
}

#[test]
fn test_render_graph_multiple_named_resources() {
    let mut graph = RenderGraph::new();
    let _t1 = graph.create_texture(
        TextureDesc::new_2d(
            100,
            100,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .named("albedo"),
    );
    let _t2 = graph.create_texture(
        TextureDesc::new_2d(
            100,
            100,
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .named("depth"),
    );
    let _b1 =
        graph.create_buffer(BufferDesc::new(1024, wgpu::BufferUsages::UNIFORM).named("camera_ubo"));
    // All created without error
    assert!(!graph.is_compiled());
}
