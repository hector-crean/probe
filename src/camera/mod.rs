//! Camera controllers for the probe system.
use bevy::prelude::*;

mod orbit;
mod probe_controller;

pub use orbit::{OrbitCameraController, OrbitCameraControllerPlugin};
pub use probe_controller::{ProbeCameraController, ProbeCameraControllerPlugin};

/// Extension trait for Bevy's Projection with additional helper methods.
///
/// This trait provides projection-specific calculations that don't require
/// a Camera component, useful for visualization and mesh building.
pub trait ProjectionExt {
    /// Returns the near plane distance.
    fn near_distance(&self) -> Option<f32>;

    /// Returns the far plane distance.
    fn far_distance(&self) -> Option<f32>;

    /// Returns (half_width, half_height) for the near plane.
    ///
    /// For perspective projection: half_height = near * tan(fov/2), half_width = half_height * aspect.
    /// For orthographic projection: uses the projection's area.
    fn near_plane_half_extents(&self) -> Option<Vec2>;
}

impl ProjectionExt for Projection {
    fn near_distance(&self) -> Option<f32> {
        match self {
            Projection::Perspective(p) => Some(p.near),
            Projection::Orthographic(p) => Some(p.near),
            _ => None,
        }
    }

    fn far_distance(&self) -> Option<f32> {
        match self {
            Projection::Perspective(p) => Some(p.far),
            Projection::Orthographic(p) => Some(p.far),
            _ => None,
        }
    }

    fn near_plane_half_extents(&self) -> Option<Vec2> {
        match self {
            Projection::Perspective(p) => {
                let half_height = p.near * (p.fov * 0.5).tan();
                let half_width = half_height * p.aspect_ratio;
                Some(Vec2::new(half_width, half_height))
            }
            Projection::Orthographic(p) => {
                let half_width = (p.area.max.x - p.area.min.x) * 0.5;
                let half_height = (p.area.max.y - p.area.min.y) * 0.5;
                Some(Vec2::new(half_width, half_height))
            }
            _ => None,
        }
    }
}

/// Extension trait for Bevy's Camera with additional coordinate conversion methods.
pub trait CameraExt {
    /// Converts UV coordinates [0,1] to a world position on the near plane.
    /// Returns None if the conversion fails or if not using a supported projection.
    fn uv_to_near_plane_world(
        &self,
        camera_transform: &GlobalTransform,
        projection: &Projection,
        uv: Vec2,
    ) -> Option<Vec3>;

    /// Converts viewport coordinates (pixels) to a world position on the near plane.
    fn viewport_to_near_plane_world(
        &self,
        camera_transform: &GlobalTransform,
        projection: &Projection,
        viewport_pos: Vec2,
    ) -> Option<Vec3>;

    /// Returns the near plane dimensions: (near_distance, half_width, half_height).
    ///
    /// For perspective projection, half_height = near * tan(fov/2), half_width = half_height * aspect.
    /// For orthographic projection, uses the projection's area.
    fn near_plane_dimensions(&self, projection: &Projection) -> Option<(f32, f32, f32)>;

    /// Returns the world position of the near plane center.
    fn near_plane_center(
        &self,
        camera_transform: &GlobalTransform,
        projection: &Projection,
    ) -> Option<Vec3>;

    /// Converts a world position to UV coordinates [0,1] on the near plane.
    ///
    /// This is the inverse of `uv_to_near_plane_world`.
    /// Returns None if the point cannot be projected onto the near plane.
    fn world_to_near_plane_uv(
        &self,
        camera_transform: &GlobalTransform,
        projection: &Projection,
        world_point: Vec3,
    ) -> Option<Vec2>;

    /// Checks if a world point is within the near plane bounds.
    ///
    /// Returns true if the point, when projected onto the near plane,
    /// falls within the camera's viewport bounds.
    fn is_within_near_plane_bounds(
        &self,
        camera_transform: &GlobalTransform,
        projection: &Projection,
        world_point: Vec3,
    ) -> bool;
}

impl CameraExt for Camera {
    fn uv_to_near_plane_world(
        &self,
        camera_transform: &GlobalTransform,
        projection: &Projection,
        uv: Vec2,
    ) -> Option<Vec3> {
        let viewport_size = self.logical_viewport_size()?;
        let viewport_pos = uv * viewport_size;
        self.viewport_to_near_plane_world(camera_transform, projection, viewport_pos)
    }

    fn viewport_to_near_plane_world(
        &self,
        camera_transform: &GlobalTransform,
        projection: &Projection,
        viewport_pos: Vec2,
    ) -> Option<Vec3> {
        // Use Bevy's camera function to get a world-space ray
        let ray = self
            .viewport_to_world(camera_transform, viewport_pos)
            .ok()?;

        // Get near plane distance
        let (near, _, _) = self.near_plane_dimensions(projection)?;

        // Intersect ray with near plane
        let forward = camera_transform.forward();
        let t = near / ray.direction.dot(forward.as_vec3());

        Some(ray.get_point(t))
    }

    fn near_plane_dimensions(&self, projection: &Projection) -> Option<(f32, f32, f32)> {
        match projection {
            Projection::Perspective(proj) => {
                let near = proj.near;
                let half_height = near * (proj.fov * 0.5).tan();
                let half_width = half_height * proj.aspect_ratio;
                Some((near, half_width, half_height))
            }
            Projection::Orthographic(proj) => {
                let near = proj.near;
                let half_width = (proj.area.max.x - proj.area.min.x) * 0.5;
                let half_height = (proj.area.max.y - proj.area.min.y) * 0.5;
                Some((near, half_width, half_height))
            }
            _ => None,
        }
    }

    fn near_plane_center(
        &self,
        camera_transform: &GlobalTransform,
        projection: &Projection,
    ) -> Option<Vec3> {
        let (near, _, _) = self.near_plane_dimensions(projection)?;
        Some(camera_transform.translation() + camera_transform.forward() * near)
    }

    fn world_to_near_plane_uv(
        &self,
        camera_transform: &GlobalTransform,
        _projection: &Projection,
        world_point: Vec3,
    ) -> Option<Vec2> {
        // Use Bevy's world_to_viewport for the conversion
        // Note: projection is included in signature for API consistency but not used
        // since world_to_viewport handles the projection internally
        let viewport_pos = self.world_to_viewport(camera_transform, world_point).ok()?;
        let viewport_size = self.logical_viewport_size()?;

        // Convert viewport coordinates to UV [0,1]
        Some(Vec2::new(
            viewport_pos.x / viewport_size.x,
            viewport_pos.y / viewport_size.y,
        ))
    }

    fn is_within_near_plane_bounds(
        &self,
        camera_transform: &GlobalTransform,
        projection: &Projection,
        world_point: Vec3,
    ) -> bool {
        let Some((_, half_width, half_height)) = self.near_plane_dimensions(projection) else {
            return false;
        };

        // Convert world point to camera-local coordinates
        let camera_to_point = world_point - camera_transform.translation();
        let local_right = camera_transform.right().dot(camera_to_point);
        let local_up = camera_transform.up().dot(camera_to_point);

        // Check if within bounds
        local_right.abs() <= half_width && local_up.abs() <= half_height
    }
}

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
