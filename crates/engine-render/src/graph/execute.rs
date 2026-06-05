use crate::graph::RenderGraph;
use crate::graph::compile::CompiledGraph;
use crate::graph::pass::{self, PassContext};

pub struct ExecuteContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
}

impl RenderGraph {
    pub fn execute<'a>(
        &self,
        compiled: &CompiledGraph,
        closures: &mut Vec<pass::ExecuteFn<'a>>,
        ctx: &mut ExecuteContext<'_>,
    ) -> Result<(), wgpu::SurfaceError> {
        for (compiled_pass, execute_fn) in compiled.passes.iter().zip(closures.drain(..)) {
            // Resolve color attachment views
            let color_attachments: Vec<Option<wgpu::RenderPassColorAttachment<'_>>> = compiled_pass
                .color_attachments
                .iter()
                .map(|ca| {
                    let view = self.textures[ca.view_index]
                        .as_ref()
                        .and_then(|n| n.view.as_ref())
                        .expect("Color attachment view must be allocated");
                    Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: ca.load,
                            store: ca.store,
                        },
                    })
                })
                .collect();

            let depth_stencil: Option<wgpu::RenderPassDepthStencilAttachment<'_>> =
                compiled_pass.depth_stencil_attachment.as_ref().map(|ds| {
                    let view = self.textures[ds.view_index]
                        .as_ref()
                        .and_then(|n| n.view.as_ref())
                        .expect("Depth stencil view must be allocated");
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: ds.depth_load,
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

            let resources = pass::RenderGraphResources {
                textures: self
                    .textures
                    .iter()
                    .map(|t| t.as_ref().and_then(|n| n.view.as_ref()))
                    .collect(),
                buffers: self
                    .buffers
                    .iter()
                    .map(|b| b.as_ref().and_then(|n| n.get_buffer()))
                    .collect(),
            };

            let mut pass_ctx = PassContext {
                pass: rpass,
                resources: &resources,
                device: ctx.device,
                queue: ctx.queue,
            };

            (execute_fn)(&mut pass_ctx);
        }

        Ok(())
    }
}
