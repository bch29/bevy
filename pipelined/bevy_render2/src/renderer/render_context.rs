use super::RenderResourceContext;
use crate::{
    pass::{ComputePass, PassDescriptor, RenderPass},
    render_resource::{BufferId, TextureId},
    texture::Extent3d,
};

pub trait RenderContext {
    fn resources(&self) -> &dyn RenderResourceContext;
    fn resources_mut(&mut self) -> &mut dyn RenderResourceContext;
    fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: BufferId,
        source_offset: u64,
        destination_buffer: BufferId,
        destination_offset: u64,
        size: u64,
    );
    #[allow(clippy::too_many_arguments)]
    fn copy_buffer_to_texture(
        &mut self,
        source_buffer: BufferId,
        source_offset: u64,
        source_bytes_per_row: u32,
        destination_texture: TextureId,
        destination_origin: [u32; 3],
        destination_mip_level: u32,
        size: Extent3d,
    );
    #[allow(clippy::too_many_arguments)]
    fn copy_texture_to_buffer(
        &mut self,
        source_texture: TextureId,
        source_origin: [u32; 3],
        source_mip_level: u32,
        destination_buffer: BufferId,
        destination_offset: u64,
        destination_bytes_per_row: u32,
        size: Extent3d,
    );
    #[allow(clippy::too_many_arguments)]
    fn copy_texture_to_texture(
        &mut self,
        source_texture: TextureId,
        source_origin: [u32; 3],
        source_mip_level: u32,
        destination_texture: TextureId,
        destination_origin: [u32; 3],
        destination_mip_level: u32,
        size: Extent3d,
    );
    fn begin_render_pass(
        &mut self,
        pass_descriptor: &PassDescriptor,
        run_pass: &mut dyn FnMut(&mut dyn RenderPass),
    );

    fn begin_compute_pass(&mut self, run_pass: &mut dyn FnMut(&mut dyn ComputePass));
}
