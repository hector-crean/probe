//! Near plane interaction for probe camera frustum visualization.
//!
//! This module provides:
//! - Click/drag interaction on the probe's near plane to control sampling point
//! - Ray-plane intersection calculation from main camera through cursor
//! - Visual feedback showing the current sampling point
//!
//! Controls:
//! - Click and drag on the near plane to set/update the sampling point
//! - The sampling point is shown with a red sphere and crosshair
//! - Out-of-bounds hover is shown with a faded indicator

use bevy::{
    color::palettes::css::*,
    gizmos::config::{GizmoConfigGroup, GizmoConfigStore},
    input::mouse::MouseButton,
    prelude::*,
};

use crate::{camera::MainCamera, probe::{components::ProbeCamera, frustum::FrustumNearPlaneIntersection, pipeline::KernelBindGroup}};

/// Gizmo group for main camera visualization.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct MainCameraGizmos;

/// Resource to track drag state for near plane interaction.
#[derive(Resource, Default)]
pub struct NearPlaneDragState {
    /// Whether the user is currently dragging on the near plane.
    pub is_dragging: bool,
    /// The last valid sampling point (persists after drag ends).
    pub locked_point: Option<Vec3>,
    /// Normalized UV coordinates of the locked sampling point.
    pub locked_uv: Option<Vec2>,
}

/// Plugin for near plane interaction.
pub struct FrustumNearPlaneInteractionPlugin;

fn setup_main_camera_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (_config, _) = config_store.config_mut::<MainCameraGizmos>();
    // Gizmos configuration - can be customized if needed
}

impl Plugin for FrustumNearPlaneInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<MainCameraGizmos>()
            .init_resource::<NearPlaneDragState>()
            .add_systems(Startup, setup_main_camera_gizmos)
            .add_systems(
                Update,
                (
                    update_near_plane_intersection,
                    handle_near_plane_click_drag,
                    draw_near_plane_visualization,
                )
                    .chain(),
            );
    }
}



/// System to calculate intersection data (hover detection only, doesn't update sampling point).
fn update_near_plane_intersection(
    window: Single<&Window>,
    mut frustrum_camera_query: Query<
        (
            &Camera,
            &GlobalTransform,
            &mut FrustumNearPlaneIntersection,
            &Projection,
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
                let local_right = frustrum_camera_transform.right().dot(camera_to_intersection);
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
                let local_right = frustrum_camera_transform.right().dot(camera_to_intersection);
                let local_up = frustrum_camera_transform.up().dot(camera_to_intersection);

                // Check bounds
                local_right.abs() <= half_width && local_up.abs() <= half_height
            }
            _ => true, // Default to true for other projection types
        };

        frustrum_intersection.is_within_bounds = is_within_bounds;
        frustrum_intersection.ray = Some(main_camera_ray);

        if is_within_bounds {
            frustrum_intersection.point = Some(intersection_point);
        } else {
            frustrum_intersection.point = None;
        }
    } else {
        frustrum_intersection.point = None;
        frustrum_intersection.ray = None;
        frustrum_intersection.is_within_bounds = false;
        frustrum_intersection.intersection_world_point = None;
    }
}

/// System to handle click/drag interaction on the near plane.
/// Updates the kernel sampling point only when user is clicking/dragging.
fn handle_near_plane_click_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut drag_state: ResMut<NearPlaneDragState>,
    mut frustrum_camera_query: Query<
        (
            &Camera,
            &GlobalTransform,
            &FrustumNearPlaneIntersection,
            &mut KernelBindGroup,
        ),
        With<ProbeCamera>,
    >,
) {
    let Ok((frustrum_camera, frustrum_camera_transform, frustrum_intersection, mut kernel_bind_group)) =
        frustrum_camera_query.single_mut()
    else {
        return;
    };

    let left_pressed = mouse_button.pressed(MouseButton::Left);
    let left_just_pressed = mouse_button.just_pressed(MouseButton::Left);
    let left_just_released = mouse_button.just_released(MouseButton::Left);

    // Handle drag start
    if left_just_pressed && frustrum_intersection.is_within_bounds {
        drag_state.is_dragging = true;
    }

    // Handle drag end
    if left_just_released {
        drag_state.is_dragging = false;
    }

    // Update sampling point while dragging (or on click)
    if drag_state.is_dragging && left_pressed {
        if let Some(intersection_point) = frustrum_intersection.point {
            let viewport_size = frustrum_camera
                .logical_viewport_size()
                .unwrap_or(Vec2::new(800.0, 600.0));

            if let Ok(frustum_viewport_position) =
                frustrum_camera.world_to_viewport(frustrum_camera_transform, intersection_point)
            {
                // Convert viewport coordinates to texture UV coordinates.
                // Viewport: (0,0) at top-left, Y increases downward
                // Texture:  (0,0) at top-left for render targets in Bevy/WGPU
                // However, the render target is rendered with camera looking "into" the scene,
                // so we need to invert Y to match the visual near plane orientation.
                let uv = Vec2::new(
                    frustum_viewport_position.x / viewport_size.x,
                    1.0 - (frustum_viewport_position.y / viewport_size.y),
                );

                // Update kernel sampling center
                kernel_bind_group.settings.center_coords = uv;

                // Store the locked point for visualization
                drag_state.locked_point = Some(intersection_point);
                drag_state.locked_uv = Some(uv);
            }
        }
    }
}

