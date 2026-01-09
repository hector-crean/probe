//! Components for frustum visualization.

use bevy::{math::Ray3d, prelude::*};

/// Marker component for frustum wireframe entities.
#[derive(Component)]
pub struct FrustumMeshMarker;

/// Marker component for near plane entities.
#[derive(Component)]
pub struct NearPlaneMarker;

/// Component that stores references to child entities for easy management.
#[derive(Component, Default)]
pub struct ProbeVisualizationChildren {
    pub frustum_entity: Option<Entity>,
    pub near_plane_entity: Option<Entity>,
}

/// Component to store the near plane intersection point.
#[derive(Component, Default)]
pub struct FrustumNearPlaneIntersection {
    /// Current hover intersection point (if within bounds).
    pub point: Option<Vec3>,
    /// The ray from main camera through cursor.
    pub ray: Option<Ray3d>,
    /// Whether the hover point is within the near plane bounds.
    pub is_within_bounds: bool,
    /// The intersection point even if outside bounds (for visualization).
    pub intersection_world_point: Option<Vec3>,
}
