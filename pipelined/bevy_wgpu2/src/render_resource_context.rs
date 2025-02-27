use crate::{
    resources::{WgpuBindGroupInfo, WgpuResources},
    type_converter::{OwnedWgpuVertexBufferLayout, WgpuInto},
};
use bevy_render2::{
    pipeline::{
        BindGroupDescriptor, BindGroupDescriptorId, BindingShaderStage, ComputePipelineDescriptor,
        PipelineId, RenderPipelineDescriptor,
    },
    render_resource::{
        BindGroup, BufferId, BufferInfo, BufferMapMode, RenderResourceBinding, SamplerId,
        SwapChainDescriptor, TextureId, TextureViewId,
    },
    renderer::RenderResourceContext,
    shader::{Shader, ShaderId},
    texture::{Extent3d, SamplerDescriptor, TextureDescriptor, TextureViewDescriptor},
};
use bevy_utils::tracing::trace;
use bevy_window::WindowId;
use futures_lite::future;
use std::{
    borrow::Cow,
    num::{NonZeroU32, NonZeroU64},
    ops::Range,
    sync::Arc,
};
use wgpu::util::DeviceExt;

#[derive(Clone, Debug)]
pub struct WgpuRenderResourceContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub resources: WgpuResources,
}

pub const COPY_BYTES_PER_ROW_ALIGNMENT: usize = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
pub const BIND_BUFFER_ALIGNMENT: usize = wgpu::BIND_BUFFER_ALIGNMENT as usize;
pub const COPY_BUFFER_ALIGNMENT: usize = wgpu::COPY_BUFFER_ALIGNMENT as usize;
pub const PUSH_CONSTANT_ALIGNMENT: u32 = wgpu::PUSH_CONSTANT_ALIGNMENT;

