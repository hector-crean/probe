pub mod events;
pub mod frustum;
pub mod kernel;
pub mod probe_pipeline;

use crate::probe_tool::{frustum::{near_plane_interaction::FrustumNearPlaneIntersection, FrustumPlugin}, kernel::{KernelDataResource, KernelHUDPlugin}, probe_pipeline::{KernelBindGroup, ProbePipelinePlugin}};

use self::{
    events::{ProbeCameraEvent},
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





#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
// #[source(ToolState = ToolState::Probe)]
pub enum ProbeToolState {
    #[default]
    Idle,
    Probing,
} 






const PROBE_KERNEL_SMALL_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("5eb828ff-9ee5-4c25-a12a-886e2aeb096d");
const PROBE_KERNEL_MEDIUM_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("5db818ff-9ee5-4c25-a12a-886e2aeb096d");
const PROBE_KERNEL_LARGE_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("5db827ff-9ee5-4c25-a12a-886e2aeb096d");



/// This plugin provides the components and systems for GPU-based render target probing.
pub struct ProbeToolPlugin;

impl Plugin for ProbeToolPlugin {
    fn build(&self, app: &mut App) {
       

        app.add_plugins((
            KernelHUDPlugin,
            FrustumPlugin,
            ProbePipelinePlugin
        ))
        .add_event::<ProbeCameraEvent>()
        .init_state::<ProbeToolState>()
        .add_systems(
            Update,
            Self::handle_state_transition
                .run_if(on_event::<StateTransitionEvent<ProbeToolState>>),
        )
        .add_systems(Update, (Self::handle_event));
    }

 
}




impl ProbeToolPlugin {
    fn handle_state_transition(
        mut state_reader: EventReader<StateTransitionEvent<ProbeToolState>>,
    ) {
        for event in state_reader.read() {
            info!("ProbeToolPlugin state changed from {:?} to {:?}", event.exited, event.entered);
        }
    } 
    pub fn toggle_probing_state(
        mut next_state: ResMut<NextState<ProbeToolState>>,
        current_state: Res<State<ProbeToolState>>,
        keyboard_input: Res<ButtonInput<KeyCode>>,
    ) {
        if keyboard_input.just_pressed(KeyCode::KeyP) {
            let new_state = match current_state.get() {
                ProbeToolState::Idle => ProbeToolState::Probing,
                ProbeToolState::Probing => ProbeToolState::Idle,
            };
            next_state.set(new_state);
        }
    } 
}







/// A component for a 3D camera that should be probed.
#[derive(Component)]
pub struct ProbeCamera {
    pub resolution: UVec2,
}
impl Default for ProbeCamera {
    fn default() -> Self {
        Self {
            resolution: UVec2::new(512, 512),
        }
    }
}

/// A query data structure for all components required by ProbeCamera.
/// This makes it easy to query for entities that have all the probe camera components.
#[derive(QueryData)]
#[query_data(mutable)]
pub struct ProbeCameraQuery {
    pub entity: Entity,
    pub probe_camera: &'static ProbeCamera,
    pub transform: &'static mut Transform,
    pub camera_3d: &'static Camera3d,
    pub camera: &'static mut Camera,
    pub projection: &'static mut Projection,
    pub kernel_settings: &'static mut KernelSettings,
    pub kernel_bind_group: &'static mut KernelBindGroup,
    pub near_plane_intersection: &'static mut FrustumNearPlaneIntersection,
    pub readback: &'static mut Readback,
}

#[derive(Component, Clone, PartialEq, ExtractComponent, ShaderType, Default)]
pub struct KernelSettings {
    /// Size of the kernel in pixels.
    pub kernel_size: Vec2,
    /// Normalized coordinates of the probe center on the render target.
    pub center_coords: Vec2,
    // Note: Std140 layout requires fields to be 16-byte aligned.
    // The `Vec2`s are padded automatically by the `ShaderType` derive.
}


impl ProbeToolPlugin {
    /// Creates all the necessary components for a probe camera.
    /// Returns a tuple of (components_bundle, image_handle, ssbo_handle, observer_closure)
    fn create_probe_camera_components(
        transform: Transform,
        resolution: UVec2,
        images: &mut Assets<Image>,
        ssbo_assets: &mut Assets<ShaderStorageBuffer>,
    ) -> (
        (
            ProbeCamera,
            KernelSettings,
            Transform,
            Camera,
            Camera3d,
            Projection,
            KernelBindGroup,
            FrustumNearPlaneIntersection,
            Readback,
            RenderLayers,
        ),
        Handle<Image>,
        Handle<ShaderStorageBuffer>,
        Vec2, // kernel_size for the observer closure
    ) {
        let aspect_ratio = resolution.x as f32 / resolution.y as f32;

        let size = Extent3d {
            width: resolution.x,
            height: resolution.y,
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
        let settings = KernelSettings {
            kernel_size,
            center_coords: Vec2::new(0.5, 0.5),
        };

        let buffer = vec![Vec4::ZERO; (kernel_size.x * kernel_size.y) as usize];
        let mut ssbo = ShaderStorageBuffer::from(buffer);
        ssbo.buffer_description.usage |=
            BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
        let ssbo_handle = ssbo_assets.add(ssbo);

        let components = (
            ProbeCamera {
                resolution: resolution.clone(),
            },
            settings.clone(),
            transform.clone(),
            Camera {
                // Render to our texture instead of the main window
                target: RenderTarget::Image(image_handle.clone().into()),
                order: 1, // Probe camera renders after main camera
                ..default()
            },
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                near: 2.0,
                far: 4.0,
                fov: std::f32::consts::PI / 3.0, // 60 degrees
                aspect_ratio,                    // Explicitly set the aspect ratio
            }),
            KernelBindGroup {
                settings,
                source_texture: image_handle.clone(),
                output_buffer: ssbo_handle.clone(),
            },
            FrustumNearPlaneIntersection::default(),
            Readback::buffer(ssbo_handle.clone()),
            RenderLayers::layer(1), // Probe camera only sees layer 1 (excludes gizmos)
        );

        (components, image_handle, ssbo_handle, kernel_size)
    }

    /// Attaches the necessary probe components to any camera that has the `Probe` marker component.
    fn handle_event(
        mut event_rdr: EventReader<ProbeCameraEvent>,
        mut commands: Commands,
        mut ssbo_assets: ResMut<Assets<ShaderStorageBuffer>>,
        mut images: ResMut<Assets<Image>>,
    ) {
        for probe_camera_event in event_rdr.read() {
            match probe_camera_event {
                ProbeCameraEvent::Add {
                    transform,
                    resolution,
                } => {
                    let (components, _image_handle, _ssbo_handle, kernel_size) =
                        Self::create_probe_camera_components(
                            transform.clone(),
                            resolution.clone(),
                            &mut images,
                            &mut ssbo_assets,
                        );

                    commands
                        .spawn(components)
                        .observe(
                            move |trigger: Trigger<ReadbackComplete>,
                                  mut kernel_data: ResMut<KernelDataResource>| {
                                // This matches the type which was used to create the `ShaderStorageBuffer` above,
                                // and is a convenient way to interpret the data.
                                let kernel: Vec<Vec4> = trigger.event().to_shader_type();

                                // Simple checksum to track if data is changing
                                let checksum: f32 = kernel.iter().map(|v| v.x + v.y + v.z + v.w).sum();

                                // Update the kernel data resource for UI visualization
                                kernel_data.data = kernel;
                                kernel_data.kernel_size = kernel_size; // Use the actual kernel size

                                info!(
                                    "Readback complete: checksum={:.3}, sample_pixel={:?}",
                                    checksum,
                                    kernel_data.data.get(0)
                                );
                            },
                        );
                }
            }
        }
    }

   
}