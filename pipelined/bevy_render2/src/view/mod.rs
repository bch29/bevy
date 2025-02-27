pub mod window;

use bevy_transform::components::GlobalTransform;
pub use window::*;

use crate::{
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext},
    render_resource::DynamicUniformVec,
    renderer::{RenderContext, RenderResources},
    RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, Vec3};
use crevice::std140::AsStd140;

pub struct ViewPlugin;

impl ViewPlugin {
    pub const VIEW_NODE: &'static str = "view";
}

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(0);
        render_app
            .init_resource::<ViewMeta>()
            .add_system_to_stage(RenderStage::Prepare, prepare_views.system());

        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node(ViewPlugin::VIEW_NODE, ViewNode);
    }
}

pub struct ExtractedView {
    pub projection: Mat4,
    pub transform: GlobalTransform,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, AsStd140)]
pub struct ViewUniformData {
    view_proj: Mat4,
    world_position: Vec3,
}

#[derive(Default)]
pub struct ViewMeta {
    pub uniforms: DynamicUniformVec<ViewUniformData>,
}

pub struct ViewUniform {
    pub view_uniform_offset: u32,
}

fn prepare_views(
    mut commands: Commands,
    render_resources: Res<RenderResources>,
    mut view_meta: ResMut<ViewMeta>,
    mut extracted_views: Query<(Entity, &ExtractedView)>,
) {
    view_meta
        .uniforms
        .reserve_and_clear(extracted_views.iter_mut().len(), &render_resources);
    for (entity, camera) in extracted_views.iter() {
        let view_uniforms = ViewUniform {
            view_uniform_offset: view_meta.uniforms.push(ViewUniformData {
                view_proj: camera.projection * camera.transform.compute_matrix().inverse(),
                world_position: camera.transform.translation,
            }),
        };

        commands.entity(entity).insert(view_uniforms);
    }

    view_meta
        .uniforms
        .write_to_staging_buffer(&render_resources);
}

pub struct ViewNode;

impl Node for ViewNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut dyn RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_meta = world.get_resource::<ViewMeta>().unwrap();
        view_meta.uniforms.write_to_uniform_buffer(render_context);
        Ok(())
    }
}
