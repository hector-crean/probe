//! Bind group and pipeline resources for probe computation.

use bevy::{
    asset::Handle,
    ecs::{
        component::Component,
        entity::Entity,
        system::{Commands, Query, Res, SystemParamItem},
        world::{FromWorld, World},
    },
    log::warn,
    prelude::*,
    render::{
        extract_component::ExtractComponent,
        render_asset::RenderAssets,
        render_resource::{
            AsBindGroup, BindGroup, BindGroupLayout, CachedComputePipelineId,
            ComputePipelineDescriptor, PipelineCache,
        },
        renderer::RenderDevice,
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
        texture::{FallbackImage, GpuImage},
    },
};

use crate::probe::components::KernelSettings;

use super::{
    PROBE_KERNEL_LARGE_SHADER_HANDLE, PROBE_KERNEL_MEDIUM_SHADER_HANDLE,
    PROBE_KERNEL_SMALL_SHADER_HANDLE,
};

/// This component is created on the render world and holds the prepared `BindGroup`.
#[derive(Component)]
pub struct PreparedKernel(pub BindGroup);

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
pub struct ProbePipeline {
    pub layout: BindGroupLayout,
    pub small_pipeline: CachedComputePipelineId, // For kernels <= 4x4
    pub medium_pipeline: CachedComputePipelineId, // For kernels 5x5 to 16x16
    pub large_pipeline: CachedComputePipelineId, // For kernels > 16x16
}

impl FromWorld for ProbePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        // Create the layout from the `AsBindGroup` struct to ensure they match.
        let layout = KernelBindGroup::bind_group_layout(render_device);

        let pipeline_cache = world.resource::<PipelineCache>();

        let small_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            zero_initialize_workgroup_memory: false,
            label: Some("probe_pipeline_small_wesl".into()),
            layout: vec![layout.clone()],
            shader: PROBE_KERNEL_SMALL_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: Some("main".into()),
            push_constant_ranges: vec![],
        });

        let medium_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            zero_initialize_workgroup_memory: false,
            label: Some("probe_pipeline_medium_wesl".into()),
            layout: vec![layout.clone()],
            shader: PROBE_KERNEL_MEDIUM_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: Some("main".into()),
            push_constant_ranges: vec![],
        });

        let large_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            zero_initialize_workgroup_memory: false,
            label: Some("probe_pipeline_large_wesl".into()),
            layout: vec![layout.clone()],
            shader: PROBE_KERNEL_LARGE_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: Some("main".into()),
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

/// Prepares the `BindGroup` for each probe on the render world.
/// This runs in `RenderSystems::PrepareBindGroups`, and Bevy's `AsBindGroup` infrastructure
/// has already prepared the underlying buffers for us.
pub fn prepare_probe_bind_groups(
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
            debug!(
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
