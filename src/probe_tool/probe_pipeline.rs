use crate::probe_tool::{
    frustum::{near_plane_interaction::FrustumNearPlaneIntersection, FrustumPlugin},
    kernel::{KernelDataResource, KernelHUDPlugin}, KernelSettings,
};

use bevy::math::FloatOrd;
use bevy::render::camera::ImageRenderTarget;
use bevy::{
    app::{App, Plugin},
    asset::{AssetServer, Assets, Handle, RenderAssetUsages, load_internal_asset, weak_handle},
    core_pipeline::core_3d::{
        Camera3d,
        graph::{Core3d, Node3d},
    },
    ecs::{
        component::Component,
        entity::Entity,
        query::{QueryData, QueryState, Without},
        system::{Commands, Query, Res, ResMut, SystemParamItem, lifetimeless::Read},
        world::{FromWorld, World},
    },
    log::info,
    math::{UVec2, Vec2, Vec4},
    prelude::*,
    prelude::{
        Added, AppExtStates, Camera, Image, IntoScheduleConfigs, Resource, Transform, Trigger,
        Update,
    },
    render::{
        Render, RenderApp, RenderSet,
        camera::{PerspectiveProjection, Projection, RenderTarget},
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        gpu_readback::{Readback, ReadbackComplete},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraphApp, RenderLabel},
        render_resource::{
            AsBindGroup, BindGroup, BindGroupLayout, BufferUsages, CachedComputePipelineId,
            ComputePassDescriptor, ComputePipelineDescriptor, Extent3d, PipelineCache, Shader,
            ShaderType, TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::{RenderContext, RenderDevice},
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
        texture::{FallbackImage, GpuImage},
        view::RenderLayers,
    },
    utils::default,
};

const PROBE_KERNEL_SMALL_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("5eb828ff-9ee5-4c25-a12a-886e2aeb096d");
const PROBE_KERNEL_MEDIUM_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("5db818ff-9ee5-4c25-a12a-886e2aeb096d");
const PROBE_KERNEL_LARGE_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("5db827ff-9ee5-4c25-a12a-886e2aeb096d");

/// This plugin provides the components and systems for GPU-based render target probing.
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

        app.add_plugins((
            ExtractComponentPlugin::<KernelSettings>::default(),
            ExtractComponentPlugin::<KernelBindGroup>::default(),
        ));

        app.add_systems(Update, (Self::sync_kernel_settings));
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<ProbePipeline>()
            .add_systems(
                Render,
                Self::prepare_probe_bind_groups.in_set(RenderSet::PrepareBindGroups),
            )
            .add_render_graph_node::<ProbeNode>(Core3d, ProbeNodeLabel)
            .add_render_graph_edge(Core3d, Node3d::EndMainPass, ProbeNodeLabel);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ProbeNodeLabel;

/// The render graph node that executes the probe compute shader.
struct ProbeNode {
    // We query for probes that are ready to be processed.
    query: QueryState<(Read<PreparedKernel>, Read<KernelSettings>)>,
}

impl FromWorld for ProbeNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query_filtered::<(Read<PreparedKernel>, Read<KernelSettings>), ()>(),
        }
    }
}



/// This component is created on the render world and holds the prepared `BindGroup`.
#[derive(Component)]
struct PreparedKernel(BindGroup);

/// This is the data that will be bound to the compute shader.
#[derive(Component, AsBindGroup, ExtractComponent, Clone, Default)]
pub struct KernelBindGroup {
    #[uniform(0)]
    pub settings: KernelSettings,
    #[texture(1, visibility(compute))]
    pub source_texture: Handle<Image>,
    #[storage(2, visibility(compute))]
    pub output_buffer: Handle<ShaderStorageBuffer>,
}

/// Caches the compute pipeline and bind group layout.
#[derive(Resource)]
struct ProbePipeline {
    layout: BindGroupLayout,
    small_pipeline: CachedComputePipelineId, // For kernels <= 4x4
    medium_pipeline: CachedComputePipelineId, // For kernels 5x5 to 16x16
    large_pipeline: CachedComputePipelineId, // For kernels > 16x16
}

impl FromWorld for ProbePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        // Create the layout from the `AsBindGroup` struct to ensure they match.
        let layout = KernelBindGroup::bind_group_layout(render_device);

        let pipeline_cache = world.resource::<PipelineCache>();
        let _asset_server = world.resource::<AssetServer>();

        let small_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            zero_initialize_workgroup_memory: false,
            label: Some("probe_pipeline_small_wesl".into()),
            layout: vec![layout.clone()],
            shader: PROBE_KERNEL_SMALL_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "main".into(),
            push_constant_ranges: vec![],
        });

        let medium_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            zero_initialize_workgroup_memory: false,
            label: Some("probe_pipeline_medium_wesl".into()),
            layout: vec![layout.clone()],
            shader: PROBE_KERNEL_MEDIUM_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "main".into(),
            push_constant_ranges: vec![],
        });

        let large_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            zero_initialize_workgroup_memory: false,
            label: Some("probe_pipeline_large_wesl".into()),
            layout: vec![layout.clone()],
            shader: PROBE_KERNEL_LARGE_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "main".into(),
            push_constant_ranges: vec![],
        });

        Self {
            layout,
            small_pipeline,
            medium_pipeline,
            large_pipeline,
        }
    }
}

