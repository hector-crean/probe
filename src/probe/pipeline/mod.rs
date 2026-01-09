//! GPU pipeline for probe kernel computation.

mod bind_group;
mod node;

pub use bind_group::KernelBindGroup;
pub use node::ProbeNodeLabel;

use bevy::{
    app::{App, Plugin},
    asset::{load_internal_asset, uuid_handle, Handle},
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    prelude::*,
    render::{
        extract_component::ExtractComponentPlugin,
        render_graph::RenderGraphExt,
        Render, RenderApp, RenderSystems,
    },
};

use self::node::ProbeNode;

const PROBE_KERNEL_SMALL_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("5eb828ff-9ee5-4c25-a12a-886e2aeb096d");
const PROBE_KERNEL_MEDIUM_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("5db818ff-9ee5-4c25-a12a-886e2aeb096d");
const PROBE_KERNEL_LARGE_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("5db827ff-9ee5-4c25-a12a-886e2aeb096d");

/// Plugin for the probe GPU pipeline.
pub struct ProbePipelinePlugin;

impl Plugin for ProbePipelinePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PROBE_KERNEL_SMALL_SHADER_HANDLE,
            "shaders/kernel_small.wesl",
            Shader::from_wesl
        );

        load_internal_asset!(
            app,
            PROBE_KERNEL_MEDIUM_SHADER_HANDLE,
            "shaders/kernel_medium.wesl",
            Shader::from_wesl
        );

        load_internal_asset!(
            app,
            PROBE_KERNEL_LARGE_SHADER_HANDLE,
            "shaders/kernel_large.wesl",
            Shader::from_wesl
        );

        app.add_plugins(ExtractComponentPlugin::<KernelBindGroup>::default());
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<bind_group::ProbePipeline>()
            .add_systems(
                Render,
                bind_group::prepare_probe_bind_groups.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_render_graph_node::<ProbeNode>(Core3d, ProbeNodeLabel)
            .add_render_graph_edge(Core3d, Node3d::EndMainPass, ProbeNodeLabel);
    }
}
