//! Core systems for the probe camera.

use bevy::{
    asset::{Assets, RenderAssetUsages},
    ecs::system::{Commands, ResMut},
    log::info,
    math::{UVec2, Vec2, Vec4},
    prelude::*,
    render::{
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{BufferUsages, Extent3d, TextureDimension, TextureFormat, TextureUsages},
        storage::ShaderStorageBuffer,
    },
    utils::default,
};
use bevy_camera::visibility::RenderLayers;
use bevy_camera::{Camera3d, PerspectiveProjection, Projection, RenderTarget};

use crate::{
    camera::ProbeCameraController,
    probe::{
        components::{
            KernelSettings, KernelSize, PreviousKernelData, ProbeCamera, ProbeCameraBundle,
            ProbeKernelConfig,
        },
        events::{ProbeCameraCommand, ProbeCameraOutputMessage},
        frustum::FrustumNearPlaneIntersection,
        pipeline::KernelBindGroup,
    },
};

/// Result of spawning a probe camera.
#[derive(Debug)]
pub struct SpawnProbeCameraResult {
    /// The entity that was spawned.
    pub entity: Entity,
    /// The kernel size that was configured for this probe camera.
    pub kernel_size: KernelSize,
}

/// Observer function to handle GPU readback completion.
///
/// Only emits `KernelChanged` if the pixel data actually differs
/// from the previous frame.
pub fn handle_readback_complete(
    trigger: On<ReadbackComplete>,
    mut query: Query<(&ProbeKernelConfig, &mut PreviousKernelData)>,
    mut event_writer: MessageWriter<ProbeCameraOutputMessage>,
) {
    let entity = trigger.entity;

    // Get the kernel configuration and previous data for this probe camera
    let Ok((kernel_config, mut prev_data)) = query.get_mut(entity) else {
        info!("No kernel config found for entity {:?}", entity);
        return;
    };

    // Convert the readback data to kernel format
    let kernel: Vec<Vec4> = trigger.event().to_shader_type();

    // Check if data actually changed
    let changed = match &prev_data.data {
        None => true, // First frame, always "changed"
        Some(prev) => !kernel_eq(prev, &kernel),
    };

    if changed {
        // Update cached data
        prev_data.data = Some(kernel.clone());

        let event = ProbeCameraOutputMessage::KernelChanged {
            entity,
            data: kernel,
            kernel_size: kernel_config.kernel_size,
        };

        info!("{}", event);
        event_writer.write(event);
    }
}

/// Fast approximate equality check for kernel data.
///
/// Uses a small epsilon for floating point comparison.
#[inline]
fn kernel_eq(a: &[Vec4], b: &[Vec4]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    const EPSILON: f32 = 1e-5;

    a.iter().zip(b.iter()).all(|(va, vb)| {
        (va.x - vb.x).abs() < EPSILON
            && (va.y - vb.y).abs() < EPSILON
            && (va.z - vb.z).abs() < EPSILON
            && (va.w - vb.w).abs() < EPSILON
    })
}

/// Spawns a probe camera entity with all necessary components.
///
/// This is the recommended way to create probe cameras. It handles:
/// - Creating the render target texture
/// - Setting up the compute shader buffers
/// - Configuring the camera projection
/// - Adding all required components
///
/// Returns information about the spawned probe camera.
pub fn spawn_probe_camera(
    commands: &mut Commands,
    transform: Transform,
    resolution: UVec2,
    images: &mut Assets<Image>,
    ssbo_assets: &mut Assets<ShaderStorageBuffer>,
) -> SpawnProbeCameraResult {
    let aspect_ratio = resolution.x as f32 / resolution.y as f32;

    let size = Extent3d {
        width: resolution.x,
        height: resolution.y,
        ..default()
    };

    // Create the render target texture
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let image_handle = images.add(image);

    // Create kernel settings and storage buffer
    let kernel_size = KernelSize::new(3., 3.);
    let kernel_size_vec2: Vec2 = kernel_size.into();
    let settings = KernelSettings {
        kernel_size: kernel_size_vec2,
        center_coords: Vec2::new(0.5, 0.5),
    };

    let buffer = vec![Vec4::ZERO; kernel_size.pixel_count()];
    let mut ssbo = ShaderStorageBuffer::from(buffer);
    ssbo.buffer_description.usage |=
        BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC;
    let ssbo_handle = ssbo_assets.add(ssbo);

    // Create the bundle
    let bundle = ProbeCameraBundle {
        probe_camera: ProbeCamera { resolution },
        transform,
        camera: Camera {
            target: RenderTarget::Image(image_handle.clone().into()),
            order: 1, // Probe camera renders after main camera
            ..default()
        },
        camera_3d: Camera3d::default(),
        projection: Projection::Perspective(PerspectiveProjection {
            near: 2.0,
            far: 4.0,
            fov: std::f32::consts::PI / 3.0, // 60 degrees
            aspect_ratio,
        }),
        kernel_bind_group: KernelBindGroup {
            settings,
            source_texture: image_handle.clone(),
            output_buffer: ssbo_handle.clone(),
        },
        near_plane_intersection: FrustumNearPlaneIntersection::default(),
        readback: Readback::buffer(ssbo_handle),
        render_layers: RenderLayers::layer(1), // Probe camera only sees layer 1
        controller: ProbeCameraController::default(),
    };

    // Spawn the entity with the bundle and additional components
    let entity = commands
        .spawn(bundle)
        .insert((
            ProbeKernelConfig {
                kernel_size: kernel_size_vec2,
            },
            PreviousKernelData::default(),
        ))
        .id();

    SpawnProbeCameraResult {
        entity,
        kernel_size,
    }
}

/// Handles incoming probe camera commands.
pub fn handle_message(
    mut event_rdr: MessageReader<ProbeCameraCommand>,
    mut commands: Commands,
    mut ssbo_assets: ResMut<Assets<ShaderStorageBuffer>>,
    mut images: ResMut<Assets<Image>>,
) {
    for probe_camera_event in event_rdr.read() {
        match probe_camera_event {
            ProbeCameraCommand::Add {
                transform,
                resolution,
            } => {
                spawn_probe_camera(
                    &mut commands,
                    *transform,
                    *resolution,
                    &mut images,
                    &mut ssbo_assets,
                );
            }
        }
    }
}
