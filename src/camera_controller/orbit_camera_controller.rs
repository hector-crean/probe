use bevy::{input::mouse::MouseMotion, prelude::*};


pub struct OrbitCameraControllerPlugin;

impl Plugin for OrbitCameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, Self::simple_orbit_camera_system);
    }
}

/// Simple orbit camera component that works with direct mouse input
#[derive(Component)]
pub struct OrbitCameraController {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,   // rotation around Y axis
    pub pitch: f32, // rotation around X axis
    pub sensitivity: f32,
}

impl Default for OrbitCameraController {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 10.0,
            yaw: 0.0,
            pitch: 0.3,
            sensitivity: 0.005,
        }
    }
}

impl OrbitCameraControllerPlugin {
   
    /// Simple orbit camera system that uses direct mouse input
    pub fn simple_orbit_camera_system(
        mut mouse_motion_events: MessageReader<MouseMotion>,
        mut mouse_wheel_events: MessageReader<bevy::input::mouse::MouseWheel>,
        mouse_button_input: Res<ButtonInput<MouseButton>>,
        mut query: Query<(&mut OrbitCameraController, &mut Transform)>,
    ) {
        for (mut orbit_camera, mut transform) in query.iter_mut() {
            // Handle mouse motion for orbiting
            if mouse_button_input.pressed(MouseButton::Left) {
                for event in mouse_motion_events.read() {
                    orbit_camera.yaw -= event.delta.x * orbit_camera.sensitivity;
                    orbit_camera.pitch -= event.delta.y * orbit_camera.sensitivity;

                    // Clamp pitch to prevent flipping
                    orbit_camera.pitch = orbit_camera.pitch.clamp(-1.5, 1.5);
                }
            }

            // Handle mouse wheel for zooming
            for event in mouse_wheel_events.read() {
                orbit_camera.distance -= event.y * 2.0;
                orbit_camera.distance = orbit_camera.distance.clamp(3.0, 50.0);
            }

            // Update camera transform
            let rotation = Quat::from_axis_angle(Vec3::Y, orbit_camera.yaw)
                * Quat::from_axis_angle(Vec3::X, orbit_camera.pitch);
            let position =
                orbit_camera.target + rotation * Vec3::new(0.0, 0.0, orbit_camera.distance);

            *transform =
                Transform::from_translation(position).looking_at(orbit_camera.target, Vec3::Y);
        }

        // Clear the events if mouse button is not pressed
        if !mouse_button_input.pressed(MouseButton::Left) {
            mouse_motion_events.clear();
        }
    }
}
