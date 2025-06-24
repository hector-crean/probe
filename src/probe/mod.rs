pub mod visualisation;use bevy::{
    ecs::system::ParamSet,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin}, gpu_readback::{GpuReadbackPlugin, Readback, ReadbackComplete}, render_asset::{RenderAssetUsages, RenderAssets}, render_graph::{self, RenderGraph, RenderLabel}, render_resource::{
            binding_types::{storage_buffer, texture_storage_2d, uniform_buffer}, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BufferUsages, CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor, Extent3d, PipelineCache, ShaderStages, ShaderType, StorageTextureAccess, TextureDimension, TextureFormat, TextureUsages
        }, renderer::{RenderContext, RenderDevice}, storage::{GpuShaderStorageBuffer, ShaderStorageBuffer}, Render, RenderApp
    },
};


pub struct ProbePlugin;

impl Plugin for ProbePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<Probe>::default(),
            UniformComponentPlugin::<Probe>::default(),
        ));

        app.add_systems(Startup, Probe::setup);
    }
    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ProbePipeline>().add_systems(
            Render,
            Probe::prepare_bind_group
                // We don't need to recreate the bind group every frame
        );

        // Add the compute node as a top level node to the render graph
        // This means it will only execute once per frame
        render_app
            .world_mut()
            .resource_mut::<RenderGraph>()
            .add_node(ProbeNodeLabel, ProbeNode::default());
    }
}

#[derive(Component, ExtractComponent, Clone, Default)]
pub struct ProbeReadbackBuffer(Handle<ShaderStorageBuffer>);

#[derive(Component, Default)]
struct ProbeReadbackBufferBindGroup(Option<BindGroup>);

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
#[require(Camera, ProbeReadbackBuffer, ProbeReadbackBufferBindGroup)]
pub struct Probe {
    // Size of the kernel in pixels
    pub kernel: Vec2,
    // Coordinates of pointer on the probe screen
    pub coords: Vec2,
}

impl Probe {
    fn kernel_size(&self) -> usize {
        self.kernel.x as usize * self.kernel.y as usize
    }
}

#[derive(Resource)]
pub struct ProbePipeline {
    layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

const SHADER_ASSET_PATH: &str = "shaders/probe_readback.wgsl";

impl FromWorld for ProbePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            None,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    // the probe struct
                    uniform_buffer::<Probe>(false),
                    // the kernel data around the mouse position sent back from the gpu
                    storage_buffer::<Vec<Vec4>>(false),
                    // the probe probe render target
                    texture_storage_2d(TextureFormat::Rgba8Unorm,StorageTextureAccess::ReadOnly),
                ),
            ),
        );
        let shader = world.load_asset(SHADER_ASSET_PATH);
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("GPU readback compute shader".into()),
            layout: vec![layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: Vec::new(),
            entry_point: "main".into(),
            zero_initialize_workgroup_memory: false,
        });
        Self { layout, pipeline }
    }
}

impl Probe {
    fn setup(
        mut commands: Commands,
        mut probe_query: Query<(Entity, &Probe), Added<Probe>>,
        mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    ) {
        for (entity, probe) in probe_query.iter() {
            // Create a storage buffer for our color data
            let buffer = vec![Vec4::ZERO; probe.kernel_size()];
            let mut buffer = ShaderStorageBuffer::from(buffer);
            // We need to enable the COPY_SRC usage so we can copy the buffer to the cpu
            buffer.buffer_description.usage |= BufferUsages::COPY_SRC;
            let buffer = buffers.add(buffer);

            commands
                .entity(entity)
                .insert(ProbeReadbackBuffer(buffer.clone()))
                .insert(Readback::buffer(buffer))
                .observe(|trigger: Trigger<ReadbackComplete>| {
                    // This matches the type which was used to create the `ShaderStorageBuffer` above,
                    // and is a convenient way to interpret the data.
                    let kernel_data: Vec<Vec4> = trigger.event().to_shader_type();
                    info!("Buffer {:?}", kernel_data);
                });
        }
    }
    fn prepare_bind_group(
        mut commands: Commands,
        pipeline: Res<ProbePipeline>,
        render_device: Res<RenderDevice>,
        buffers: Res<RenderAssets<GpuShaderStorageBuffer>>,
        probe_query: Query<(Entity, &ProbeReadbackBuffer, &Probe, &Camera), Without<ProbeReadbackBufferBindGroup>>,
    ) {
        for (entity, probe_readback_buffer, probe, camera) in probe_query.iter() {
            let buffer = buffers.get(&probe_readback_buffer.0).unwrap();

            let render_target = camera.target.as_image().unwrap();

            let bind_group = render_device.create_bind_group(
                None,
                &pipeline.layout,
                &BindGroupEntries::sequential((
                    probe.as_entire_uniform_buffer_binding(),
                    render_target,
                    buffer.buffer.as_entire_buffer_binding(),
                )),
            );
            commands
                .entity(entity)
                .insert(ProbeReadbackBufferBindGroup(Some(bind_group)));
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ProbeNodeLabel;

#[derive(Default)]
struct ProbeNode {}

impl render_graph::Node for ProbeNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<ProbePipeline>();

        let mut query = world.query::<(Entity, &Probe,&ProbeReadbackBuffer, &ProbeReadbackBufferBindGroup)>();

        let init_pipeline = if let Some(init_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) {
            init_pipeline
        } else {
            return Ok(());
        };

        for (entity, probe, probe_readback_buffer, probe_readback_buffer_bind_group) in query.iter(world) {
            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("GPU readback compute pass"),
                        ..default()
                    });

           
            pass.set_bind_group(0, &probe_readback_buffer_bind_group.0.unwrap(), &[]);
            pass.set_pipeline(init_pipeline);
            pass.dispatch_workgroups(probe.kernel_size() as u32, 1, 1);
        }
        
        Ok(())
    }
}
