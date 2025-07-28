pub mod camera_controller;
pub mod probe;

use std::f32::consts::PI;

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    render::{
        extract_component::ExtractComponent,
        primitives::{CubemapFrusta, Frustum},
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers,
    },
};

use crate::{
    camera_controller::{
        orbit_camera_controller::{OrbitCameraController, OrbitCameraControllerPlugin},
        probe_camera_controller::{ProbeCameraController, ProbeCameraControllerPlugin},
    },
    probe::{near_plane::NearPlanePlugin, ProbeCamera, ProbePlugin},
};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins,
            OrbitCameraControllerPlugin,
            ProbeCameraControllerPlugin,
            ProbePlugin,
            NearPlanePlugin,
        ))
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(CameraFrustrumPlugin)
        .add_systems(
            Startup,
            (
                setup_main_camera,
                setup_cubes,
                setup_lighting,
                setup_probe_camera,
            ),
        )
        .add_systems(Update, RotateInPlace::update);
    }
}

#[derive(Component)]
pub struct MainCamera;

pub fn setup_main_camera(mut commands: Commands) {
    commands.spawn((
        OrbitCameraController {
            target: Vec3::ZERO,
            distance: 10.0,
            yaw: 0.0,
            pitch: 0.3,
            sensitivity: 0.005,
        },
        Camera::default(),
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 25.0)).looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera,
    ));
}

fn setup_lighting(mut commands: Commands) {
    // This specifies that the lights should affect both layer 0 (main scene)
    // and layer 1 (probe scene).
    let light_layers = RenderLayers::layer(0).with(1);

    // Add ambient light with improved settings
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.9, 0.9, 1.0), // Slightly blue-tinted white for better atmosphere
        brightness: 0.4,                   // Increased brightness for better visibility
        affects_lightmapped_meshes: true,
    });

    // Add primary directional light (sun-like)
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.9), // Warm sunlight color
            illuminance: 12000.0,               // More realistic illuminance value
            shadows_enabled: true,
            shadow_depth_bias: 0.02, // Reduce shadow acne
            shadow_normal_bias: 0.6, // Improve shadow quality
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        light_layers.clone(),
    ));

    // Add secondary fill light (opposite direction, lower intensity)
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.8, 0.85, 1.0), // Cooler color for fill light
            illuminance: 3000.0,                // Lower intensity than main light
            shadows_enabled: false,             // No shadows from fill lightå
            ..default()
        },
        Transform::from_xyz(-3.0, 5.0, -3.0).looking_at(Vec3::ZERO, Vec3::Y),
        light_layers,
    ));
}

fn setup_cubes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Define the layer for objects that should be seen by the probe.
    // let probe_visible_layer = RenderLayers::layer(1);

    for y in -5..5 {
        for z in -5..5 {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
                MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
                Transform::from_xyz(0.0, y as f32, z as f32),
                RotateInPlace,
                // Add the layer here. Now only the probe camera will see this cube.
                // The main camera (on layer 0) will not.
                // probe_visible_layer,
            ));
        }
    }
}

#[derive(Component)]
pub struct RotateInPlace;

impl RotateInPlace {
    fn update(mut query: Query<&mut Transform, With<RotateInPlace>>, time: Res<Time>) {
        // for mut transform in query.iter_mut() {
        //     transform.rotate(Quat::from_rotation_y(time.delta_secs()));
        // }
    }
}

fn setup_probe_camera(mut commands: Commands) {
    commands.spawn((
        Transform::from_xyz(4.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ProbeCamera {
            resolution: UVec2::new(1024, 1024),
        },
    ));
}

//https://hackmd.io/@bevy/rendering_summary

pub struct CameraFrustrumPlugin;

impl Plugin for CameraFrustrumPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera_frustum)
            .add_systems(Update, CameraFrustum::draw_cursor);
    }
}

fn spawn_camera_frustum(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let frustum_params = CameraFrustumParams {
        fov_y: PI / 4.0, // 45 degrees
        aspect_ratio: 16.0 / 9.0,
        near: 0.1,
        far: 100.0,
    };

    let frustum = CameraFrustum::from_params(frustum_params);

    // Spawn near plane visualization
    commands.spawn((
        Transform::from_translation(Vec3::new(0.0, 0.0, -frustum_params.near)),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(2.0, 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.0, 1.0, 0.0, 0.3),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        frustum,
    ));
}

#[derive(Component)]
pub struct CameraFrustum {
    pub planes: [Vec4; 6], // [near, far, left, right, top, bottom]
    pub params: CameraFrustumParams,
}

#[derive(Clone, Copy)]
pub struct CameraFrustumParams {
    pub fov_y: f32,        // Field of view in Y direction (radians)
    pub aspect_ratio: f32, // Width / Height
    pub near: f32,         // Near plane distance
    pub far: f32,          // Far plane distance
}

