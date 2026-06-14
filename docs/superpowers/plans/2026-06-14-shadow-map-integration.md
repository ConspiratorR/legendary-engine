# Shadow Map Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the existing shadow map generation into the deferred lighting pass so 3D scenes render with real-time shadows.

**Architecture:** The shadow generation side (depth texture, depth-only pipeline, light matrix computation, cascade splits) is already complete. We need to build the consumption side: a GPU uniform buffer for shadow parameters, a combined bind group (uniform + depth texture + comparison sampler) at group slot 3 in the lighting pipeline, and shader modifications to sample the shadow map with PCF filtering and apply shadow attenuation to directional light contribution.

**Tech Stack:** wgpu, WGSL, Rust

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `crates/engine-render/src/shadow.rs` | Modify | Extend `ShadowPass::bind_group_layout` to include uniform buffer binding; add `ShadowPass::create_lighting_bind_group()` method |
| `crates/engine-render/src/deferred.rs` | Modify | Add `shadow_bind_group_layout` to `DeferredPass`; add it as group 3 in lighting pipeline layout |
| `crates/engine-render/src/renderer.rs` | Modify | Create shadow uniform buffer + bind group in `init_deferred_resources()`; write shadow uniform each frame; set bind group 3 in lighting pass |
| `crates/engine-render/src/pipeline/deferred_lighting.wgsl` | Modify | Add `@group(3)` shadow bindings, `ShadowUniform` struct, light-space projection, PCF sampling, shadow attenuation |

---

### Task 1: Extend ShadowPass bind group layout for lighting consumption

**Files:**
- Modify: `crates/engine-render/src/shadow.rs`

The existing `ShadowPass::bind_group_layout` has 2 entries (depth texture + comparison sampler). The lighting shader also needs the `ShadowUniform` buffer. We extend the layout with a 3rd binding for the uniform buffer, and add a method to create the combined bind group.

- [ ] **Step 1: Add uniform buffer binding to the shadow bind group layout**

In `ShadowPass::new()`, modify the `bind_group_layout` creation (lines 121-141) to add binding 2:

```rust
let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: Some("shadow_bind_group_layout"),
    entries: &[
        // binding 0: shadow depth texture
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        },
        // binding 1: comparison sampler
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
            count: None,
        },
        // binding 2: shadow uniform buffer
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ],
});
```

- [ ] **Step 2: Add `create_lighting_bind_group()` method to ShadowPass**

Add a new method after `create_bind_group()` that creates a bind group suitable for the lighting pass (includes the uniform buffer):

```rust
/// Create a bind group for the lighting pass that includes the shadow uniform buffer.
pub fn create_lighting_bind_group(
    &self,
    device: &wgpu::Device,
    uniform_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("shadow_lighting_bind_group"),
        layout: &self.bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&self.depth_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&self.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    })
}
```

- [ ] **Step 3: Remove the unused `create_bind_group()` method**

The existing `create_bind_group()` (lines 218-233) is never called in production code. The shadow depth pass uses push constants and no bind groups. Remove it to avoid confusion, since the layout now has 3 bindings but the old method only provides 2.

- [ ] **Step 4: Commit**

```bash
git add crates/engine-render/src/shadow.rs
git commit -m "feat(render): extend shadow bind group layout for lighting consumption"
```

---

### Task 2: Add shadow bind group layout to DeferredPass lighting pipeline

**Files:**
- Modify: `crates/engine-render/src/deferred.rs`

The `DeferredPass::new()` creates the lighting pipeline layout with 3 bind group layouts (camera=0, light=1, gbuffer=2). We need to add the shadow layout as group 3.

- [ ] **Step 1: Add `shadow_bind_group_layout` field to `DeferredPass`**

```rust
pub struct DeferredPass {
    pub geometry_pipeline: wgpu::RenderPipeline,
    pub lighting_pipeline: wgpu::RenderPipeline,
    pub gbuffer_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub light_bind_group_layout: wgpu::BindGroupLayout,
    pub shadow_bind_group_layout: wgpu::BindGroupLayout,
}
```

- [ ] **Step 2: Modify `DeferredPass::new()` to accept a shadow layout parameter**

Change the signature:

```rust
pub fn new(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    shadow_bind_group_layout: wgpu::BindGroupLayout,
) -> Self {
```

- [ ] **Step 3: Add shadow layout to the lighting pipeline layout**

Modify the lighting pipeline layout (around line 466):

```rust
let lighting_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: Some("deferred_lighting_layout"),
    bind_group_layouts: &[
        &camera_bind_group_layout,
        &light_bind_group_layout,
        &gbuffer_bind_group_layout,
        &shadow_bind_group_layout,
    ],
    push_constant_ranges: &[],
});
```

- [ ] **Step 4: Store the shadow layout in the struct return**

```rust
Self {
    geometry_pipeline,
    lighting_pipeline,
    gbuffer_bind_group_layout,
    camera_bind_group_layout,
    light_bind_group_layout,
    shadow_bind_group_layout,
}
```

- [ ] **Step 5: Update the test `test_deferred_pass_creation`**

Create a shadow bind group layout for testing:

```rust
#[test]
fn test_deferred_pass_creation() {
    let device = create_test_device();
    let format = wgpu::TextureFormat::Bgra8UnormSrgb;

    let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("test_shadow_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let deferred = DeferredPass::new(&device, format, shadow_layout);
    let _ = deferred.geometry_pipeline;
    let _ = deferred.lighting_pipeline;
}
```

- [ ] **Step 6: Commit**

```bash
git add crates/engine-render/src/deferred.rs
git commit -m "feat(render): add shadow bind group layout to deferred lighting pipeline"
```

---

### Task 3: Create shadow uniform buffer and bind group in renderer

