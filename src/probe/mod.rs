pub mod visualisation;

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
        event::EventWriter,
        query::{QueryState, With, Without},
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut, SystemParamItem, lifetimeless::Read},
        world::{FromWorld, World},
    },
    log::info,
    math::{UVec2, Vec2, Vec4},
    prelude::{
        Added, Camera, Children, Color, Image, PluginGroup, Resource, Startup, Transform, Trigger, Update,
    },
    ui::prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        camera::{PerspectiveProjection, Projection, RenderTarget},
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        gpu_readback::{GpuReadbackPlugin, Readback, ReadbackComplete},
        graph::CameraDriverLabel,
        render_asset::{RenderAsset, RenderAssets},
        render_graph::{self, RenderGraph, RenderGraphApp, RenderLabel},
        render_resource::{
            AsBindGroup, BindGroup, BindGroupEntries, BindGroupEntry, BindGroupLayout, Buffer,
            BufferUsages, CachedComputePipelineId, ComputePassDescriptor,
            ComputePipelineDescriptor, Extent3d, PipelineCache, Shader, ShaderRef, ShaderSource,
            ShaderStages, ShaderType, StorageBuffer, StorageTextureAccess, TextureDimension,
            TextureFormat, TextureUsages, TextureViewDimension, UniformBuffer,
        },
        renderer::{RenderContext, RenderDevice},
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
        texture::{FallbackImage, GpuImage},
        view::RenderLayers,
    },
    utils::default,
};

const PROBE_KERNEL_SMALL_SHADER_HANDLE: Handle<Shader> = weak_handle!("5eb828ff-9ee5-4c25-a12a-886e2aeb096d");
const PROBE_KERNEL_MEDIUM_SHADER_HANDLE: Handle<Shader> = weak_handle!("5db818ff-9ee5-4c25-a12a-886e2aeb096d");
const PROBE_KERNEL_LARGE_SHADER_HANDLE: Handle<Shader> = weak_handle!("5db827ff-9ee5-4c25-a12a-886e2aeb096d");

/// Resource to store the current kernel data for UI visualization
#[derive(Resource, Default)]
pub struct KernelDataResource {
    pub data: Vec<Vec4>,
    pub kernel_size: Vec2,
}

/// Marker component for the kernel visualization UI
#[derive(Component)]
pub struct KernelVisualizationUI;

/// Marker component for the kernel grid container
#[derive(Component)]
pub struct KernelGrid;


/// This plugin provides the components and systems for GPU-based render target probing.
pub struct ProbePlugin;

impl Plugin for ProbePlugin {
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
            ExtractComponentPlugin::<ProbeSettings>::default(),
            ExtractComponentPlugin::<ProbeBindGroup>::default(),
        ))
        .init_resource::<KernelDataResource>()
        .add_systems(Startup, Self::setup_kernel_visualization_ui)
        .add_systems(Update, (Self::setup_probe_on_camera, Self::update_kernel_visualization_ui));
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

/// A component for a 3D camera that should be probed.
#[derive(Component)]
#[require(Transform, Camera, Camera3d, Projection)]
pub struct Probe {
    pub resolution: UVec2,
}
impl Default for Probe {
    fn default() -> Self {
        Self {
            resolution: UVec2::new(512, 512),
        }
    }
}

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
pub struct ProbeBindGroup {
    #[uniform(0)]
    settings: ProbeSettings,
    #[texture(1, visibility(compute))]
    source_texture: Handle<Image>,
    #[storage(2, visibility(compute))]
    output_buffer: Handle<ShaderStorageBuffer>,
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
        let layout = ProbeBindGroup::bind_group_layout(render_device);

