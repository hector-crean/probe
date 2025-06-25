pub mod camera_controller;
pub mod probe;

use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::{
        primitives::{Frustum, CubemapFrusta},
        extract_component::ExtractComponent,
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        view::RenderLayers,
    },
    input::mouse::MouseMotion,
};

use crate::{camera_controller::{CameraControllerPlugin, ControlledCamera, SimpleOrbitCamera}, probe::{visualisation::ProbeVisualizationPlugin, Probe, ProbePlugin}};

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DefaultPlugins, CameraControllerPlugin, ProbePlugin, ProbeVisualizationPlugin))
            .add_systems(Startup, (setup_main_camera, setup_cubes, setup_lighting, setup_probe_camera))
            .add_systems(Update, RotateInPlace::update);
    }
}


pub fn setup_main_camera(mut commands: Commands) {
    commands.spawn((
        SimpleOrbitCamera {
            target: Vec3::ZERO,
            distance: 10.0,
            yaw: 0.0,
            pitch: 0.3,
            sensitivity: 0.005,
        },
        Camera::default(),
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 25.0)).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn setup_lighting(mut commands: Commands) {
     // Add ambient light with improved settings
     commands.insert_resource(AmbientLight {
        color: Color::srgb(0.9, 0.9, 1.0), // Slightly blue-tinted white for better atmosphere
        brightness: 0.4,                  // Increased brightness for better visibility
        affects_lightmapped_meshes: true,
    });

    // Add primary directional light (sun-like)
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.9), // Warm sunlight color
            illuminance: 12000.0,              // More realistic illuminance value
            shadows_enabled: true,
            shadow_depth_bias: 0.02,           // Reduce shadow acne
            shadow_normal_bias: 0.6,           // Improve shadow quality
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Add secondary fill light (opposite direction, lower intensity)
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.8, 0.85, 1.0), // Cooler color for fill light
            illuminance: 3000.0,               // Lower intensity than main light
            shadows_enabled: false,            // No shadows from fill lightå
            ..default()
        },
        Transform::from_xyz(-3.0, 5.0, -3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}


fn setup_cubes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Define the layer for objects that should be seen by the probe.
    // let probe_visible_layer = RenderLayers::layer(1);

    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        RotateInPlace,
        // Add the layer here. Now only the probe camera will see this cube.
        // The main camera (on layer 0) will not.
        // probe_visible_layer,
    ));
}



#[derive(Component)]
pub struct RotateInPlace;

impl RotateInPlace  {
    fn update(mut query: Query<&mut Transform, With<RotateInPlace>>, time: Res<Time>) {
        for mut transform in query.iter_mut() {
            transform.rotate(Quat::from_rotation_y(time.delta_secs()));
        }
    }
}


fn setup_probe_camera(mut commands: Commands) {
    commands.spawn((
       Transform::from_xyz(10., 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
       Probe {
        resolution: UVec2::new(1024, 1024),
    },
    ));
}









//https://hackmd.io/@bevy/rendering_summary