impl ProbePipelinePlugin {
    /// Prepares the `BindGroup` for each probe on the render world.
    /// This runs in `RenderSet::PrepareBindGroups`, and Bevy's `AsBindGroup` infrastructure
    /// has already prepared the underlying buffers for us.
    fn prepare_probe_bind_groups(
        mut commands: Commands,
        pipeline: Res<ProbePipeline>,
        render_device: Res<RenderDevice>,
        mut system_params: SystemParamItem<(
            Res<RenderAssets<GpuImage>>,
            Res<FallbackImage>,
            Res<RenderAssets<GpuShaderStorageBuffer>>,
        )>,
        probe_query: Query<(Entity, &KernelBindGroup)>,
    ) {
        for (entity, bind_group_data) in probe_query.iter() {
            // Debug: Check if we can access the GPU texture
            if let Some(gpu_image) = system_params.0.get(&bind_group_data.source_texture) {
                info!(
                    "Preparing bind group for probe {:?} - GPU texture found with size: {:?}",
                    entity, gpu_image.size
                );
            } else {
                warn!(
                    "GPU texture not found for probe {:?} with handle {:?}",
                    entity, bind_group_data.source_texture
                );
            }

            let bind_group = match bind_group_data.as_bind_group(
                &pipeline.layout,
                &render_device,
                &mut system_params,
            ) {
                Ok(bind_group) => bind_group,
                Err(bevy::render::render_resource::AsBindGroupError::RetryNextUpdate) => {
                    // This is expected during startup when GPU resources aren't ready yet
                    continue;
                }
                Err(e) => {
                    warn!(
                        "Failed to create bind group for probe {:?}: {:?}",
                        entity, e
                    );
                    continue;
                }
            };
            commands
                .entity(entity)
                .insert(PreparedKernel(bind_group.bind_group));
        }
    }

    /// Syncs changes from the `KernelSettings` component to the `KernelBindGroup`'s `settings` field.
    fn sync_kernel_settings(
        mut query: Query<(&KernelSettings, &mut KernelBindGroup), Changed<KernelSettings>>,
    ) {
        for (settings, mut bind_group) in query.iter_mut() {
            if bind_group.settings.center_coords != settings.center_coords
                || bind_group.settings.kernel_size != settings.kernel_size
            {
                bind_group.settings = settings.clone();
                info!(
                    "Updated KernelBindGroup settings: center_coords={:?}",
                    settings.center_coords
                );
            }
        }
    }
}

/// The render graph node that executes the probe compute shader.
impl render_graph::Node for ProbeNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let probe_pipeline = world.resource::<ProbePipeline>();

        info!(
            "ProbeNode: Running compute shader for {} probes",
            self.query.iter_manual(world).count()
        );

        for (probe, settings) in self.query.iter_manual(world) {
            info!(
                "ProbeNode: Processing probe with center_coords: ({:.3}, {:.3})",
                settings.center_coords.x, settings.center_coords.y
            );

            // Choose pipeline based on kernel size
            let kernel_area = settings.kernel_size.x * settings.kernel_size.y;

            let (pipeline_id, dispatch_type) = if kernel_area <= 16.0 {
                // 4x4 or smaller - use small pipeline (1x1 workgroups)
                (probe_pipeline.small_pipeline, "small")
            } else if kernel_area <= 256.0 {
                // 5x5 to 16x16 - use medium pipeline (8x8 workgroups)
                (probe_pipeline.medium_pipeline, "medium")
            } else {
                // Larger than 16x16 - use large pipeline (16x16 workgroups)
                (probe_pipeline.large_pipeline, "large")
            };

            let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline_id) else {
                continue;
            };

            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("probe_compute_pass"),
                        timestamp_writes: None,
                    });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &probe.0, &[]);

            match dispatch_type {
                "small" => {
                    // Small pipeline uses 1x1 workgroups - dispatch one per pixel
                    pass.dispatch_workgroups(
                        settings.kernel_size.x as u32,
                        settings.kernel_size.y as u32,
                        1,
                    );
                }
                "medium" => {
                    // Medium pipeline uses 8x8 workgroups - calculate needed workgroups
                    let workgroups_x = (settings.kernel_size.x as u32 + 7) / 8;
                    let workgroups_y = (settings.kernel_size.y as u32 + 7) / 8;
                    pass.dispatch_workgroups(workgroups_x.max(1), workgroups_y.max(1), 1);
                }
                "large" => {
                    // Large pipeline uses 16x16 workgroups - calculate needed workgroups
                    let workgroups_x = (settings.kernel_size.x as u32 + 15) / 16;
                    let workgroups_y = (settings.kernel_size.y as u32 + 15) / 16;
                    pass.dispatch_workgroups(workgroups_x.max(1), workgroups_y.max(1), 1);
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}
