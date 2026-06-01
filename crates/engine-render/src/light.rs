/// Directional light component (sun, moon).
///
/// Illuminates the entire scene from a single direction.
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub enabled: bool,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: [0.3, -1.0, -0.5],
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            enabled: true,
        }
    }
}

/// Point light component (light bulb, torch).
///
/// Emits light in all directions from a position.
#[derive(Debug, Clone)]
pub struct PointLight {
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub enabled: bool,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            range: 10.0,
            enabled: true,
        }
    }
}

/// Spot light component (flashlight, spotlight).
///
/// Emits light in a cone from a position in a direction.
#[derive(Debug, Clone)]
pub struct SpotLight {
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    /// Inner cone angle in radians (full intensity).
    pub inner_angle: f32,
    /// Outer cone angle in radians (falloff to zero).
    pub outer_angle: f32,
    pub enabled: bool,
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            direction: [0.0, -1.0, 0.0],
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            range: 15.0,
            inner_angle: 0.3,
            outer_angle: 0.6,
            enabled: true,
        }
    }
}

/// Maximum number of each light type supported in the shader.
pub const MAX_DIRECTIONAL_LIGHTS: usize = 4;
pub const MAX_POINT_LIGHTS: usize = 16;
pub const MAX_SPOT_LIGHTS: usize = 8;

/// GPU-friendly packed lighting uniform for the shader.
///
/// Contains counts followed by light data arrays.
/// Layout matches the WGSL `LightingUniform` struct.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingUniform {
    /// Ambient color + intensity (xyz = color, w = intensity).
    pub ambient: [f32; 4],
    /// Number of active directional lights.
    pub dir_count: u32,
    /// Number of active point lights.
    pub point_count: u32,
    /// Number of active spot lights.
    pub spot_count: u32,
    pub _pad: u32,
    /// Packed directional lights (dir.xyz, _pad, color.xyz, intensity).
    pub directional: [[f32; 8]; MAX_DIRECTIONAL_LIGHTS],
    /// Packed point lights (pos.xyz, range, color.xyz, intensity).
    pub point: [[f32; 8]; MAX_POINT_LIGHTS],
    /// Packed spot lights (pos.xyz, range, dir.xyz, intensity, inner, outer, _pad, _pad).
    pub spot: [[f32; 12]; MAX_SPOT_LIGHTS],
}

impl Default for LightingUniform {
    fn default() -> Self {
        Self {
            ambient: [0.05, 0.05, 0.05, 1.0],
            dir_count: 0,
            point_count: 0,
            spot_count: 0,
            _pad: 0,
            directional: [[0.0; 8]; MAX_DIRECTIONAL_LIGHTS],
            point: [[0.0; 8]; MAX_POINT_LIGHTS],
            spot: [[0.0; 12]; MAX_SPOT_LIGHTS],
        }
    }
}

impl LightingUniform {
    /// Pack directional lights from components into the uniform.
    pub fn set_directional_lights(&mut self, lights: &[(&DirectionalLight, &[f32; 3])]) {
        self.dir_count = lights.len().min(MAX_DIRECTIONAL_LIGHTS) as u32;
        for (i, (light, _pos)) in lights.iter().enumerate().take(MAX_DIRECTIONAL_LIGHTS) {
            if !light.enabled {
                continue;
            }
            self.directional[i] = [
                light.direction[0],
                light.direction[1],
                light.direction[2],
                0.0,
                light.color[0] * light.intensity,
                light.color[1] * light.intensity,
                light.color[2] * light.intensity,
                0.0,
            ];
        }
    }

    /// Pack point lights from components into the uniform.
    pub fn set_point_lights(&mut self, lights: &[(&PointLight, &[f32; 3])]) {
        self.point_count = lights.len().min(MAX_POINT_LIGHTS) as u32;
        for (i, (light, pos)) in lights.iter().enumerate().take(MAX_POINT_LIGHTS) {
            if !light.enabled {
                continue;
            }
            self.point[i] = [
                pos[0],
                pos[1],
                pos[2],
                light.range,
                light.color[0] * light.intensity,
                light.color[1] * light.intensity,
                light.color[2] * light.intensity,
                0.0,
            ];
        }
    }

    /// Pack spot lights from components into the uniform.
    pub fn set_spot_lights(&mut self, lights: &[(&SpotLight, &[f32; 3])]) {
        self.spot_count = lights.len().min(MAX_SPOT_LIGHTS) as u32;
        for (i, (light, pos)) in lights.iter().enumerate().take(MAX_SPOT_LIGHTS) {
            if !light.enabled {
                continue;
            }
            self.spot[i] = [
                pos[0],
                pos[1],
                pos[2],
                light.range,
                light.direction[0],
                light.direction[1],
                light.direction[2],
                light.intensity,
                light.inner_angle,
                light.outer_angle,
                0.0,
                0.0,
            ];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directional_light_default() {
        let l = DirectionalLight::default();
        assert!(l.enabled);
        assert!((l.intensity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_point_light_default() {
        let l = PointLight::default();
        assert!((l.range - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_spot_light_default() {
        let l = SpotLight::default();
        assert!((l.inner_angle - 0.3).abs() < 1e-6);
        assert!((l.outer_angle - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_lighting_uniform_default() {
        let u = LightingUniform::default();
        assert_eq!(u.dir_count, 0);
        assert_eq!(u.point_count, 0);
        assert_eq!(u.spot_count, 0);
    }

    #[test]
    fn test_pack_directional_lights() {
        let mut u = LightingUniform::default();
        let light = DirectionalLight {
            direction: [0.0, -1.0, 0.0],
            color: [1.0, 1.0, 1.0],
            intensity: 2.0,
            enabled: true,
        };
        let pos = [0.0; 3];
        u.set_directional_lights(&[(&light, &pos)]);
        assert_eq!(u.dir_count, 1);
        // Direction should be packed at [0..2]
        assert!((u.directional[0][0]).abs() < 1e-6);
        assert!((u.directional[0][1] - (-1.0)).abs() < 1e-6);
        // Color * intensity at [4..6]
        assert!((u.directional[0][4] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_pack_point_lights() {
        let mut u = LightingUniform::default();
        let light = PointLight {
            color: [1.0, 0.5, 0.0],
            intensity: 3.0,
            range: 20.0,
            enabled: true,
        };
        let pos = [1.0, 2.0, 3.0];
        u.set_point_lights(&[(&light, &pos)]);
        assert_eq!(u.point_count, 1);
        assert!((u.point[0][0] - 1.0).abs() < 1e-6);
        assert!((u.point[0][3] - 20.0).abs() < 1e-6);
        assert!((u.point[0][4] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_max_lights_clamp() {
        let mut u = LightingUniform::default();
        let lights: Vec<(DirectionalLight, [f32; 3])> = (0..10)
            .map(|_| (DirectionalLight::default(), [0.0; 3]))
            .collect();
        let refs: Vec<(&DirectionalLight, &[f32; 3])> =
            lights.iter().map(|(l, p)| (l, p)).collect();
        u.set_directional_lights(&refs);
        assert_eq!(u.dir_count, MAX_DIRECTIONAL_LIGHTS as u32);
    }
}
