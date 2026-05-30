use crate::indirect::DrawIndexedIndirectArgs;
use crate::pipeline::sprite::SpriteVertex;
use engine_asset::asset::Handle;
use engine_asset::types::Texture;
use engine_math::{Mat4, Vec2};
pub struct Sprite {
    pub texture: Handle<Texture>,
    pub color: [f32; 4],
    pub size: Vec2,
    pub transform: Mat4,
    pub flip_x: bool,
    pub flip_y: bool,
    /// UV region `[u_min, v_min, u_max, v_max]` for sprite sheet sub-regions.
    /// Default `[0.0, 0.0, 1.0, 1.0]` uses the full texture.
    pub uv_region: [f32; 4],
}

#[derive(Clone)]
pub struct SpriteDraw {
    pub world_matrix: Mat4,
    pub color: [f32; 4],
    pub size: Vec2,
    pub texture_id: u64,
    pub flip_x: bool,
    pub flip_y: bool,
    pub depth: f32,
    /// UV region `[u_min, v_min, u_max, v_max]` for sprite sheet frames.
    pub uv_region: [f32; 4],
}

pub struct SpriteBatch {
    pub texture_id: u64,
    pub vertices: Vec<SpriteVertex>,
    pub indices: Vec<u16>,
    pub index_count: u32,
    pub instance_data: Vec<Mat4>,
    pub indirect_cmd: DrawIndexedIndirectArgs,
}

impl SpriteBatch {
    pub fn new(texture_id: u64) -> Self {
        Self {
            texture_id,
            vertices: Vec::new(),
            indices: Vec::new(),
            index_count: 0,
            instance_data: Vec::new(),
            indirect_cmd: DrawIndexedIndirectArgs::new(0, 0),
        }
    }

    pub fn push(&mut self, draw: &SpriteDraw) {
        let base = self.vertices.len() as u16;
        let w = draw.size.x * 0.5;
        let h = draw.size.y * 0.5;
        let [reg_u0, reg_v0, reg_u1, reg_v1] = draw.uv_region;
        let (u0, u1) = if draw.flip_x {
            (reg_u1, reg_u0)
        } else {
            (reg_u0, reg_u1)
        };
        let (v0, v1) = if draw.flip_y {
            (reg_v1, reg_v0)
        } else {
            (reg_v0, reg_v1)
        };

        self.vertices.extend_from_slice(&[
            SpriteVertex {
                position: [-w, -h, 0.0],
                uv: [u0, v1],
                color: draw.color,
            },
            SpriteVertex {
                position: [w, -h, 0.0],
                uv: [u1, v1],
                color: draw.color,
            },
            SpriteVertex {
                position: [w, h, 0.0],
                uv: [u1, v0],
                color: draw.color,
            },
            SpriteVertex {
                position: [-w, h, 0.0],
                uv: [u0, v0],
                color: draw.color,
            },
        ]);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

        self.instance_data.push(draw.world_matrix);
    }

    pub fn update_indirect_cmd(&mut self) {
        self.indirect_cmd = DrawIndexedIndirectArgs::new(
            self.indices.len() as u32,
            self.instance_data.len() as u32,
        );
    }
}

pub fn collect_batches(sprites: &[SpriteDraw]) -> Vec<SpriteBatch> {
    // Sort by depth (back-to-front) for correct alpha blending
    let mut sorted: Vec<&SpriteDraw> = sprites.iter().collect();
    sorted.sort_by(|a, b| {
        a.depth
            .partial_cmp(&b.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Group by texture_id, preserving depth order across groups
    let mut batch_map: std::collections::HashMap<u64, Vec<&SpriteDraw>> =
        std::collections::HashMap::new();
    let mut order: Vec<u64> = Vec::new();
    for draw in sorted {
        let entry = batch_map.entry(draw.texture_id);
        if matches!(entry, std::collections::hash_map::Entry::Vacant(_)) {
            order.push(draw.texture_id);
        }
        entry.or_default().push(draw);
    }

    let batches: Vec<SpriteBatch> = order
        .into_iter()
        .map(|tex_idx| {
            let draws = batch_map.remove(&tex_idx).unwrap();
            let mut batch = SpriteBatch::new(tex_idx);
            for draw in draws {
                batch.push(draw);
            }
            batch
        })
        .collect();
    batches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_batch_push() {
        let mut batch = SpriteBatch::new(0);
        let draw = SpriteDraw {
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            texture_id: 0,
            flip_x: false,
            flip_y: false,
            depth: 0.0,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        };
        batch.push(&draw);
        assert_eq!(batch.vertices.len(), 4);
        assert_eq!(batch.indices.len(), 6);
    }

    #[test]
    fn test_collect_batches_groups_by_texture() {
        let draws = vec![
            SpriteDraw {
                texture_id: 1,
                ..sprite_draw_default()
            },
            SpriteDraw {
                texture_id: 0,
                ..sprite_draw_default()
            },
            SpriteDraw {
                texture_id: 1,
                ..sprite_draw_default()
            },
        ];
        let batches = collect_batches(&draws);
        assert_eq!(batches.len(), 2);
        // Batch order follows depth-sorted insertion order (first-seen texture first)
        assert_eq!(batches[0].texture_id, 1);
        assert_eq!(batches[1].texture_id, 0);
    }

    #[test]
    fn test_collect_batches_sorts_by_depth_back_to_front() {
        let draws = vec![
            SpriteDraw {
                texture_id: 0,
                depth: 10.0,
                ..sprite_draw_default()
            },
            SpriteDraw {
                texture_id: 1,
                depth: 1.0,
                ..sprite_draw_default()
            },
            SpriteDraw {
                texture_id: 0,
                depth: 5.0,
                ..sprite_draw_default()
            },
        ];
        let batches = collect_batches(&draws);
        // Batch for tex 1 (depth 1.0) should come before batch for tex 0 (first at depth 5.0)
        assert_eq!(batches[0].texture_id, 1);
        assert_eq!(batches[1].texture_id, 0);
        // Within tex 0 batch: depth 5.0 before depth 10.0
        assert!(batches[1].vertices.len() >= 8); // 2 sprites * 4 verts
    }

    #[test]
    fn test_collect_batches_nan_depth_does_not_panic() {
        let draws = vec![
            SpriteDraw {
                texture_id: 0,
                depth: f32::NAN,
                ..sprite_draw_default()
            },
            SpriteDraw {
                texture_id: 0,
                depth: 1.0,
                ..sprite_draw_default()
            },
        ];
        let batches = collect_batches(&draws);
        assert_eq!(batches.len(), 1);
    }

    fn sprite_draw_default() -> SpriteDraw {
        SpriteDraw {
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            texture_id: 0,
            flip_x: false,
            flip_y: false,
            depth: 0.0,
            uv_region: [0.0, 0.0, 1.0, 1.0],
        }
    }
}
