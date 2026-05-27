// crates/engine-render/src/indirect.rs

use bytemuck::{Pod, Zeroable};

/// DrawIndexedIndirect 命令参数
/// 对应 wgpu 的 DrawIndexedIndirectArgs
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct DrawIndexedIndirectArgs {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

impl DrawIndexedIndirectArgs {
    pub fn new(index_count: u32, instance_count: u32) -> Self {
        Self {
            index_count,
            instance_count,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        }
    }
}