**Files:**
- Modify: `crates/engine-render/src/renderer.rs`

- [ ] **Step 1: Add shadow fields to the `Renderer` struct**

```rust
shadow_uniform_buffer: Option<wgpu::Buffer>,
shadow_lighting_bind_group: Option<wgpu::BindGroup>,
```

- [ ] **Step 2: Initialize to `None` in `Renderer::new()`**

```rust
shadow_uniform_buffer: None,
shadow_lighting_bind_group: None,
```

- [ ] **Step 3: Create shadow resources in `init_deferred_resources()`**

After shadow pass creation, before deferred pass creation:

```rust
let shadow_pass = ShadowPass::new(device, ShadowMapConfig::default());
let shadow_layout = shadow_pass.bind_group_layout.clone();

let shadow_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("shadow_uniform_buffer"),
    size: std::mem::size_of::<ShadowUniform>() as u64,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
});

let deferred_pass = DeferredPass::new(device, self.config.format, shadow_layout);
let shadow_lighting_bind_group = shadow_pass.create_lighting_bind_group(device, &shadow_uniform_buffer);
```

- [ ] **Step 4: Store the new resources**

```rust
self.shadow_uniform_buffer = Some(shadow_uniform_buffer);
self.shadow_lighting_bind_group = Some(shadow_lighting_bind_group);
```

- [ ] **Step 5: Write shadow uniform each frame**

After computing `shadow_uniform` in `render_frame_3d()`:

```rust
let shadow_buf = self.shadow_uniform_buffer.as_ref().expect("shadow uniform buffer");
queue.write_buffer(shadow_buf, 0, bytemuck::bytes_of(&shadow_uniform));
```

- [ ] **Step 6: Set shadow bind group in lighting pass**

In `record_deferred_lighting_pass()`, remove the `_shadow_uniform` parameter and add:

```rust
let shadow_bg = self.shadow_lighting_bind_group.as_ref().expect("shadow bind group");
pass.set_bind_group(3, shadow_bg, &[]);
```

Update the call site in `render_frame_3d()` accordingly.

- [ ] **Step 7: Commit**

```bash
git add crates/engine-render/src/renderer.rs
git commit -m "feat(render): create shadow uniform buffer and bind group in renderer"
```

---

### Task 4: Add shadow sampling to the deferred lighting shader

**Files:**
- Modify: `crates/engine-render/src/pipeline/deferred_lighting.wgsl`

- [ ] **Step 1: Add shadow bindings after G-Buffer bindings (after line 44)**

```wgsl
struct ShadowUniform {
    light_vp: mat4x4<f32>,
    shadow_bias: f32,
    normal_bias: f32,
    cascade_count: u32,
    _pad: f32,
}

@group(3) @binding(0)
var shadow_map: texture_depth_2d;

@group(3) @binding(1)
var shadow_sampler: sampler_comparison;

@group(3) @binding(2)
var<uniform> shadow: ShadowUniform;
```

- [ ] **Step 2: Add shadow sampling helper (before `fs_lighting`)**

```wgsl
fn sample_shadow(world_position: vec3<f32>, normal: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    let light_pos = shadow.light_vp * vec4(world_position, 1.0);
    let ndc = light_pos.xyz / light_pos.w;
    let shadow_uv = vec2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let shadow_depth = ndc.z;

    let n_dot_l = saturate(dot(normal, light_dir));
    let bias = shadow.shadow_bias + shadow.normal_bias * (1.0 - n_dot_l);

    if shadow_uv.x < 0.0 || shadow_uv.x > 1.0 || shadow_uv.y < 0.0 || shadow_uv.y > 1.0 {
        return 1.0;
    }
    if shadow_depth < 0.0 || shadow_depth > 1.0 {
        return 1.0;
    }

    let texel_size = 1.0 / 2048.0;
    let offsets = array<vec2<f32>, 5>(
        vec2(0.0, 0.0),
        vec2(-1.0, 0.0) * texel_size,
        vec2(1.0, 0.0) * texel_size,
        vec2(0.0, -1.0) * texel_size,
        vec2(0.0, 1.0) * texel_size,
    );

    var shadow_acc = 0.0;
    for (var i = 0u; i < 5u; i++) {
        let uv = shadow_uv + offsets[i];
        shadow_acc += textureSampleCompare(shadow_map, shadow_sampler, uv, shadow_depth - bias);
    }

    return shadow_acc / 5.0;
}
```

- [ ] **Step 3: Apply shadow to directional light loop**

In the directional light loop, after computing `diffuse + specular`:

```wgsl
var shadow_factor = 1.0;
if i == 0u {
    shadow_factor = sample_shadow(world_position, normal, light_dir);
}
total_light += (diffuse + specular) * shadow_factor;
```

- [ ] **Step 4: Build and verify**

```bash
cargo build -p engine-render
cargo test -p engine-render
cargo clippy -p engine-render
cargo fmt -p engine-render
```

- [ ] **Step 5: Commit**

```bash
git add crates/engine-render/src/pipeline/deferred_lighting.wgsl
git commit -m "feat(render): add shadow map sampling with PCF to deferred lighting shader"
```

---

## Design Decisions

1. **Bind group slot 3** — Slots 0-2 are taken (camera, light, gbuffer).
2. **5-tap disk PCF** — Center + 4 cardinal offsets. Uses `textureSampleCompare` for hardware comparison.
3. **Shadow on first directional light only** — Shadow map is from one light's perspective.
4. **Normal bias** — Offsets depth along surface normal to reduce acne.
5. **Texel size 1/2048** — Matches default `ShadowMapConfig.resolution`. Acceptable for now.
6. **Layout cloned** — `wgpu::BindGroupLayout` is `Arc`-backed, clone is cheap.