impl WgpuRenderResourceContext {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        WgpuRenderResourceContext {
            device,
            queue,
            resources: WgpuResources::default(),
        }
    }

    pub fn set_window_surface(&self, window_id: WindowId, surface: wgpu::Surface) {
        let mut window_surfaces = self.resources.window_surfaces.write();
        window_surfaces.insert(window_id, surface);
    }

    pub fn contains_window_surface(&self, window_id: WindowId) -> bool {
        self.resources
            .window_surfaces
            .read()
            .contains_key(&window_id)
    }

    pub fn copy_buffer_to_buffer(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        source_buffer: BufferId,
        source_offset: u64,
        destination_buffer: BufferId,
        destination_offset: u64,
        size: u64,
    ) {
        let buffers = self.resources.buffers.read();

        let source = buffers.get(&source_buffer).unwrap();
        let destination = buffers.get(&destination_buffer).unwrap();
        command_encoder.copy_buffer_to_buffer(
            source,
            source_offset,
            destination,
            destination_offset,
            size,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn copy_texture_to_texture(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        source_texture: TextureId,
        source_origin: [u32; 3], // TODO: replace with math type
        source_mip_level: u32,
        destination_texture: TextureId,
        destination_origin: [u32; 3], // TODO: replace with math type
        destination_mip_level: u32,
        size: Extent3d,
    ) {
        let textures = self.resources.textures.read();
        let source = textures.get(&source_texture).unwrap();
        let destination = textures.get(&destination_texture).unwrap();
        command_encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: source,
                mip_level: source_mip_level,
                origin: wgpu::Origin3d {
                    x: source_origin[0],
                    y: source_origin[1],
                    z: source_origin[2],
                },
            },
            wgpu::ImageCopyTexture {
                texture: destination,
                mip_level: destination_mip_level,
                origin: wgpu::Origin3d {
                    x: destination_origin[0],
                    y: destination_origin[1],
                    z: destination_origin[2],
                },
            },
            size.wgpu_into(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn copy_texture_to_buffer(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        source_texture: TextureId,
        source_origin: [u32; 3], // TODO: replace with math type
        source_mip_level: u32,
        destination_buffer: BufferId,
        destination_offset: u64,
        destination_bytes_per_row: u32,
        size: Extent3d,
    ) {
        let buffers = self.resources.buffers.read();
        let textures = self.resources.textures.read();

        let source = textures.get(&source_texture).unwrap();
        let destination = buffers.get(&destination_buffer).unwrap();
        command_encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: source,
                mip_level: source_mip_level,
                origin: wgpu::Origin3d {
                    x: source_origin[0],
                    y: source_origin[1],
                    z: source_origin[2],
                },
            },
            wgpu::ImageCopyBuffer {
                buffer: destination,
                layout: wgpu::ImageDataLayout {
                    offset: destination_offset,
                    bytes_per_row: NonZeroU32::new(destination_bytes_per_row),
                    rows_per_image: NonZeroU32::new(size.height),
                },
            },
            size.wgpu_into(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn copy_buffer_to_texture(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        source_buffer: BufferId,
        source_offset: u64,
        source_bytes_per_row: u32,
        destination_texture: TextureId,
        destination_origin: [u32; 3], // TODO: replace with math type
        destination_mip_level: u32,
        size: Extent3d,
    ) {
        let buffers = self.resources.buffers.read();
        let textures = self.resources.textures.read();

        let source = buffers.get(&source_buffer).unwrap();
        let destination = textures.get(&destination_texture).unwrap();
        command_encoder.copy_buffer_to_texture(
            wgpu::ImageCopyBuffer {
                buffer: source,
                layout: wgpu::ImageDataLayout {
                    offset: source_offset,
                    bytes_per_row: NonZeroU32::new(source_bytes_per_row),
                    rows_per_image: NonZeroU32::new(size.height),
                },
            },
            wgpu::ImageCopyTexture {
                texture: destination,
                mip_level: destination_mip_level,
                origin: wgpu::Origin3d {
                    x: destination_origin[0],
                    y: destination_origin[1],
                    z: destination_origin[2],
                },
            },
            size.wgpu_into(),
        );
    }

    pub fn create_bind_group_layout(&self, descriptor: &BindGroupDescriptor) {
        if self
            .resources
            .bind_group_layouts
            .read()
            .get(&descriptor.id)
            .is_some()
        {
            return;
        }

        let mut bind_group_layouts = self.resources.bind_group_layouts.write();
        // TODO: consider re-checking existence here
        let bind_group_layout_entries = descriptor
            .bindings
            .iter()
            .map(|binding| {
                let shader_stage = if binding.shader_stage
                    == BindingShaderStage::VERTEX | BindingShaderStage::FRAGMENT
                {
                    wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT
                } else if binding.shader_stage == BindingShaderStage::VERTEX {
                    wgpu::ShaderStage::VERTEX
                } else if binding.shader_stage == BindingShaderStage::FRAGMENT {
                    wgpu::ShaderStage::FRAGMENT
                } else if binding.shader_stage == BindingShaderStage::COMPUTE {
                    wgpu::ShaderStage::COMPUTE
                } else {
                    panic!("Invalid binding shader stage.")
                };
                wgpu::BindGroupLayoutEntry {
                    binding: binding.index,
                    visibility: shader_stage,
                    ty: (&binding.bind_type).wgpu_into(),
                    count: binding.count,
                }
            })
            .collect::<Vec<wgpu::BindGroupLayoutEntry>>();
        let wgpu_descriptor = wgpu::BindGroupLayoutDescriptor {
            entries: bind_group_layout_entries.as_slice(),
            label: None,
        };
        let bind_group_layout = self.device.create_bind_group_layout(&wgpu_descriptor);
        bind_group_layouts.insert(descriptor.id, bind_group_layout);
    }

    fn try_next_swap_chain_texture(
        &self,
        window_id: bevy_window::WindowId,
    ) -> Option<TextureViewId> {
        let mut window_swap_chains = self.resources.window_swap_chains.write();
        let mut swap_chain_outputs = self.resources.swap_chain_frames.write();

        let window_swap_chain = window_swap_chains.get_mut(&window_id)?;
        let next_texture = window_swap_chain.get_current_frame().ok()?;
        let id = TextureViewId::new();
        swap_chain_outputs.insert(id, next_texture);
        Some(id)
    }
}

impl RenderResourceContext for WgpuRenderResourceContext {
    fn create_sampler(&self, sampler_descriptor: &SamplerDescriptor) -> SamplerId {
        let mut samplers = self.resources.samplers.write();

        let descriptor: wgpu::SamplerDescriptor = (*sampler_descriptor).wgpu_into();
        let sampler = self.device.create_sampler(&descriptor);

        let id = SamplerId::new();
        samplers.insert(id, sampler);
        id
    }

    fn create_texture(&self, texture_descriptor: TextureDescriptor) -> TextureId {
        let mut textures = self.resources.textures.write();
        let mut texture_descriptors = self.resources.texture_descriptors.write();

        let descriptor: wgpu::TextureDescriptor = (&texture_descriptor).wgpu_into();
        let texture = self.device.create_texture(&descriptor);

        let id = TextureId::new();
        texture_descriptors.insert(id, texture_descriptor);
        textures.insert(id, texture);
        id
    }

    fn create_texture_view(
        &self,
        texture_id: TextureId,
        texture_view_descriptor: TextureViewDescriptor,
    ) -> TextureViewId {
        let mut texture_views = self.resources.texture_views.write();
        let textures = self.resources.textures.read();
        let texture = textures.get(&texture_id).unwrap();
        let descriptor: wgpu::TextureViewDescriptor = texture_view_descriptor.wgpu_into();
        let texture_view = texture.create_view(&descriptor);
        let id = TextureViewId::new();
        texture_views.insert(id, texture_view);
        id
    }

    fn create_buffer(&self, buffer_info: BufferInfo) -> BufferId {
        // TODO: consider moving this below "create" for efficiency
        let mut buffer_infos = self.resources.buffer_infos.write();
        let mut buffers = self.resources.buffers.write();

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_info.size as u64,
            usage: buffer_info.buffer_usage.wgpu_into(),
            mapped_at_creation: buffer_info.mapped_at_creation,
        });

        let id = BufferId::new();
        buffer_infos.insert(id, buffer_info);
        buffers.insert(id, Arc::new(buffer));
        id
    }

    fn create_buffer_with_data(&self, mut buffer_info: BufferInfo, data: &[u8]) -> BufferId {
        // TODO: consider moving this below "create" for efficiency
        let mut buffer_infos = self.resources.buffer_infos.write();
        let mut buffers = self.resources.buffers.write();

        buffer_info.size = data.len();
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: data,
                label: None,
                usage: buffer_info.buffer_usage.wgpu_into(),
            });

        let id = BufferId::new();
        buffer_infos.insert(id, buffer_info);
        buffers.insert(id, Arc::new(buffer));
        id
    }

    fn remove_buffer(&self, buffer: BufferId) {
        let mut buffer_infos = self.resources.buffer_infos.write();
        let mut buffers = self.resources.buffers.write();

        buffers.remove(&buffer);
        buffer_infos.remove(&buffer);
    }

    fn remove_texture(&self, texture: TextureId) {
        let mut textures = self.resources.textures.write();
        let mut texture_descriptors = self.resources.texture_descriptors.write();

        textures.remove(&texture);
        texture_descriptors.remove(&texture);
    }

    fn remove_texture_view(&self, texture_view: TextureViewId) {
        let mut texture_views = self.resources.texture_views.write();
        texture_views.remove(&texture_view);
    }

    fn remove_sampler(&self, sampler: SamplerId) {
        let mut samplers = self.resources.samplers.write();
        samplers.remove(&sampler);
    }

    fn create_shader_module(&self, shader: &Shader) -> ShaderId {
        let mut shader_modules = self.resources.shader_modules.write();
        let spirv: Cow<[u32]> = shader.get_spirv(None).unwrap().into();
        let shader_module = self
            .device
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::SpirV(spirv),
                flags: Default::default(),
            });
        let id = ShaderId::new();
        shader_modules.insert(id, shader_module);
        id
    }

    fn next_swap_chain_texture(&self, descriptor: &SwapChainDescriptor) -> TextureViewId {
        if let Some(texture_id) = self.try_next_swap_chain_texture(descriptor.window_id) {
            texture_id
        } else {
            {
                let surfaces = self.resources.window_surfaces.read();
                let swap_chain_descriptor: wgpu::SwapChainDescriptor = descriptor.wgpu_into();
                let mut window_swap_chains = self.resources.window_swap_chains.write();
                let surface = surfaces
                    .get(&descriptor.window_id)
                    .expect("No surface found for window.");
                let swap_chain = self
                    .device
                    .create_swap_chain(surface, &swap_chain_descriptor);

                window_swap_chains.insert(descriptor.window_id, swap_chain);
            }
            self.try_next_swap_chain_texture(descriptor.window_id)
                .expect("Failed to acquire next swap chain texture!")
        }
    }

    fn drop_swap_chain_texture(&self, texture: TextureViewId) {
        let mut swap_chain_outputs = self.resources.swap_chain_frames.write();
        swap_chain_outputs.remove(&texture);
    }

    fn drop_all_swap_chain_textures(&self) {
        let mut swap_chain_outputs = self.resources.swap_chain_frames.write();
        swap_chain_outputs.clear();
    }

    fn create_render_pipeline(&self, pipeline_descriptor: &RenderPipelineDescriptor) -> PipelineId {
        let layout = &pipeline_descriptor.layout;
        for bind_group_descriptor in layout.bind_groups.iter() {
            self.create_bind_group_layout(&bind_group_descriptor);
        }

        let bind_group_layouts = self.resources.bind_group_layouts.read();
        // setup and collect bind group layouts
        let bind_group_layouts = layout
            .bind_groups
            .iter()
            .map(|bind_group| bind_group_layouts.get(&bind_group.id).unwrap())
            .collect::<Vec<&wgpu::BindGroupLayout>>();

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: bind_group_layouts.as_slice(),
                push_constant_ranges: &[],
            });

        let owned_vertex_buffer_descriptors = layout
            .vertex_buffer_descriptors
            .iter()
            .map(|v| v.wgpu_into())
            .collect::<Vec<OwnedWgpuVertexBufferLayout>>();

        let color_states = pipeline_descriptor
            .color_target_states
            .iter()
            .map(|c| c.wgpu_into())
            .collect::<Vec<wgpu::ColorTargetState>>();

        let shader_modules = self.resources.shader_modules.read();
        let vertex_shader_module = shader_modules
            .get(&pipeline_descriptor.shader_stages.vertex)
            .unwrap();

        let fragment_shader_module = pipeline_descriptor
            .shader_stages
            .fragment
            .as_ref()
            .map(|fragment_handle| shader_modules.get(fragment_handle).unwrap());
        let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader_module,
                entry_point: "main",
                buffers: &owned_vertex_buffer_descriptors
                    .iter()
                    .map(|v| v.into())
                    .collect::<Vec<wgpu::VertexBufferLayout>>(),
            },
            fragment: pipeline_descriptor
                .shader_stages
                .fragment
                .as_ref()
                .map(|_| wgpu::FragmentState {
                    entry_point: "main",
                    module: fragment_shader_module.as_ref().unwrap(),
                    targets: color_states.as_slice(),
                }),
            primitive: pipeline_descriptor.primitive.clone().wgpu_into(),
            depth_stencil: pipeline_descriptor
                .depth_stencil
                .clone()
                .map(|depth_stencil| depth_stencil.wgpu_into()),
            multisample: pipeline_descriptor.multisample.clone().wgpu_into(),
        };

        let render_pipeline = self
            .device
            .create_render_pipeline(&render_pipeline_descriptor);
        let mut render_pipelines = self.resources.render_pipelines.write();
        let id = PipelineId::new();
        render_pipelines.insert(id, render_pipeline);
        id
    }

    fn create_compute_pipeline(
        &self,
        pipeline_descriptor: &ComputePipelineDescriptor,
    ) -> PipelineId {
        let layout = &pipeline_descriptor.layout;
        for bind_group_descriptor in layout.bind_groups.iter() {
            self.create_bind_group_layout(&bind_group_descriptor);
        }

        let bind_group_layouts = self.resources.bind_group_layouts.read();
        // setup and collect bind group layouts
        let bind_group_layouts = layout
            .bind_groups
            .iter()
            .map(|bind_group| bind_group_layouts.get(&bind_group.id).unwrap())
            .collect::<Vec<&wgpu::BindGroupLayout>>();

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: bind_group_layouts.as_slice(),
                push_constant_ranges: &[],
            });

        let shader_modules = self.resources.shader_modules.read();
        let compute_shader_module = shader_modules
            .get(&pipeline_descriptor.shader_stages.compute)
            .unwrap();

        let compute_pipeline_descriptor = wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            entry_point: "main",
            module: compute_shader_module,
        };

        let compute_pipeline = self
            .device
            .create_compute_pipeline(&compute_pipeline_descriptor);
        let mut compute_pipelines = self.resources.compute_pipelines.write();
        let id = PipelineId::new();
        compute_pipelines.insert(id, compute_pipeline);
        id
    }

    fn bind_group_descriptor_exists(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
    ) -> bool {
        let bind_group_layouts = self.resources.bind_group_layouts.read();
        bind_group_layouts.get(&bind_group_descriptor_id).is_some()
    }

    fn create_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        bind_group: &BindGroup,
    ) {
        if !self
            .resources
            .has_bind_group(bind_group_descriptor_id, bind_group.id)
        {
            trace!(
                "start creating bind group for RenderResourceSet {:?}",
                bind_group.id
            );
            let texture_views = self.resources.texture_views.read();
            let samplers = self.resources.samplers.read();
            let buffers = self.resources.buffers.read();
            let bind_group_layouts = self.resources.bind_group_layouts.read();
            let mut bind_groups = self.resources.bind_groups.write();

            let mut texture_arrays = Vec::new();

            for indexed_binding in &*bind_group.indexed_bindings {
                match &indexed_binding.entry {
                    RenderResourceBinding::TextureArrayView(resources) => {
                        let texture_views: Vec<_> = resources
                            .iter()
                            .map(|resource| {
                                texture_views
                                    .get(&resource)
                                    .unwrap_or_else(|| panic!("{:?}", resource))
                            })
                            .collect();
                        texture_arrays.push(texture_views);
                    }
                    _ => {}
                }
            }

            let mut texture_array_ix = 0;
            let entries = bind_group
                .indexed_bindings
                .iter()
                .map(|indexed_binding| {
                    let wgpu_resource = match &indexed_binding.entry {
                        RenderResourceBinding::TextureView(resource) => {
                            let texture_view = texture_views
                                .get(&resource)
                                .unwrap_or_else(|| panic!("{:?}", resource));
                            wgpu::BindingResource::TextureView(texture_view)
                        }
                        RenderResourceBinding::TextureArrayView(_) => {
                            let texture_array = texture_arrays[texture_array_ix].as_slice();
                            texture_array_ix += 1;
                            wgpu::BindingResource::TextureViewArray(texture_array)
                        }
                        RenderResourceBinding::Sampler(resource) => {
                            let sampler = samplers.get(&resource).unwrap();
                            wgpu::BindingResource::Sampler(sampler)
                        }
                        RenderResourceBinding::Buffer { buffer, range, .. } => {
                            let wgpu_buffer = buffers.get(&buffer).unwrap();
                            let size = NonZeroU64::new(range.end - range.start)
                                .expect("Size of the buffer needs to be greater than 0!");
                            wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: wgpu_buffer,
                                offset: range.start,
                                size: Some(size),
                            })
                        }
                    };
                    wgpu::BindGroupEntry {
                        binding: indexed_binding.index,
                        resource: wgpu_resource,
                    }
                })
                .collect::<Vec<wgpu::BindGroupEntry>>();

            let bind_group_layout = bind_group_layouts.get(&bind_group_descriptor_id).unwrap();
            let wgpu_bind_group_descriptor = wgpu::BindGroupDescriptor {
                label: None,
                layout: bind_group_layout,
                entries: entries.as_slice(),
            };
            let wgpu_bind_group = self.device.create_bind_group(&wgpu_bind_group_descriptor);

            let bind_group_info = bind_groups
                .entry(bind_group_descriptor_id)
                .or_insert_with(WgpuBindGroupInfo::default);
            bind_group_info
                .bind_groups
                .insert(bind_group.id, wgpu_bind_group);
            trace!(
                "created bind group for RenderResourceSet {:?}",
                bind_group.id
            );
        }
    }

    fn clear_bind_groups(&self) {
        self.resources.bind_groups.write().clear();
    }

    fn remove_stale_bind_groups(&self) {
        self.resources.remove_stale_bind_groups();
    }

    fn get_buffer_info(&self, buffer: BufferId) -> Option<BufferInfo> {
        self.resources.buffer_infos.read().get(&buffer).cloned()
    }

    fn write_mapped_buffer(
        &self,
        id: BufferId,
        range: Range<u64>,
        write: &mut dyn FnMut(&mut [u8], &dyn RenderResourceContext),
    ) {
        let buffer = {
            let buffers = self.resources.buffers.read();
            buffers.get(&id).unwrap().clone()
        };
        let buffer_slice = buffer.slice(range);
        let mut data = buffer_slice.get_mapped_range_mut();
        write(&mut data, self);
    }

    fn read_mapped_buffer(
        &self,
        id: BufferId,
        range: Range<u64>,
        read: &dyn Fn(&[u8], &dyn RenderResourceContext),
    ) {
        let buffer = {
            let buffers = self.resources.buffers.read();
            buffers.get(&id).unwrap().clone()
        };
        let buffer_slice = buffer.slice(range);
        let data = buffer_slice.get_mapped_range();
        read(&data, self);
    }

    fn map_buffer(&self, id: BufferId, mode: BufferMapMode) {
        let buffers = self.resources.buffers.read();
        let buffer = buffers.get(&id).unwrap();
        let buffer_slice = buffer.slice(..);
        let wgpu_mode = match mode {
            BufferMapMode::Read => wgpu::MapMode::Read,
            BufferMapMode::Write => wgpu::MapMode::Write,
        };
        let data = buffer_slice.map_async(wgpu_mode);
        self.device.poll(wgpu::Maintain::Wait);
        if future::block_on(data).is_err() {
            panic!("Failed to map buffer to host.");
        }
    }

    fn unmap_buffer(&self, id: BufferId) {
        let buffers = self.resources.buffers.read();
        let buffer = buffers.get(&id).unwrap();
        buffer.unmap();
    }

    fn get_aligned_texture_size(&self, size: usize) -> usize {
        (size + COPY_BYTES_PER_ROW_ALIGNMENT - 1) & !(COPY_BYTES_PER_ROW_ALIGNMENT - 1)
    }

    fn get_aligned_uniform_size(&self, size: usize, dynamic: bool) -> usize {
        if dynamic {
            (size + BIND_BUFFER_ALIGNMENT - 1) & !(BIND_BUFFER_ALIGNMENT - 1)
        } else {
            size
        }
    }
}
