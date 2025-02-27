use crate::{resources::WgpuResourceRefs, WgpuRenderContext};
use bevy_render2::{
    pass::ComputePass,
    pipeline::{BindGroupDescriptorId, ComputePipelineDescriptor, PipelineId},
    render_resource::BindGroupId,
    renderer::RenderContext,
};
use bevy_utils::tracing::trace;

#[derive(Debug)]
pub struct WgpuComputePass<'a> {
    pub compute_pass: wgpu::ComputePass<'a>,
    pub render_context: &'a WgpuRenderContext,
    pub wgpu_resources: WgpuResourceRefs<'a>,
    pub pipeline_descriptor: Option<&'a ComputePipelineDescriptor>,
}

impl<'a> ComputePass for WgpuComputePass<'a> {
    fn get_render_context(&self) -> &dyn RenderContext {
        self.render_context
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
                self.compute_pass
                    .set_bind_group(index, wgpu_bind_group, dynamic_uniform_indices);
            }
        }
    }

    fn set_pipeline(&mut self, pipeline: PipelineId) {
        let pipeline = self
            .wgpu_resources
            .compute_pipelines
            .get(&pipeline)
            .expect(
            "Attempted to use a pipeline that does not exist in this `RenderPass`'s `RenderContext`.",
        );
        self.compute_pass.set_pipeline(pipeline);
    }

    fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.compute_pass.dispatch(x, y, z);
    }
}
