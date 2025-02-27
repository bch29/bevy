use super::WgpuRenderResourceContext;
use crate::{
    compute_pass::WgpuComputePass, resources::WgpuResourceRefs, type_converter::WgpuInto,
    WgpuRenderPass,
};

use bevy_render2::{
    pass::{
        ComputePass, PassDescriptor, RenderPass, RenderPassColorAttachment,
        RenderPassDepthStencilAttachment, TextureAttachment,
    },
    render_resource::{BufferId, TextureId},
    renderer::{RenderContext, RenderResourceContext},
    texture::Extent3d,
};

use std::sync::Arc;

#[derive(Debug, Default)]
pub struct LazyCommandEncoder {
    command_encoder: Option<wgpu::CommandEncoder>,
}

impl LazyCommandEncoder {
    pub fn get_or_create(&mut self, device: &wgpu::Device) -> &mut wgpu::CommandEncoder {
        match self.command_encoder {
            Some(ref mut command_encoder) => command_encoder,
            None => {
                self.create(device);
                self.command_encoder.as_mut().unwrap()
            }
        }
    }

    pub fn is_some(&self) -> bool {
        self.command_encoder.is_some()
    }

    pub fn create(&mut self, device: &wgpu::Device) {
        let command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.command_encoder = Some(command_encoder);
    }

    pub fn take(&mut self) -> Option<wgpu::CommandEncoder> {
        self.command_encoder.take()
    }

    pub fn set(&mut self, command_encoder: wgpu::CommandEncoder) {
        self.command_encoder = Some(command_encoder);
    }
}

#[derive(Debug)]
pub struct WgpuRenderContext {
    pub device: Arc<wgpu::Device>,
    pub command_encoder: LazyCommandEncoder,
    pub render_resource_context: WgpuRenderResourceContext,
}

impl WgpuRenderContext {
    pub fn new(device: Arc<wgpu::Device>, resources: WgpuRenderResourceContext) -> Self {
        WgpuRenderContext {
            device,
            render_resource_context: resources,
            command_encoder: LazyCommandEncoder::default(),
        }
    }

    /// Consume this context, finalize the current CommandEncoder (if it exists), and take the
    /// current WgpuResources. This is intended to be called from a worker thread right before
    /// synchronizing with the main thread.
    pub fn finish(&mut self) -> Option<wgpu::CommandBuffer> {
        self.command_encoder.take().map(|encoder| encoder.finish())
    }
}

impl RenderContext for WgpuRenderContext {
    fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: BufferId,
        source_offset: u64,
        destination_buffer: BufferId,
        destination_offset: u64,
        size: u64,
    ) {
        self.render_resource_context.copy_buffer_to_buffer(
            self.command_encoder.get_or_create(&self.device),
            source_buffer,
            source_offset,
            destination_buffer,
            destination_offset,
            size,
        );
    }

    fn copy_buffer_to_texture(
        &mut self,
        source_buffer: BufferId,
        source_offset: u64,
        source_bytes_per_row: u32,
        destination_texture: TextureId,
        destination_origin: [u32; 3],
        destination_mip_level: u32,
        size: Extent3d,
    ) {
        self.render_resource_context.copy_buffer_to_texture(
            self.command_encoder.get_or_create(&self.device),
            source_buffer,
            source_offset,
            source_bytes_per_row,
            destination_texture,
            destination_origin,
            destination_mip_level,
            size,
        )
    }

    fn copy_texture_to_buffer(
        &mut self,
        source_texture: TextureId,
        source_origin: [u32; 3],
        source_mip_level: u32,
        destination_buffer: BufferId,
        destination_offset: u64,
        destination_bytes_per_row: u32,
        size: Extent3d,
    ) {
        self.render_resource_context.copy_texture_to_buffer(
            self.command_encoder.get_or_create(&self.device),
            source_texture,
            source_origin,
            source_mip_level,
            destination_buffer,
            destination_offset,
            destination_bytes_per_row,
            size,
        )
    }

    fn copy_texture_to_texture(
        &mut self,
        source_texture: TextureId,
        source_origin: [u32; 3],
        source_mip_level: u32,
        destination_texture: TextureId,
        destination_origin: [u32; 3],
        destination_mip_level: u32,
        size: Extent3d,
    ) {
        self.render_resource_context.copy_texture_to_texture(
            self.command_encoder.get_or_create(&self.device),
            source_texture,
            source_origin,
            source_mip_level,
            destination_texture,
            destination_origin,
            destination_mip_level,
            size,
        )
    }

    fn resources(&self) -> &dyn RenderResourceContext {
        &self.render_resource_context
    }

    fn resources_mut(&mut self) -> &mut dyn RenderResourceContext {
        &mut self.render_resource_context
    }

    fn begin_render_pass(
        &mut self,
        pass_descriptor: &PassDescriptor,
        run_pass: &mut dyn FnMut(&mut dyn RenderPass),
    ) {
        if !self.command_encoder.is_some() {
            self.command_encoder.create(&self.device);
        }
        let resource_lock = self.render_resource_context.resources.read();
        let refs = resource_lock.refs();
        let mut encoder = self.command_encoder.take().unwrap();
        {
            let render_pass = create_render_pass(pass_descriptor, &refs, &mut encoder);
            let mut wgpu_render_pass = WgpuRenderPass {
                render_pass,
                render_context: self,
                wgpu_resources: refs,
                pipeline_descriptor: None,
            };

            run_pass(&mut wgpu_render_pass);
        }

        self.command_encoder.set(encoder);
    }

    fn begin_compute_pass(&mut self, run_pass: &mut dyn FnMut(&mut dyn ComputePass)) {
        if !self.command_encoder.is_some() {
            self.command_encoder.create(&self.device);
        }
        let resource_lock = self.render_resource_context.resources.read();
        let refs = resource_lock.refs();
        let mut encoder = self.command_encoder.take().unwrap();
        {
            let compute_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            let mut wgpu_render_pass = WgpuComputePass {
                compute_pass,
                render_context: self,
                wgpu_resources: refs,
                pipeline_descriptor: None,
            };

            run_pass(&mut wgpu_render_pass);
        }

        self.command_encoder.set(encoder);
    }
}

