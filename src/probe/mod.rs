pub mod visualisation;

use bevy::{
    app::{App, Plugin},
    asset::{AssetServer, Assets, Handle, load_internal_asset, weak_handle},
    core_pipeline::core_3d::graph::{Core3d, Node3d},
    ecs::{
        component::Component,
        entity::Entity,
        event::EventWriter,
        query::{QueryState, With, Without},
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut, SystemParamItem, lifetimeless::Read},
        world::{FromWorld, World},
    },
    log::info,
    math::{Vec2, Vec4},
    prelude::{Added, Camera, Color, Image, PluginGroup, Resource, Startup, Trigger, Update},
    render::{
        Render, RenderApp, RenderSet,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        gpu_readback::{GpuReadbackPlugin, Readback, ReadbackComplete},
        graph::CameraDriverLabel,
        render_asset::{RenderAsset, RenderAssets},
        render_graph::{self, RenderGraph, RenderGraphApp, RenderLabel},
        render_resource::{
            AsBindGroup, BindGroup, BindGroupEntries, BindGroupEntry, BindGroupLayout, Buffer,
            BufferUsages, CachedComputePipelineId, ComputePassDescriptor,
            ComputePipelineDescriptor, PipelineCache, Shader, ShaderRef, ShaderStages, ShaderType,
            StorageBuffer, StorageTextureAccess, TextureFormat, TextureUsages,
            TextureViewDimension, UniformBuffer,
        },
        renderer::{RenderContext, RenderDevice},
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
        texture::{FallbackImage, GpuImage},
    },
    utils::default,
};

/// This plugin provides the components and systems for GPU-based render target probing.
pub struct ProbePlugin;

impl Plugin for ProbePlugin {
    fn build(&self, app: &mut App) {
        // This asset is needed by the probe pipeline.
        load_internal_asset!(
            app,
            PROBE_SHADER_HANDLE,
            "probe_readback.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins((
            ExtractComponentPlugin::<ProbeSettings>::default(),
            ExtractComponentPlugin::<ProbeBindGroup>::default(),
        ))
        .add_systems(Update, Self::setup_probe_on_camera);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<ProbePipeline>()
            .add_systems(
                Render,
                (Self::prepare_probe_bind_groups)
                    .chain()
                    .in_set(RenderSet::PrepareBindGroups),
            )
            .add_render_graph_node::<ProbeNode>(Core3d, ProbeNodeLabel)
            .add_render_graph_edge(Core3d, Node3d::EndMainPass, ProbeNodeLabel);
    }
}

/// Handle to the compute shader used by the probe.
const PROBE_SHADER_HANDLE: Handle<Shader> = weak_handle!("5db828ff-9ee5-4c25-a12a-886e2aeb096d");

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ProbeNodeLabel;

/// The render graph node that executes the probe compute shader.
struct ProbeNode {
    // We query for probes that are ready to be processed.
    query: QueryState<(Read<PreparedProbe>, Read<ProbeSettings>)>,
}

impl FromWorld for ProbeNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query_filtered::<(Read<PreparedProbe>, Read<ProbeSettings>), ()>(),
        }
    }
}

/// A marker component for a 3D camera that should be probed.
#[derive(Component)]
pub struct Probe;

#[derive(Component, Clone, ExtractComponent, ShaderType)]
pub struct ProbeSettings {
    /// Size of the kernel in pixels.
    pub kernel_size: Vec2,
    /// Normalized coordinates of the probe center on the render target.
    pub center_coords: Vec2,
    // Note: Std140 layout requires fields to be 16-byte aligned.
    // The `Vec2`s are padded automatically by the `ShaderType` derive.
}

/// This component is created on the render world and holds the prepared `BindGroup`.
#[derive(Component)]
struct PreparedProbe(BindGroup);

/// This is the data that will be bound to the compute shader.
#[derive(Component, AsBindGroup, ExtractComponent, Clone)]
struct ProbeBindGroup {
    #[uniform(0)]
    settings: ProbeSettings,
    #[storage_texture(1, access = ReadOnly)]
    source_texture: Handle<Image>,
    #[storage(2, visibility(compute))]
    output_buffer: Handle<ShaderStorageBuffer>,
}

/// Caches the compute pipeline and bind group layout.
#[derive(Resource)]
struct ProbePipeline {
    layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

impl FromWorld for ProbePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        // Create the layout from the `AsBindGroup` struct to ensure they match.
        let layout = ProbeBindGroup::bind_group_layout(render_device);

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            zero_initialize_workgroup_memory: false,
            label: Some("probe_pipeline".into()),
            layout: vec![layout.clone()],
            shader: PROBE_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "main".into(),
            push_constant_ranges: vec![],
        });

        Self { layout, pipeline }
    }
}

impl ProbePlugin {
    /// Attaches the necessary probe components to any camera that has the `Probe` marker component.
    fn setup_probe_on_camera(
        mut commands: Commands,
        // This query runs for any camera that has our `Probe` marker but doesn't yet have `ProbeSettings`.
        camera_query: Query<(Entity, &Camera), (Added<Probe>, Without<ProbeSettings>)>,
        mut ssbo_assets: ResMut<Assets<ShaderStorageBuffer>>,
    ) {
        for (entity, camera) in camera_query.iter() {
            let Some(render_target) = camera.target.as_image().cloned() else {
                // This probe setup only works when rendering to a texture.
                // You could extend it to work with the primary window.
                continue;
            };

            let kernel_size = Vec2::new(16.0, 16.0);
            let settings = ProbeSettings {
                kernel_size,
                center_coords: Vec2::new(0.5, 0.5),
            };

            let buffer = vec![Vec4::ZERO; (kernel_size.x * kernel_size.y) as usize];
            let mut ssbo = ShaderStorageBuffer::from(buffer);
            ssbo.buffer_description.usage |=
                BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
            let ssbo_handle = ssbo_assets.add(ssbo);

            commands
                .entity(entity)
                .insert((
                    settings.clone(),
                    // The `ProbeBindGroup` contains all the data needed by the shader.
                    // Bevy will automatically extract this to the render world.
                    ProbeBindGroup {
                        settings,
                        source_texture: render_target,
                        output_buffer: ssbo_handle.clone(),
                    },
                    Readback::buffer(ssbo_handle),
                ))
                .observe(|trigger: Trigger<ReadbackComplete>| {
                    // This matches the type which was used to create the `ShaderStorageBuffer` above,
                    // and is a convenient way to interpret the data.
                    let data: Vec<Vec4> = trigger.event().to_shader_type();
                    info!("Buffer {:?}", data);
                });
        }
    }

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
        probe_query: Query<(Entity, &ProbeBindGroup)>,
    ) {
        for (entity, bind_group_data) in probe_query.iter() {
            let bind_group = bind_group_data
                .as_bind_group(&pipeline.layout, &render_device, &mut system_params)
                .unwrap();
            commands
                .entity(entity)
                .insert(PreparedProbe(bind_group.bind_group));
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

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(probe_pipeline.pipeline) else {
            return Ok(());
        };

        for (probe, settings) in self.query.iter_manual(world) {
            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("probe_compute_pass"),
                        timestamp_writes: None,
                    });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &probe.0, &[]);
            pass.dispatch_workgroups(
                settings.kernel_size.x as u32,
                settings.kernel_size.y as u32,
                1,
            );
        }

        Ok(())
    }
}