/// System to draw visualization based on intersection and drag state.
fn draw_near_plane_visualization(
    mut gizmos: Gizmos<MainCameraGizmos>,
    drag_state: Res<NearPlaneDragState>,
    frustrum_camera_query: Query<
        (&Camera, &GlobalTransform, &FrustumNearPlaneIntersection),
        With<ProbeCamera>,
    >,
) {
    let Ok((frustrum_camera, frustrum_camera_transform, frustrum_intersection)) =
        frustrum_camera_query.single()
    else {
        return;
    };

    // Draw the locked sampling point (from click/drag) - this is the active sampling point
    if let Some(locked_point) = drag_state.locked_point {
        // Main sampling point sphere (green to indicate it's the active point)
        gizmos.sphere(locked_point, 0.06, GREEN);

        // Line from camera to sampling point
        gizmos.line(
            frustrum_camera_transform.translation(),
            locked_point,
            GREEN,
        );

        // Draw a crosshair at the sampling point
        let cross_size = 0.12;
        let camera_right = frustrum_camera_transform.right() * cross_size;
        let camera_up = frustrum_camera_transform.up() * cross_size;

        gizmos.line(
            locked_point - camera_right,
            locked_point + camera_right,
            GREEN,
        );
        gizmos.line(locked_point - camera_up, locked_point + camera_up, GREEN);

        // Draw the probe ray from sampling point into the scene
        if let Ok(frustum_viewport_position) =
            frustrum_camera.world_to_viewport(frustrum_camera_transform, locked_point)
        {
            if let Ok(frustum_camera_ray) = frustrum_camera
                .viewport_to_world(frustrum_camera_transform, frustum_viewport_position)
            {
                gizmos.line(
                    locked_point,
                    locked_point + frustum_camera_ray.direction * 10.0,
                    Color::srgba(0.0, 1.0, 0.0, 0.7),
                );
            }
        }
    }

    // Draw the current hover point (different from locked point)
    if let Some(hover_point) = frustrum_intersection.point {
        // Only draw hover indicator if it's different from the locked point
        let should_draw_hover = drag_state
            .locked_point
            .map(|locked| hover_point.distance(locked) > 0.01)
            .unwrap_or(true);

        if should_draw_hover {
            // Hover point sphere (red, smaller)
            gizmos.sphere(hover_point, 0.04, RED);

            // Small cross at hover point
            let cross_size = 0.08;
            let camera_right = frustrum_camera_transform.right() * cross_size;
            let camera_up = frustrum_camera_transform.up() * cross_size;

            gizmos.line(hover_point - camera_right, hover_point + camera_right, RED);
            gizmos.line(hover_point - camera_up, hover_point + camera_up, RED);
        }
    }

    // Draw out-of-bounds hover with faded visualization
    if !frustrum_intersection.is_within_bounds {
        if let Some(intersection_point) = frustrum_intersection.intersection_world_point {
            gizmos.sphere(intersection_point, 0.02, Color::srgba(1.0, 0.5, 0.5, 0.3));
        }
    }

    // Draw drag indicator when actively dragging
    if drag_state.is_dragging {
        if let Some(point) = frustrum_intersection.point {
            // Animated ring around the drag point
            // Create an isometry (position + rotation) for the circle
            let rotation =
                Quat::from_rotation_arc(Vec3::Z, frustrum_camera_transform.forward().as_vec3());
            let isometry = Isometry3d::new(point, rotation);
            gizmos.circle(isometry, 0.15, YELLOW);
        }
    }
}