impl CameraFrustum {
    /// Create a frustum from standard camera parameters
    pub fn from_params(params: CameraFrustumParams) -> Self {
        let CameraFrustumParams {
            fov_y,
            aspect_ratio,
            near,
            far,
        } = params;

        // Calculate half-extents at near plane
        let half_height_near = near * (fov_y * 0.5).tan();
        let half_width_near = half_height_near * aspect_ratio;

        // Calculate half-extents at far plane
        let half_height_far = far * (fov_y * 0.5).tan();
        let half_width_far = half_height_far * aspect_ratio;

        // Define the 6 planes as Vec4 (normal.xyz, distance)
        // Plane equation: normal·point + distance = 0
        let planes = [
            // Near plane: normal points toward camera (+Z in view space)
            Vec4::new(0.0, 0.0, 1.0, near),
            // Far plane: normal points away from camera (-Z in view space)
            Vec4::new(0.0, 0.0, -1.0, -far),
            // Left plane
            Vec4::new(1.0, 0.0, near / half_width_near, 0.0).normalize(),
            // Right plane
            Vec4::new(-1.0, 0.0, near / half_width_near, 0.0).normalize(),
            // Top plane
            Vec4::new(0.0, -1.0, near / half_height_near, 0.0).normalize(),
            // Bottom plane
            Vec4::new(0.0, 1.0, near / half_height_near, 0.0).normalize(),
        ];

        Self { planes, params }
    }

    /// Create frustum from view and projection matrices
    pub fn from_matrices(view: Mat4, projection: Mat4) -> Self {
        let combined = projection * view;
        let m = combined.to_cols_array();

        // Extract planes from combined matrix using Gribb/Hartmann method
        let planes = [
            // Near: row3 + row2
            Vec4::new(m[3] + m[2], m[7] + m[6], m[11] + m[10], m[15] + m[14]).normalize(),
            // Far: row3 - row2
            Vec4::new(m[3] - m[2], m[7] - m[6], m[11] - m[10], m[15] - m[14]).normalize(),
            // Left: row3 + row0
            Vec4::new(m[3] + m[0], m[7] + m[4], m[11] + m[8], m[15] + m[12]).normalize(),
            // Right: row3 - row0
            Vec4::new(m[3] - m[0], m[7] - m[4], m[11] - m[8], m[15] - m[12]).normalize(),
            // Top: row3 - row1
            Vec4::new(m[3] - m[1], m[7] - m[5], m[11] - m[9], m[15] - m[13]).normalize(),
            // Bottom: row3 + row1
            Vec4::new(m[3] + m[1], m[7] + m[5], m[11] + m[9], m[15] + m[13]).normalize(),
        ];

        // Derive parameters from planes (approximate)
        let params = CameraFrustumParams {
            fov_y: PI / 4.0,          // Default, would need more complex extraction
            aspect_ratio: 16.0 / 9.0, // Default
            near: -planes[0].w,
            far: planes[1].w,
        };

        Self { planes, params }
    }

    /// Test if a point is inside the frustum
    pub fn contains_point(&self, point: Vec3) -> bool {
        for plane in &self.planes {
            let distance = plane.x * point.x + plane.y * point.y + plane.z * point.z + plane.w;
            if distance < 0.0 {
                return false;
            }
        }
        true
    }

    /// Test if a sphere intersects the frustum
    pub fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            let distance = plane.x * center.x + plane.y * center.y + plane.z * center.z + plane.w;
            if distance < -radius {
                return false;
            }
        }
        true
    }

    fn draw_cursor(
        camera_query: Single<(&Camera, &GlobalTransform)>,
        ground: Single<&GlobalTransform, With<Self>>,
        windows: Query<&Window>,
        mut gizmos: Gizmos,
    ) {
        let Ok(windows) = windows.single() else {
            return;
        };

        let (camera, camera_transform) = *camera_query;

        let Some(cursor_position) = windows.cursor_position() else {
            return;
        };

        // Calculate a ray pointing from the camera into the world based on the cursor's position.
        let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
            return;
        };

        // Calculate if and where the ray is hitting the ground plane.
        let Some(distance) =
            ray.intersect_plane(ground.translation(), InfinitePlane3d::new(ground.up()))
        else {
            return;
        };
        let point = ray.get_point(distance);

        // Draw a circle just above the ground plane at that position.
        gizmos.circle(
            Isometry3d::new(
                point + ground.up() * 0.01,
                Quat::from_rotation_arc(Vec3::Z, ground.up().as_vec3()),
            ),
            0.2,
            Color::WHITE,
        );
    }
}
