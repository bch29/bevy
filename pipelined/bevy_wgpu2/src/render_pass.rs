use crate::{resources::WgpuResourceRefs, type_converter::WgpuInto, WgpuRenderContext};
use bevy_render2::{
    pass::RenderPass,
    pipeline::{BindGroupDescriptorId, IndexFormat, PipelineId, RenderPipelineDescriptor},
    render_resource::{BindGroupId, BufferId},
    renderer::RenderContext,
};
use bevy_utils::tracing::trace;
use std::ops::Range;

#[derive(Debug)]
pub struct WgpuRenderPass<'a> {
    pub render_pass: wgpu::RenderPass<'a>,
    pub render_context: &'a WgpuRenderContext,
    pub wgpu_resources: WgpuResourceRefs<'a>,
    pub pipeline_descriptor: Option<&'a RenderPipelineDescriptor>,
}

impl<'a> RenderPass for WgpuRenderPass<'a> {
    fn get_render_context(&self) -> &dyn RenderContext {
        self.render_context
    }

    fn set_vertex_buffer(&mut self, start_slot: u32, buffer_id: BufferId, offset: u64) {
        let buffer = self.wgpu_resources.buffers.get(&buffer_id).unwrap();
        self.render_pass
            .set_vertex_buffer(start_slot, buffer.slice(offset..));
    }

    fn set_viewport(&mut self, x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32) {
        self.render_pass
            .set_viewport(x, y, w, h, min_depth, max_depth);
    }

    fn set_scissor_rect(&mut self, x: u32, y: u32, w: u32, h: u32) {
        self.render_pass.set_scissor_rect(x, y, w, h);
    }

    fn set_stencil_reference(&mut self, reference: u32) {
        self.render_pass.set_stencil_reference(reference);
    }

    fn set_index_buffer(&mut self, buffer_id: BufferId, offset: u64, index_format: IndexFormat) {
        let buffer = self.wgpu_resources.buffers.get(&buffer_id).unwrap();
        self.render_pass
            .set_index_buffer(buffer.slice(offset..), index_format.wgpu_into());
    }

    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);
    }

    fn multi_draw_indirect(&mut self, indirect_buffer: BufferId, indirect_offset: u64, count: u32) {
        let indirect_buffer = self.wgpu_resources.buffers.get(&indirect_buffer).unwrap();
        self.render_pass
            .multi_draw_indirect(indirect_buffer, indirect_offset, count)
    }

    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.render_pass.draw(vertices, instances);
    }

    fn set_bind_group(
        &mut self,
        index: u32,
        bind_group_descriptor_id: BindGroupDescriptorId,
        bind_group: BindGroupId,
        dynamic_uniform_indices: Option<&[u32]>,
    ) {
        if let Some(bind_group_info) = self
            .wgpu_resources
            .bind_groups
            .get(&bind_group_descriptor_id)
        {
            if let Some(wgpu_bind_group) = bind_group_info.bind_groups.get(&bind_group) {
                const EMPTY: &[u32] = &[];
                let dynamic_uniform_indices =
                    if let Some(dynamic_uniform_indices) = dynamic_uniform_indices {
                        dynamic_uniform_indices
                    } else {
                        EMPTY
                    };
                self.wgpu_resources
                    .used_bind_group_sender
                    .send(bind_group)
                    .unwrap();

                trace!(
                    "set bind group {:?} {:?}: {:?}",
                    bind_group_descriptor_id,
                    dynamic_uniform_indices,
                    bind_group
                );
                self.render_pass
                    .set_bind_group(index, wgpu_bind_group, dynamic_uniform_indices);
            }
        }
    }

    fn set_pipeline(&mut self, pipeline: PipelineId) {
        let pipeline = self
            .wgpu_resources
            .render_pipelines
            .get(&pipeline)
            .expect(
            "Attempted to use a pipeline that does not exist in this `RenderPass`'s `RenderContext`.",
        );
        self.render_pass.set_pipeline(pipeline);
    }
}
