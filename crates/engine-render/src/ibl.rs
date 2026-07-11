//! Image-Based Lighting (IBL) data structures and prefiltering math.
//!
//! Provides configuration, probe definitions, bind group layout, and analytical
//! helpers for IBL-driven ambient lighting in a PBR pipeline.

use engine_math::Vec3;

/// Configuration for IBL prefiltering passes.
#[derive(Debug, Clone)]
pub struct IblConfig {
    /// Number of mip levels for the prefiltered environment map.
    pub prefilter_mip_levels: u32,
    /// Resolution (width and height) of the irradiance cubemap.
    pub irradiance_size: u32,
    /// Resolution of the prefiltered environment cubemap.
    pub prefilter_size: u32,
    /// Resolution of the BRDF integration look-up table.
    pub brdf_lut_size: u32,
}

impl Default for IblConfig {
    fn default() -> Self {
        Self {
            prefilter_mip_levels: 5,
            irradiance_size: 64,
            prefilter_size: 256,
            brdf_lut_size: 512,
        }
    }
}

/// Handle to a cubemap texture stored in the texture store.
#[derive(Debug, Clone, Copy)]
pub struct EnvironmentMap {
    /// Key into the `TextureStore` for this cubemap.
    pub handle: u64,
    /// Number of mip levels in the cubemap.
    pub mip_levels: u32,
}

/// Complete IBL probe containing all precomputed lighting data.
#[derive(Debug, Clone, Copy)]
pub struct IblProbe {
    /// Diffuse irradiance cubemap (convolved from the environment).
    pub irradiance_map: EnvironmentMap,
    /// Specular prefiltered environment cubemap (roughness-mip-chain).
    pub prefilter_map: EnvironmentMap,
    /// Key into the `TextureStore` for the 2D BRDF integration LUT.
    pub brdf_lut: u64,
}

/// GPU uniform data for IBL parameters.
///
/// 16 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct IblUniform {
    /// Ambient light intensity multiplier.
    pub ambient_intensity: f32,
    /// IBL specular intensity multiplier.
    pub ibl_intensity: f32,
    pub _pad: f32,
    pub _pad2: f32,
}

impl Default for IblUniform {
    fn default() -> Self {
        Self {
            ambient_intensity: 1.0,
            ibl_intensity: 1.0,
            _pad: 0.0,
            _pad2: 0.0,
        }
    }
}

/// Create the bind group layout for the IBL resource set (group 3).
///
/// Bindings:
/// - `t0`: irradiance cubemap (Fragment)
/// - `t1`: prefiltered environment cubemap (Fragment)
/// - `t2`: BRDF integration LUT (Fragment)
/// - `s3`: sampler (Fragment)
pub fn create_ibl_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ibl_bind_group_layout"),
        entries: &[
            // t0: irradiance cubemap
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            },
            // t1: prefiltered environment cubemap
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            },
            // t2: BRDF integration LUT (2D)
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // s3: sampler
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// Generate sample directions for the GGX importance-sampling distribution.
///
/// Given a surface normal and roughness, returns `sample_count` half-vector
/// directions distributed according to the GGX NDF. Each entry is a unit `Vec3`.
///
/// Uses the standard Hammersley + GGX mapping (no GPU required).
pub fn importance_sample_ggx(sample_count: u32, normal: Vec3, roughness: f32) -> Vec<Vec3> {
    let a = roughness * roughness;
    let a2 = a * a;

    let up = if normal.z.abs() < 0.999 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let tangent = normal.cross(up).normalize();
    let bitangent = normal.cross(tangent);

    let mut samples = Vec::with_capacity(sample_count as usize);

    for i in 0..sample_count {
        // Hammersley quasi-random point on [0,1)^2
        let xi = hammersley_2d(i, sample_count);

        // GGX importance sampling: spherical -> Cartesian
        let cos_theta = ((1.0 - xi.1) / (1.0 + (a2 - 1.0) * xi.1)).sqrt();
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        let phi = std::f32::consts::TAU * xi.0;

        let h_local = Vec3::new(phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta);

        // Transform from tangent space to world space
        let h = tangent * h_local.x + bitangent * h_local.y + normal * h_local.z;
        samples.push(h.normalize());
    }

    samples
}

/// Hammersley 2D quasi-random sequence point.
///
/// Returns a 2D point in `[0, 1)^2` for index `i` out of `n` total samples.
fn hammersley_2d(i: u32, n: u32) -> (f32, f32) {
    let x = i as f32 / n as f32;
    let y = radical_inverse_vdc(i);
    (x, y)
}

/// Van der Corput radical inverse for a single index.
fn radical_inverse_vdc(mut bits: u32) -> f32 {
    bits = bits.rotate_right(16);
    bits = ((bits & 0x5555_5555) << 1) | ((bits & 0xAAAA_AAAA) >> 1);
    bits = ((bits & 0x3333_3333) << 2) | ((bits & 0xCCCC_CCCC) >> 2);
    bits = ((bits & 0x0F0F_0F0F) << 4) | ((bits & 0xF0F0_F0F0) >> 4);
    bits = ((bits & 0x00FF_00FF) << 8) | ((bits & 0xFF00_FF00) >> 8);
    bits as f32 * 2.328_306_4e-10 // 0x100000000 in f32
}

