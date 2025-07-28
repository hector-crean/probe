//! This example demonstrates how to visualize a camera's frustum and calculate
//! the intersection of the mouse pointer with the camera's near plane.
//!
//! The example shows:
//! - How to extract frustum vertices from a camera's view-projection matrix
//! - How to draw a wireframe frustum using gizmos
//! - How to use `Camera::viewport_to_world` to cast a ray from screen coordinates
//! - How to calculate the intersection point with the near plane
//! - How to visualize the intersection point and ray direction
//!
//! Controls:
//! - Move camera with WASD and mouse (controls the frustum camera)
//! - The frustum wireframe will update in real-time
//! - Move the mouse to see the intersection point on the near plane

use bevy::{
    color::palettes::css::*,
    gizmos::config::{GizmoConfigGroup, GizmoConfigStore},
    math::Ray3d,
    prelude::*,
    render::{camera::Projection, view::RenderLayers},
};

use crate::{
    probe_tool::{probe_pipeline::KernelBindGroup, KernelSettings, ProbeCamera}, MainCamera
};

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct MainCameraGizmos;

pub struct FrustumNearPlaneInteractionPlugin;

fn setup_main_camera_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<MainCameraGizmos>();
    config.render_layers = RenderLayers::layer(0);
}

impl Plugin for FrustumNearPlaneInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<MainCameraGizmos>()
            .add_systems(Startup, setup_main_camera_gizmos)
            .add_systems(
                Update,
                (
                    update_near_plane_intersection,
                    draw_near_plane_visualization,
                )
                    .chain(),
            );
    }
}

/// Component to store the near plane intersection point
#[derive(Component, Default)]
pub struct FrustumNearPlaneIntersection {
    point: Option<Vec3>,
    ray: Option<Ray3d>,
    is_within_bounds: bool,
    intersection_world_point: Option<Vec3>, // Store the actual intersection point even if out of bounds
}

/// System to calculate and update intersection data
fn update_near_plane_intersection(
    window: Single<&Window>,
    mut frustrum_camera_query: Query<
        (
            &Camera,
            &GlobalTransform,
            &mut FrustumNearPlaneIntersection,
            &Projection,
            &mut KernelBindGroup,
        ),
        (With<ProbeCamera>, Without<MainCamera>),
    >,
    main_camera_query: Query<
        (&Camera, &GlobalTransform, &Projection),
        (With<MainCamera>, Without<ProbeCamera>),
    >,
) {
    let Ok((main_camera, main_camera_transform, _main_projection)) = main_camera_query.single()
    else {
        return;
    };

    let Ok((
        frustrum_camera,
        frustrum_camera_transform,
        mut frustrum_intersection,
        frustrum_projection,
        mut kernel_bind_group,
    )) = frustrum_camera_query.single_mut()
    else {
        return;
    };

    // Get cursor position
    let Some(cursor_position) = window.cursor_position() else {
        frustrum_intersection.point = None;
        frustrum_intersection.ray = None;
        frustrum_intersection.is_within_bounds = false;
        frustrum_intersection.intersection_world_point = None;
        return;
    };

    // Convert cursor position to world ray
    let Ok(main_camera_ray) = main_camera.viewport_to_world(main_camera_transform, cursor_position)
    else {
        frustrum_intersection.point = None;
        frustrum_intersection.ray = None;
        frustrum_intersection.is_within_bounds = false;
        frustrum_intersection.intersection_world_point = None;
        return;
    };

    // Calculate the near plane
    let frustum_camera_forward = frustrum_camera_transform.forward();
    let near_distance = match frustrum_projection {
        Projection::Perspective(proj) => proj.near,
        Projection::Orthographic(proj) => proj.near,
        _ => 0.0,
    };

    let near_plane_center =
        frustrum_camera_transform.translation() + frustum_camera_forward * near_distance;
    let near_plane = InfinitePlane3d::new(frustum_camera_forward.as_vec3());

    // Calculate intersection with near plane
    if let Some(distance) = main_camera_ray.intersect_plane(near_plane_center, near_plane) {
        let intersection_point = main_camera_ray.get_point(distance);
        frustrum_intersection.intersection_world_point = Some(intersection_point);

        // Check if intersection point is within the finite near plane bounds
        let viewport_size = frustrum_camera
            .logical_viewport_size()
            .unwrap_or(Vec2::new(800.0, 600.0));
        let aspect_ratio = viewport_size.x / viewport_size.y;

        let is_within_bounds = match frustrum_projection {
            Projection::Perspective(proj) => {
                // Calculate near plane dimensions for perspective projection
                let half_height = near_distance * (proj.fov * 0.5).tan();
                let half_width = half_height * aspect_ratio;

                // Convert world intersection point to camera-local coordinates
                let camera_to_intersection =
                    intersection_point - frustrum_camera_transform.translation();
                let local_right = frustrum_camera_transform
                    .right()
                    .dot(camera_to_intersection);
                let local_up = frustrum_camera_transform.up().dot(camera_to_intersection);

                // Check bounds
                local_right.abs() <= half_width && local_up.abs() <= half_height
            }
            Projection::Orthographic(proj) => {
                // For orthographic projection, use scale with aspect ratio
                let half_height = proj.scale * 0.5;
                let half_width = half_height * aspect_ratio;

                // Convert world intersection point to camera-local coordinates
                let camera_to_intersection =
                    intersection_point - frustrum_camera_transform.translation();
                let local_right = frustrum_camera_transform
                    .right()
                    .dot(camera_to_intersection);
                let local_up = frustrum_camera_transform.up().dot(camera_to_intersection);

                // Check bounds
                local_right.abs() <= half_width && local_up.abs() <= half_height
            }
            _ => true, // Default to true for other projection types
        };

        frustrum_intersection.is_within_bounds = is_within_bounds;
        frustrum_intersection.ray = Some(main_camera_ray);

        if is_within_bounds {
            // Store the valid intersection data
            frustrum_intersection.point = Some(intersection_point);

            // Update kernel settings
            let frutum_viewport_position = match frustrum_camera
                .world_to_viewport(frustrum_camera_transform, intersection_point)
            {
                Ok(position) => position,
                Err(_) => {
                    frustrum_intersection.point = None;
                    frustrum_intersection.ray = None;
                    frustrum_intersection.is_within_bounds = false;
                    return;
                }
            };

            kernel_bind_group.settings.center_coords = Vec2::new(
                frutum_viewport_position.x / viewport_size.x,
                frutum_viewport_position.y / viewport_size.y,
            );
        } else {
            // Intersection is outside the finite plane bounds
            frustrum_intersection.point = None;
        }
    } else {
        frustrum_intersection.point = None;
        frustrum_intersection.ray = None;
        frustrum_intersection.is_within_bounds = false;
        frustrum_intersection.intersection_world_point = None;
    }
}