pub fn create_render_pass<'a, 'b>(
    pass_descriptor: &PassDescriptor,
    refs: &WgpuResourceRefs<'a>,
    encoder: &'a mut wgpu::CommandEncoder,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &pass_descriptor
            .color_attachments
            .iter()
            .map(|c| create_wgpu_color_attachment(refs, c))
            .collect::<Vec<wgpu::RenderPassColorAttachment>>(),
        depth_stencil_attachment: pass_descriptor
            .depth_stencil_attachment
            .as_ref()
            .map(|d| create_wgpu_depth_stencil_attachment(refs, d)),
    })
}

fn get_texture_view<'a>(
    refs: &WgpuResourceRefs<'a>,
    attachment: &TextureAttachment,
) -> &'a wgpu::TextureView {
    match attachment {
        TextureAttachment::Id(render_resource) => refs.texture_views.get(&render_resource).unwrap_or_else(|| &refs.swap_chain_frames.get(&render_resource).unwrap().output.view),
        TextureAttachment::Input(_) => panic!("Encountered unset `TextureAttachment::Input`. The `RenderGraph` executor should always set `TextureAttachment::Inputs` to `TextureAttachment::RenderResource` before running. This is a bug, please report it!"),
    }
}

fn create_wgpu_color_attachment<'a>(
    refs: &WgpuResourceRefs<'a>,
    color_attachment: &RenderPassColorAttachment,
) -> wgpu::RenderPassColorAttachment<'a> {
    let view = get_texture_view(refs, &color_attachment.attachment);

    let resolve_target = color_attachment
        .resolve_target
        .as_ref()
        .map(|target| get_texture_view(refs, &target));

    wgpu::RenderPassColorAttachment {
        ops: (&color_attachment.ops).wgpu_into(),
        view,
        resolve_target,
    }
}

fn create_wgpu_depth_stencil_attachment<'a>(
    refs: &WgpuResourceRefs<'a>,
    depth_stencil_attachment: &RenderPassDepthStencilAttachment,
) -> wgpu::RenderPassDepthStencilAttachment<'a> {
    let view = get_texture_view(refs, &depth_stencil_attachment.attachment);

    wgpu::RenderPassDepthStencilAttachment {
        view,
        depth_ops: depth_stencil_attachment
            .depth_ops
            .as_ref()
            .map(|ops| ops.wgpu_into()),
        stencil_ops: depth_stencil_attachment
            .stencil_ops
            .as_ref()
            .map(|ops| ops.wgpu_into()),
    }
}
