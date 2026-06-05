use crate::light::{DirectionalLight, LightingUniform, PointLight, SpotLight};
use engine_ecs::world::World;
use engine_scene::transform::GlobalTransform;

const DEFAULT_POSITION: [f32; 3] = [0.0; 3];

/// 光源收集系统：遍历光源组件，打包到 LightingUniform
pub fn light_collect_system(world: &mut World) {
    let mut uniform = LightingUniform::default();

    // 1. Collect DirectionalLight entities
    let dir_indices = world.component_entities::<DirectionalLight>();
    let mut dir_lights: Vec<(&DirectionalLight, [f32; 3])> = Vec::new();
    for idx in &dir_indices {
        if let Some(light) = world.get_by_index::<DirectionalLight>(*idx) {
            let pos = world
                .get_by_index::<GlobalTransform>(*idx)
                .map(extract_position)
                .unwrap_or(DEFAULT_POSITION);
            dir_lights.push((light, pos));
        }
    }
    let dir_refs: Vec<(&DirectionalLight, &[f32; 3])> =
        dir_lights.iter().map(|(l, p)| (*l, p)).collect();
    uniform.set_directional_lights(&dir_refs);

    // 2. Collect PointLight entities
    let point_indices = world.component_entities::<PointLight>();
    let mut point_lights: Vec<(&PointLight, [f32; 3])> = Vec::new();
    for idx in &point_indices {
        if let Some(light) = world.get_by_index::<PointLight>(*idx) {
            let pos = world
                .get_by_index::<GlobalTransform>(*idx)
                .map(extract_position)
                .unwrap_or(DEFAULT_POSITION);
            point_lights.push((light, pos));
        }
    }
    let point_refs: Vec<(&PointLight, &[f32; 3])> =
        point_lights.iter().map(|(l, p)| (*l, p)).collect();
    uniform.set_point_lights(&point_refs);

    // 3. Collect SpotLight entities
    let spot_indices = world.component_entities::<SpotLight>();
    let mut spot_lights: Vec<(&SpotLight, [f32; 3])> = Vec::new();
    for idx in &spot_indices {
        if let Some(light) = world.get_by_index::<SpotLight>(*idx) {
            let pos = world
                .get_by_index::<GlobalTransform>(*idx)
                .map(extract_position)
                .unwrap_or(DEFAULT_POSITION);
            spot_lights.push((light, pos));
        }
    }
    let spot_refs: Vec<(&SpotLight, &[f32; 3])> =
        spot_lights.iter().map(|(l, p)| (*l, p)).collect();
    uniform.set_spot_lights(&spot_refs);

    // 4. Insert as world resource
    world.insert_resource(uniform);
}

/// Extract translation position from a GlobalTransform's Mat4.
fn extract_position(gt: &GlobalTransform) -> [f32; 3] {
    let col = gt.0.w_axis;
    [col.x, col.y, col.z]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::light::PointLight;

    #[test]
    fn test_light_collect_empty_world() {
        let mut world = World::new();
        light_collect_system(&mut world);
        let uniform = world.get_resource::<LightingUniform>().unwrap();
        assert_eq!(uniform.dir_count, 0);
        assert_eq!(uniform.point_count, 0);
        assert_eq!(uniform.spot_count, 0);
    }

    #[test]
    fn test_light_collect_point_light() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            PointLight {
                color: [1.0, 0.0, 0.0],
                intensity: 2.0,
                range: 10.0,
                enabled: true,
            },
        );
        world.add_component(e, GlobalTransform::default());

        light_collect_system(&mut world);
        let uniform = world.get_resource::<LightingUniform>().unwrap();
        assert_eq!(uniform.point_count, 1);
    }

    #[test]
    fn test_light_collect_disabled_light() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            PointLight {
                enabled: false,
                ..Default::default()
            },
        );

        light_collect_system(&mut world);
        let uniform = world.get_resource::<LightingUniform>().unwrap();
        assert_eq!(uniform.point_count, 1);
    }

    #[test]
    fn test_extract_position_identity() {
        let gt = GlobalTransform::default();
        let pos = extract_position(&gt);
        assert_eq!(pos, [0.0, 0.0, 0.0]);
    }
}