/// System to draw visualization based on intersection data
fn draw_near_plane_visualization(
    mut gizmos: Gizmos<MainCameraGizmos>,
    frustrum_camera_query: Query<
        (&Camera, &GlobalTransform, &FrustumNearPlaneIntersection),
        (With<ProbeCamera>, Without<MainCamera>),
    >,
) {
    let Ok((frustrum_camera, frustrum_camera_transform, frustrum_intersection)) =
        frustrum_camera_query.single()
    else {
        return;
    };

    // Draw the valid intersection point (within bounds)
    if let Some(intersection_point) = frustrum_intersection.point {
        // Main intersection sphere
        gizmos.sphere(intersection_point, 0.05, RED);

        // Line from camera to intersection
        gizmos.line(
            frustrum_camera_transform.translation(),
            intersection_point,
            RED,
        );

        // Draw a small cross at the intersection point
        let cross_size = 0.1;
        let camera_right = frustrum_camera_transform.right() * cross_size;
        let camera_up = frustrum_camera_transform.up() * cross_size;

        gizmos.line(
            intersection_point - camera_right,
            intersection_point + camera_right,
            RED,
        );
        gizmos.line(
            intersection_point - camera_up,
            intersection_point + camera_up,
            RED,
        );

        // Draw the frustum camera ray from intersection point
        let viewport_size = frustrum_camera
            .logical_viewport_size()
            .unwrap_or(Vec2::new(800.0, 600.0));
        if let Ok(frutum_viewport_position) =
            frustrum_camera.world_to_viewport(frustrum_camera_transform, intersection_point)
        {
            if let Ok(frustum_camera_ray) = frustrum_camera
                .viewport_to_world(frustrum_camera_transform, frutum_viewport_position)
            {
                gizmos.line(
                    intersection_point,
                    intersection_point + frustum_camera_ray.direction * 10.0,
                    Color::srgba(0.0, 0.0, 1.0, 0.5),
                );
            }
        }
    }

    // Draw out-of-bounds intersection with different visualization
    if !frustrum_intersection.is_within_bounds {
        if let Some(intersection_point) = frustrum_intersection.intersection_world_point {
            gizmos.sphere(intersection_point, 0.03, Color::srgba(1.0, 0.5, 0.5, 0.5));
        }
    }
}
