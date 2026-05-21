use wgpu::util::DeviceExt;
use engine_math::{Mat4, Vec2};
use crate::pipeline::sprite::SpriteVertex;

pub struct Sprite {
    pub texture_handle: u64,
    pub color: [f32; 4],
    pub size: Vec2,
    pub flip_x: bool,
    pub flip_y: bool,
}

pub struct SpriteDraw {
    pub world_matrix: Mat4,
    pub color: [f32; 4],
    pub size: Vec2,
    pub texture_index: u64,
    pub flip_x: bool,
    pub flip_y: bool,
}

pub struct SpriteBatch {
    pub texture_index: u64,
    pub vertices: Vec<SpriteVertex>,
    pub indices: Vec<u16>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub index_count: u32,
}

impl SpriteBatch {
    pub fn new(texture_index: u64) -> Self {
        Self {
            texture_index,
            vertices: Vec::new(),
            indices: Vec::new(),
            vertex_buffer: None,
            index_buffer: None,
            index_count: 0,
        }
    }

    pub fn push(&mut self, draw: &SpriteDraw) {
        let base = self.vertices.len() as u16;
        let w = draw.size.x * 0.5;
        let h = draw.size.y * 0.5;
        let (u0, u1) = if draw.flip_x { (1.0, 0.0) } else { (0.0, 1.0) };
        let (v0, v1) = if draw.flip_y { (1.0, 0.0) } else { (0.0, 1.0) };

        self.vertices.extend_from_slice(&[
            SpriteVertex { position: [-w, -h, 0.0], uv: [u0, v1], color: draw.color },
            SpriteVertex { position: [ w, -h, 0.0], uv: [u1, v1], color: draw.color },
            SpriteVertex { position: [ w,  h, 0.0], uv: [u1, v0], color: draw.color },
            SpriteVertex { position: [-w,  h, 0.0], uv: [u0, v0], color: draw.color },
        ]);
        self.indices.extend_from_slice(&[
            base, base + 1, base + 2,
            base, base + 2, base + 3,
        ]);
    }

    pub fn upload(&mut self, device: &wgpu::Device) {
        let vertex_data = bytemuck::cast_slice(&self.vertices);
        self.vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite_vertex_buffer"),
            contents: vertex_data,
            usage: wgpu::BufferUsages::VERTEX,
        }));
        let index_data = bytemuck::cast_slice(&self.indices);
        self.index_count = self.indices.len() as u32;
        self.index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite_index_buffer"),
            contents: index_data,
            usage: wgpu::BufferUsages::INDEX,
        }));
    }
}

pub fn collect_batches(sprites: &[SpriteDraw]) -> Vec<SpriteBatch> {
    let mut batch_map: std::collections::HashMap<u64, Vec<&SpriteDraw>> = std::collections::HashMap::new();
    for draw in sprites {
        batch_map.entry(draw.texture_index).or_default().push(draw);
    }

    let mut batches: Vec<SpriteBatch> = batch_map.into_iter().map(|(tex_idx, draws)| {
        let mut batch = SpriteBatch::new(tex_idx);
        for draw in draws {
            batch.push(draw);
        }
        batch
    }).collect();

    batches.sort_by_key(|b| b.texture_index);
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
            texture_index: 0,
            flip_x: false,
            flip_y: false,
        };
        batch.push(&draw);
        assert_eq!(batch.vertices.len(), 4);
        assert_eq!(batch.indices.len(), 6);
    }

    #[test]
    fn test_collect_batches_groups_by_texture() {
        let draws = vec![
            SpriteDraw { texture_index: 1, ..sprite_draw_default() },
            SpriteDraw { texture_index: 0, ..sprite_draw_default() },
            SpriteDraw { texture_index: 1, ..sprite_draw_default() },
        ];
        let batches = collect_batches(&draws);
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].texture_index, 0);
        assert_eq!(batches[1].texture_index, 1);
    }

    fn sprite_draw_default() -> SpriteDraw {
        SpriteDraw {
            world_matrix: Mat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(100.0, 100.0),
            texture_index: 0,
            flip_x: false,
            flip_y: false,
        }
    }
}
