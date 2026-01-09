use super::CameraSettings;

use super::CameraController;
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::pointer::{PointerAction, PointerInput},
    prelude::*,
};

use std::{f32::consts::FRAC_PI_2, ops::RangeInclusive};

#[derive(Default)]
pub struct OrbitCameraControllerPlugin<T: CameraSettings>(pub T);

/// System sets for camera controller ordering
#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum CameraControllerSystemSet {
    /// Input handling (reads pointer/scroll events)
    InputHandling,
    /// Process camera input events
    ProcessInput,
    /// Update camera transform (runs last, after all interactions)
    UpdateTransform,
}

impl<T: CameraSettings + Send + Sync + 'static> Plugin for OrbitCameraControllerPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_resource::<T>()
            .add_message::<CameraInputEvent>()
            .configure_sets(
                Update,
                (
                    CameraControllerSystemSet::InputHandling,
                    CameraControllerSystemSet::ProcessInput
                        .after(CameraControllerSystemSet::InputHandling),
                ),
            )
            .configure_sets(PostUpdate, (CameraControllerSystemSet::UpdateTransform,))
            .add_systems(
                Update,
                (Self::handle_pointer_input, Self::handle_scroll_input)
                    .chain()
                    .in_set(CameraControllerSystemSet::InputHandling),
            )
            .add_systems(
                Update,
                Self::process_camera_input.in_set(CameraControllerSystemSet::ProcessInput),
            )
            .add_systems(
                PostUpdate,
                Self::update_camera_transform
                    .in_set(CameraControllerSystemSet::UpdateTransform)
                    .run_if(run_criteria::<T>),
            );
    }
}

fn run_criteria<T: CameraSettings>(mode: Res<T>) -> bool {
    !(*mode).is_locked()
}

#[derive(Event, Message)]
pub enum CameraInputEvent {
    Rotate { delta: Vec2 },
    Pan { delta: Vec2 },
    Zoom { delta: f32 },
}

#[derive(Component)]
pub struct OrbitCameraController {
    /// Horizontal rotation angle (yaw) around the target
    pub yaw: f32,
    /// Vertical rotation angle (pitch) around the target  
    pub pitch: f32,
    /// Allowable pitch range to prevent camera flipping
    pub pitch_range: RangeInclusive<f32>,
    /// Distance from the target center
    pub distance: f32,
    /// Distance limits
    pub distance_range: RangeInclusive<f32>,
    /// The center point of the camera's orbit
    pub center: Vec3,
    /// Rotation sensitivity
    pub rotate_sensitivity: f32,
    /// Panning sensitivity
    pub pan_sensitivity: f32,
    /// Zoom sensitivity
    pub zoom_sensitivity: f32,
    /// Mouse button for rotation
    pub rotate_button: MouseButton,
    /// Mouse button for panning
    pub pan_button: MouseButton,
    /// Whether the controller is enabled
    pub enabled: bool,
    /// Smoothing factor for camera movement (0.0 = no smoothing, 1.0 = instant)
    pub smoothing: f32,
    /// Target values for smooth interpolation
    target_yaw: f32,
    target_pitch: f32,
    target_distance: f32,
    target_center: Vec3,
}

impl Default for OrbitCameraController {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: FRAC_PI_2 * 0.3, // Start at a nice angle
            pitch_range: -FRAC_PI_2 * 0.99..=FRAC_PI_2 * 0.99, // Prevent gimbal lock
            distance: 20.0,
            distance_range: 1.0..=100.0,
            center: Vec3::ZERO,
            rotate_sensitivity: 1.0,
            pan_sensitivity: 1.0,
            zoom_sensitivity: 1.0,
            rotate_button: MouseButton::Left,
            pan_button: MouseButton::Right,
            enabled: true,
            smoothing: 0.9,
            target_yaw: 0.0,
            target_pitch: FRAC_PI_2 * 0.3,
            target_distance: 20.0,
            target_center: Vec3::ZERO,
        }
    }
}

impl OrbitCameraController {
    pub fn new(distance: f32, center: Vec3, initial_transform: Transform) -> Self {
        // Calculate initial angles from transform
        let to_center = (center - initial_transform.translation).normalize();

        let yaw = (-to_center.x).atan2(-to_center.z);
        let pitch = to_center.y.asin();

        Self {
            distance,
            center,
            yaw,
            pitch,
            target_yaw: yaw,
            target_pitch: pitch,
            target_distance: distance,
            target_center: center,
            ..Default::default()
        }
    }

    /// Generate the camera transform based on current orbit parameters
    pub fn generate_transform(&self) -> Transform {
        let rotation = Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0);
        let offset = rotation * Vec3::new(0.0, 0.0, self.distance);
        let translation = self.center + offset;

