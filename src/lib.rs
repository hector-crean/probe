pub mod camera_controller;
pub mod probe;

use bevy::{
    prelude::*,
    render::{
        view::RenderLayers,
    },
};

use crate::{
    camera_controller::{
        orbit_camera_controller::{OrbitCameraController, OrbitCameraControllerPlugin},
        probe_camera_controller::ProbeCameraControllerPlugin,
    },
    probe::{near_plane::NearPlanePlugin, ProbeCamera, ProbePlugin, events::ProbeCameraEvent},
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
        .add_systems(
            Startup,
            (
                setup_main_camera,
                setup_cubes,
                setup_lighting,
                setup_probe_camera,
            ),
        );
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
        Camera {
            order: 0, // Main camera renders first
            ..default()
        },
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 25.0)).looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera,
        RenderLayers::layer(0), // Main camera only sees layer 0
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
            shadows_enabled: false,             // No shadows from fill light
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
    for y in -5..5 {
        for z in -5..5 {
            // Generate distinct colors based on position
            let hue: f32 = ((y + 5) * 10 + (z + 5)) as f32 * 36.0; // Spread hues across spectrum
            let saturation: f32 = 0.8 + ((y + z) as f32 * 0.02); // Vary saturation slightly
            let lightness: f32 = 0.5 + ((y * z) as f32 * 0.03); // Vary lightness
            
            // Convert HSL to RGB for more predictable color distribution
            let color = Color::hsl(hue % 360.0, saturation.clamp(0.0, 1.0), lightness.clamp(0.3, 0.8));
            
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
                MeshMaterial3d(materials.add(color)),
                Transform::from_xyz(0.0, y as f32, z as f32),
                RenderLayers::layer(0).with(1), // Visible to both main camera (layer 0) and probe camera (layer 1)
            ));
        }
    }
}

fn setup_probe_camera(mut event_writer: EventWriter<ProbeCameraEvent>) {
    event_writer.write(ProbeCameraEvent::Add {
        transform: Transform::from_xyz(4.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        resolution: UVec2::new(1024, 1024),
    });
}
