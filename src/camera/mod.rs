//! Camera controllers for the probe system.
use bevy::prelude::*;

mod orbit;
mod probe_controller;

pub use orbit::{OrbitCameraController, OrbitCameraControllerPlugin};
pub use probe_controller::{ProbeCameraController, ProbeCameraControllerPlugin};



#[derive(Component)]
pub struct MainCamera;


pub trait CameraController: Component
where
    Self: Sized,
{
    fn update_camera_transform_system(
        query: Query<(&Self, &mut Transform), (Or<(Changed<Self>, Added<Self>)>, With<Camera3d>)>,
    );
}

pub trait CameraSettings: Resource + Default + PartialEq + Eq {
    fn is_locked(&self) -> bool;
    fn lock(&mut self);
    fn unlock(&mut self);
}

#[derive(Resource, Default, PartialEq, Eq)]
pub struct CameraSettingsImpl {
    is_locked: bool,
}

impl CameraSettings for CameraSettingsImpl {
    fn is_locked(&self) -> bool {
        self.is_locked
    }
    fn lock(&mut self) {
        self.is_locked = true;
    }
    fn unlock(&mut self) {
        self.is_locked = false;
    }
}