/// Compute a simplified IBL ambient contribution for a surface point.
///
/// Returns an approximate RGB ambient color. This is an analytical stand-in
/// for the full GPU-based IBL lookup — useful for testing and offline probes.
///
/// `roughness` and `metallic` are in `[0, 1]`. `normal` and `view_dir` should
/// be normalized.
pub fn compute_ibl_ambient(roughness: f32, metallic: f32, normal: Vec3, view_dir: Vec3) -> Vec3 {
    let n_dot_v = normal.dot(view_dir).clamp(0.001, 1.0);

    // Fresnel-Schlick approximation at normal incidence
    let f0 = Vec3::splat(0.04).lerp(Vec3::splat(0.5), metallic);
    let fresnel = f0 + (Vec3::ONE - f0) * (1.0 - n_dot_v).powi(5);

    // Diffuse term: Lambertian, reduced by Fresnel and metallicness
    let diffuse = (Vec3::ONE - fresnel) * (1.0 - metallic);

    // Specular term: rougher surfaces spread the reflection more
    let specular = fresnel * (1.0 - roughness * 0.5);

    // Combine (normalized so max ≈ 1.0 for a white environment)
    (diffuse * 0.5 + specular) * n_dot_v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_gpu::create_test_device;

    #[test]
    fn test_ibl_config_defaults() {
        let config = IblConfig::default();
        assert_eq!(config.prefilter_mip_levels, 5);
        assert_eq!(config.irradiance_size, 64);
        assert_eq!(config.prefilter_size, 256);
        assert_eq!(config.brdf_lut_size, 512);
    }

    #[test]
    fn test_ibl_uniform_size() {
        assert_eq!(std::mem::size_of::<IblUniform>(), 16);
    }

    #[test]
    fn test_ibl_uniform_default() {
        let u = IblUniform::default();
        assert!((u.ambient_intensity - 1.0).abs() < 1e-6);
        assert!((u.ibl_intensity - 1.0).abs() < 1e-6);
    }

    #[test]
    #[ignore] // Requires GPU — run with: cargo test -p engine-render -- --ignored
    fn test_ibl_bind_group_layout_creation() {
        let (device, _queue) = create_test_device();

        let layout = create_ibl_bind_group_layout(&device);
        // Verify the layout was created (non-zero handle) by creating a bind group
        // with dummy resources. If layout creation failed, this would panic.
        let dummy_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dummy_cube"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let cube_view = dummy_tex.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let dummy_2d = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dummy_2d"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let tex_2d_view = dummy_2d.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let _bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("test_ibl_bind_group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&cube_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&cube_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&tex_2d_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        // If we got here, layout creation and bind group construction succeeded.
    }

    #[test]
    fn test_ggx_importance_sampling_valid_directions() {
        let normal = Vec3::Y;
        let samples = importance_sample_ggx(64, normal, 0.5);

        assert_eq!(samples.len(), 64);

        for (i, s) in samples.iter().enumerate() {
            // Each sample should be a unit vector
            let len = s.length();
            assert!(
                (len - 1.0).abs() < 1e-4,
                "Sample {i} is not unit length: {len}"
            );
            // Each component should be finite
            assert!(
                s.x.is_finite() && s.y.is_finite() && s.z.is_finite(),
                "Sample {i} contains non-finite: {s:?}"
            );
        }
    }

    #[test]
    fn test_ggx_sampling_concentrates_toward_normal() {
        let normal = Vec3::Y;
        // Low roughness → samples cluster near the normal
        let tight = importance_sample_ggx(256, normal, 0.1);
        let avg_dot_tight: f32 = tight.iter().map(|s| s.dot(normal)).sum::<f32>() / 256.0;

        // High roughness → samples spread out
        let wide = importance_sample_ggx(256, normal, 0.9);
        let avg_dot_wide: f32 = wide.iter().map(|s| s.dot(normal)).sum::<f32>() / 256.0;

        assert!(
            avg_dot_tight > avg_dot_wide,
            "Low roughness ({avg_dot_tight}) should concentrate more than high ({avg_dot_wide})"
        );
    }

    #[test]
    fn test_compute_ibl_ambient_roughness_range() {
        let normal = Vec3::Y;
        let view = Vec3::Z;

        let low_roughness = compute_ibl_ambient(0.0, 0.0, normal, view);
        let mid_roughness = compute_ibl_ambient(0.5, 0.0, normal, view);
        let high_roughness = compute_ibl_ambient(1.0, 0.0, normal, view);

        // All should be finite and non-negative
        for (label, v) in [
            ("low", low_roughness),
            ("mid", mid_roughness),
            ("high", high_roughness),
        ] {
            assert!(
                v.x.is_finite() && v.y.is_finite() && v.z.is_finite(),
                "{label} roughness produced non-finite: {v:?}"
            );
            assert!(
                v.x >= 0.0 && v.y >= 0.0 && v.z >= 0.0,
                "{label} roughness produced negative: {v:?}"
            );
        }

        // Higher roughness should generally reduce specular contribution
        assert!(
            low_roughness.length() >= high_roughness.length(),
            "Smooth surface should have higher IBL than rough"
        );
    }

    #[test]
    fn test_compute_ibl_ambient_metallic_affects_color() {
        let normal = Vec3::Y;
        let view = Vec3::new(0.0, 1.0, 0.0).normalize();

        let dielectric = compute_ibl_ambient(0.3, 0.0, normal, view);
        let metallic = compute_ibl_ambient(0.3, 1.0, normal, view);

        // Both finite and non-negative
        assert!(dielectric.x.is_finite() && metallic.x.is_finite());
        // Metallic should differ from dielectric (different F0)
        let diff = (dielectric - metallic).length();
        assert!(
            diff > 0.001,
            "Metallic and dielectric should differ: diff={diff}"
        );
    }
}
