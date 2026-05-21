use crate::graph::compile::CompiledGraph;
use crate::graph::pass::PassContext;
use crate::graph::RenderGraph;

pub struct ExecuteContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
}

impl RenderGraph {
    pub fn execute(
        &mut self,
        compiled: &CompiledGraph,
        ctx: &mut ExecuteContext<'_>,
    ) -> Result<(), wgpu::SurfaceError> {
        for compiled_pass in &compiled.passes {
            // Take ownership of the pass node to call its FnOnce closure
            let mut pass_node = self.passes.remove(0);

            // Resolve color attachment views
            let color_attachments: Vec<Option<wgpu::RenderPassColorAttachment<'_>>> = compiled_pass
                .color_attachments.iter().map(|ca| {
                    let view = self.textures[ca.view_index]
                        .as_ref()
                        .and_then(|n| n.view.as_ref())
                        .expect("Color attachment view not allocated");
                    Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: ca.load.clone(),
                            store: ca.store,
                        },
                    })
                }).collect();

            let depth_stencil: Option<wgpu::RenderPassDepthStencilAttachment<'_>> = compiled_pass
                .depth_stencil_attachment.as_ref().map(|ds| {
                    let view = self.textures[ds.view_index]
                        .as_ref()
                        .and_then(|n| n.view.as_ref())
                        .expect("Depth stencil view not allocated");
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: ds.depth_load.clone(),
                            store: ds.depth_store,
                        }),
                        stencil_ops: None,
                    }
                });

            let rpass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: compiled_pass.label.as_deref(),
                color_attachments: &color_attachments,
                depth_stencil_attachment: depth_stencil,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            let resources = crate::graph::pass::RenderGraphResources {
                textures: self.textures.iter().map(|t| {
                    t.as_ref().and_then(|n| n.view.as_ref())
                }).collect(),
                buffers: self.buffers.iter().map(|b| {
                    b.as_ref().and_then(|n| n.buffer.as_ref())
                }).collect(),
            };

            let mut pass_ctx = PassContext {
                pass: rpass,
                resources: &resources,
            };

            (pass_node.desc.execute)(&mut pass_ctx);
        }

        Ok(())
    }
}
