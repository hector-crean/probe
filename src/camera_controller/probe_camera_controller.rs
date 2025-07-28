use bevy::prelude::*;

use crate::probe_tool::ProbeCamera;


pub struct ProbeCameraControllerPlugin;

impl Plugin for ProbeCameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, frustrum_camera_controller);
    }
}

/// Simple camera controller for moving around the scene
#[derive(Component)]
pub struct ProbeCameraController {
    pub sensitivity: f32,
    pub speed: f32,
}

impl Default for ProbeCameraController {
    fn default() -> Self {
        Self {
            sensitivity: 0.002,
            speed: 5.0,
        }
    }
}

fn frustrum_camera_controller(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &ProbeCameraController), With<ProbeCamera>>,
) {
    let Ok((mut transform, controller)) = query.single_mut() else {
        return;
    };


    // Handle WASD movement
    let mut velocity = Vec3::ZERO;
    let local_z = transform.local_z();
    let forward = -Vec3::new(local_z.x, 0.0, local_z.z).normalize();
    let right = Vec3::new(local_z.z, 0.0, -local_z.x).normalize();

    if keyboard.pressed(KeyCode::KeyW) {
        velocity += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        velocity -= forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        velocity -= right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        velocity += right;
    }
    if keyboard.pressed(KeyCode::Space) {
        velocity += Vec3::Y;
    }
    if keyboard.pressed(KeyCode::ControlLeft) {
        velocity -= Vec3::Y;
    }

    if velocity != Vec3::ZERO {
        transform.translation += velocity.normalize() * controller.speed * time.delta_secs();
    }
}