        Transform::from_translation(translation).looking_at(self.center, Vec3::Y)
    }

    /// Update the controller with input delta, respecting constraints
    fn apply_rotation(&mut self, delta: Vec2, time_delta: f32) {
        if !self.enabled {
            return;
        }

        self.target_yaw -= delta.x * self.rotate_sensitivity * time_delta;
        self.target_pitch = (self.target_pitch - delta.y * self.rotate_sensitivity * time_delta)
            .clamp(*self.pitch_range.start(), *self.pitch_range.end());
    }

    fn apply_pan(&mut self, delta: Vec2, transform: &Transform, time_delta: f32) {
        if !self.enabled {
            return;
        }

        let right = transform.right();
        let up = transform.up();
        let pan_vector = (right * -delta.x + up * delta.y)
            * self.pan_sensitivity
            * time_delta
            * (self.distance * 0.1); // Scale by distance for consistent feel

        self.target_center += pan_vector;
    }

    fn apply_zoom(&mut self, delta: f32) {
        if !self.enabled {
            return;
        }

        let zoom_factor = 1.0 + delta * self.zoom_sensitivity * 0.1;
        self.target_distance = (self.target_distance / zoom_factor)
            .clamp(*self.distance_range.start(), *self.distance_range.end());
    }

    /// Smoothly interpolate towards target values
    fn update_smooth(&mut self, time_delta: f32) {
        let lerp_factor = 1.0 - (1.0 - self.smoothing).powf(time_delta * 60.0); // 60fps reference

        self.yaw = self.yaw + (self.target_yaw - self.yaw) * lerp_factor;
        self.pitch = self.pitch + (self.target_pitch - self.pitch) * lerp_factor;
        self.distance = self.distance + (self.target_distance - self.distance) * lerp_factor;
        self.center = self.center.lerp(self.target_center, lerp_factor);
    }
}

impl CameraController for OrbitCameraController {
    fn update_camera_transform_system(
        mut query: Query<
            (&Self, &mut Transform),
            (Or<(Changed<Self>, Added<Self>)>, With<Camera3d>),
        >,
    ) {
        for (controller, mut transform) in query.iter_mut() {
            if controller.enabled {
                *transform = controller.generate_transform();
            }
        }
    }
}

impl<T: CameraSettings> OrbitCameraControllerPlugin<T> {
    fn handle_pointer_input(
        mut pointer_events: MessageReader<PointerInput>,
        mouse_input: Res<ButtonInput<MouseButton>>,
        mut camera_events: MessageWriter<CameraInputEvent>,
        camera_query: Query<&OrbitCameraController>,
        settings: Res<T>,
    ) {
        if settings.is_locked() {
            return;
        }

        let Some(controller) = camera_query.iter().next() else {
            return;
        };
        if !controller.enabled {
            return;
        }

        for event in pointer_events.read() {
            if let PointerAction::Move { delta } = event.action {
                if mouse_input.pressed(controller.rotate_button) {
                    camera_events.write(CameraInputEvent::Rotate { delta });
                } else if mouse_input.pressed(controller.pan_button) {
                    camera_events.write(CameraInputEvent::Pan { delta });
                }
            }
        }
    }

    fn handle_scroll_input(
        mut scroll_events: MessageReader<MouseWheel>,
        mut camera_events: MessageWriter<CameraInputEvent>,
        camera_query: Query<&OrbitCameraController>,
        settings: Res<T>,
    ) {
        if settings.is_locked() {
            return;
        }

        let Some(controller) = camera_query.iter().next() else {
            return;
        };
        if !controller.enabled {
            return;
        }

        let mut total_delta = 0.0;
        for event in scroll_events.read() {
            total_delta += event.y
                * match event.unit {
                    MouseScrollUnit::Line => 1.0,
                    MouseScrollUnit::Pixel => 0.01,
                };
        }

        if total_delta != 0.0 {
            camera_events.write(CameraInputEvent::Zoom { delta: total_delta });
        }
    }

    fn process_camera_input(
        mut camera_events: MessageReader<CameraInputEvent>,
        mut camera_query: Query<(&mut OrbitCameraController, &Transform)>,
        time: Res<Time>,
    ) {
        let Ok((mut controller, transform)) = camera_query.single_mut() else {
            return;
        };
        let time_delta = time.delta_secs();

        for event in camera_events.read() {
            match event {
                CameraInputEvent::Rotate { delta } => {
                    controller.apply_rotation(*delta, time_delta);
                }
                CameraInputEvent::Pan { delta } => {
                    controller.apply_pan(*delta, transform, time_delta);
                }
                CameraInputEvent::Zoom { delta } => {
                    controller.apply_zoom(*delta);
                }
            }
        }
    }

    fn update_camera_transform(
        mut camera_query: Query<(&mut OrbitCameraController, &mut Transform), With<Camera3d>>,
        time: Res<Time>,
    ) {
        for (mut controller, mut transform) in camera_query.iter_mut() {
            if controller.enabled {
                controller.update_smooth(time.delta_secs());
                *transform = controller.generate_transform();
            }
        }
    }
}