        let pipeline_cache = world.resource::<PipelineCache>();
        let asset_server = world.resource::<AssetServer>();

      

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

impl ProbePlugin {
    /// Attaches the necessary probe components to any camera that has the `Probe` marker component.
    fn setup_probe_on_camera(
        mut commands: Commands,
        // This query runs for any camera that has our `Probe` marker but doesn't yet have `ProbeSettings`.
        camera_query: Query<(Entity, &Probe), (Added<Probe>, Without<ProbeSettings>)>,
        mut ssbo_assets: ResMut<Assets<ShaderStorageBuffer>>,
        mut images: ResMut<Assets<Image>>,
    ) {
        // This specifies the layer used for the probe pass.
        // let probe_layer = RenderLayers::layer(1);

        for (entity, probe) in camera_query.iter() {
            let aspect_ratio = probe.resolution.x as f32 / probe.resolution.y as f32;

            let size = Extent3d {
                width: probe.resolution.x,
                height: probe.resolution.y,
                ..default()
            };

            // This is the texture that will be rendered to.
            let mut image = Image::new_fill(
                size,
                TextureDimension::D2,
                &[0, 0, 0, 0],
                TextureFormat::Bgra8UnormSrgb,
                RenderAssetUsages::default(),
            );
            // You need to set ALL the usage flags for how the image will be used.
            image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT;

            let image_handle = images.add(image);

            let kernel_size = Vec2::new(3., 3.);
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
                    Camera {
                        // Render to our texture instead of the main window
                        target: RenderTarget::Image(image_handle.clone().into()),
                        ..default()
                    },
                    Camera3d::default(),
                    // probe_layer.clone(),
                    // The `ProbeBindGroup` contains all the data needed by the shader.
                    // Bevy will automatically extract this to the render world.
                    Projection::Perspective(PerspectiveProjection {
                        near: 2.0,
                        far: 4.0,
                        fov: std::f32::consts::PI / 3.0, // 60 degrees
                        aspect_ratio,                    // Explicitly set the aspect ratio
                    }),
                    ProbeBindGroup {
                        settings,
                        source_texture: image_handle,
                        output_buffer: ssbo_handle.clone(),
                    },
                    Readback::buffer(ssbo_handle),
                ))
                .observe(move |trigger: Trigger<ReadbackComplete>, mut kernel_data: ResMut<KernelDataResource>| {
                    // This matches the type which was used to create the `ShaderStorageBuffer` above,
                    // and is a convenient way to interpret the data.
                    let kernel: Vec<Vec4> = trigger.event().to_shader_type();
                    
                    // Update the kernel data resource for UI visualization
                    kernel_data.data = kernel;
                    kernel_data.kernel_size = kernel_size; // Use the actual kernel size
                    
                    // info!("Buffer {:?}", kernel_data.data);
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

    /// Sets up the kernel visualization UI
    fn setup_kernel_visualization_ui(mut commands: Commands) {
        // Create a UI panel to show kernel data as colored grid
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(10.0),
                    right: Val::Px(10.0),
                    width: Val::Px(200.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                KernelVisualizationUI,
            ))
            .with_children(|parent| {
                // Title
                parent.spawn(Text::new("Kernel Data"));
                
                // Grid container that will hold the colored squares
                parent.spawn((
                    Node {
                        display: Display::Grid,
                        grid_template_columns: vec![GridTrack::px(30.0); 3], // 3x3 grid initially
                        grid_template_rows: vec![GridTrack::px(30.0); 3],
                        column_gap: Val::Px(2.0),
                        row_gap: Val::Px(2.0),
                        margin: UiRect::top(Val::Px(10.0)),
                        ..default()
                    },
                    KernelGrid,
                ));
            });
    }

    /// Updates the kernel visualization UI with current data
    fn update_kernel_visualization_ui(
        mut commands: Commands,
        grid_query: Query<Entity, With<KernelGrid>>,
        kernel_data: Res<KernelDataResource>,
        children_query: Query<&Children>,
        mut background_query: Query<&mut BackgroundColor>,
        mut text_query: Query<&mut Text>,
    ) {
        // Only update if kernel data is not empty
        if kernel_data.data.is_empty() {
            return;
        }

        for grid_entity in grid_query.iter() {
            // Check if grid already has children (squares)
            if let Ok(children) = children_query.get(grid_entity) {
                if children.len() != kernel_data.data.len() {
                    // Grid size changed, need to rebuild
                    for &child in children.iter() {
                        commands.entity(child).despawn_recursive();
                    }
                    Self::create_kernel_grid(&mut commands, grid_entity, &kernel_data);
                } else {
                    // Update existing squares
                    for (index, &child) in children.iter().enumerate() {
                        if index < kernel_data.data.len() {
                            let pixel = &kernel_data.data[index];
                            let color = Color::srgba(
                                pixel.x.clamp(0.0, 1.0),
                                pixel.y.clamp(0.0, 1.0), 
                                pixel.z.clamp(0.0, 1.0),
                                1.0,
                            );
                            
                            if let Ok(mut bg_color) = background_query.get_mut(child) {
                                *bg_color = BackgroundColor(color);
                            }
                            
                            // Update text in grandchildren if exists
                            if let Ok(child_children) = children_query.get(child) {
                                for &grandchild in child_children.iter() {
                                    if let Ok(mut text) = text_query.get_mut(grandchild) {
                                        text.0 = format!("{:.1}", pixel.x);
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // No children yet, create initial grid
                Self::create_kernel_grid(&mut commands, grid_entity, &kernel_data);
            }
        }
    }

    /// Helper function to create the kernel grid
    fn create_kernel_grid(
        commands: &mut Commands,
        grid_entity: Entity,
        kernel_data: &KernelDataResource,
    ) {
        commands.entity(grid_entity).with_children(|parent| {
            for (index, pixel) in kernel_data.data.iter().enumerate() {
                let color = Color::srgba(
                    pixel.x.clamp(0.0, 1.0),
                    pixel.y.clamp(0.0, 1.0), 
                    pixel.z.clamp(0.0, 1.0),
                    1.0,
                );
                
                parent.spawn((
                    Node {
                        width: Val::Px(30.0),
                        height: Val::Px(30.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(color),
                    BorderColor(Color::WHITE),
                )).with_children(|cell| {
                    cell.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(1.0),
                            right: Val::Px(1.0),
                            ..default()
                        },
                        Text::new(format!("{:.1}", pixel.x)),
                    ));
                });
            }
        });
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

        for (probe, settings) in self.query.iter_manual(world) {
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
