//! Near plane interaction for probe camera frustum visualization.
//!
//! This module provides:
//! - Click/drag interaction on the probe's near plane to control sampling point
//! - Ray-plane intersection calculation from main camera through cursor
//! - Visual feedback showing the current sampling point
//!
//! Controls:
//! - Click and drag on the near plane to set/update the sampling point
//! - The sampling point is shown with a green crosshair and probe ray
//! - Hover point shown with red indicator when different from locked point
//! - Out-of-bounds hover is shown with a faded indicator

use bevy::{
    color::palettes::css::*,
    gizmos::config::{GizmoConfigGroup, GizmoConfigStore},
    input::mouse::MouseButton,
    prelude::*,
};

use crate::{
    camera::{CameraExt, MainCamera},
    probe::{
        components::ProbeCamera, frustum::FrustumNearPlaneIntersection, pipeline::KernelBindGroup,
    },
};

// =============================================================================
// Components & Resources
// =============================================================================

/// Gizmo group for main camera visualization.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct MainCameraGizmos;

/// Resource to track drag state for near plane interaction.
#[derive(Resource, Default)]
pub struct NearPlaneDragState {
    /// Whether the user is currently dragging on the near plane.
    pub is_dragging: bool,
    /// Normalized UV coordinates [0,1] of the sampling point on the near plane.
    /// The world position is computed from this each frame.
    pub locked_uv: Option<Vec2>,
}

/// Mouse button state for drag handling.
#[derive(Clone, Copy)]
enum DragEvent {
    /// Left mouse just pressed while within bounds
    Started,
    /// Left mouse just released
    Ended,
    /// Actively dragging with left mouse held
    Dragging,
    /// No drag activity
    Idle,
}

impl DragEvent {
    fn from_input(mouse: &ButtonInput<MouseButton>, is_within_bounds: bool) -> Self {
        let pressed = mouse.pressed(MouseButton::Left);
        let just_pressed = mouse.just_pressed(MouseButton::Left);
        let just_released = mouse.just_released(MouseButton::Left);

        match (just_pressed, just_released, pressed, is_within_bounds) {
            (true, _, _, true) => DragEvent::Started,
            (_, true, _, _) => DragEvent::Ended,
            (_, _, true, _) => DragEvent::Dragging,
            _ => DragEvent::Idle,
        }
    }
}

// =============================================================================
// Plugin
// =============================================================================

/// Plugin for near plane interaction.
pub struct FrustumNearPlaneInteractionPlugin;

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
                    .chain()
                    .run_if(any_with_component::<ProbeCamera>),
            );
    }
}

fn setup_main_camera_gizmos(mut config_store: ResMut<GizmoConfigStore>) {
    let (_config, _) = config_store.config_mut::<MainCameraGizmos>();
    // Gizmos configuration - can be customized if needed
}

// =============================================================================
// Systems
// =============================================================================

/// System to calculate intersection data (hover detection only, doesn't update sampling point).
fn update_near_plane_intersection(
    window: Single<&Window>,
    mut probe_camera_query: Query<
        (
            &Camera,
            &GlobalTransform,
            &mut FrustumNearPlaneIntersection,
            &Projection,
        ),
        (With<ProbeCamera>, Without<MainCamera>),
    >,
    main_camera_query: Query<(&Camera, &GlobalTransform), (With<MainCamera>, Without<ProbeCamera>)>,
) {
    // Early return with let-else for required entities
    let Ok((main_camera, main_camera_transform)) = main_camera_query.single() else {
        return;
    };

    let Ok((probe_camera, probe_transform, mut intersection, projection)) =
        probe_camera_query.single_mut()
    else {
        return;
    };

    // Get cursor position - reset state if no cursor
    let Some(cursor_position) = window.cursor_position() else {
        *intersection = FrustumNearPlaneIntersection::default();
        return;
    };

    // Convert cursor position to world ray
    let Ok(camera_ray) = main_camera.viewport_to_world(main_camera_transform, cursor_position)
    else {
        *intersection = FrustumNearPlaneIntersection::default();
        return;
    };

    // Get near plane center using CameraExt
    let Some(near_plane_center) = probe_camera.near_plane_center(probe_transform, projection)
    else {
        *intersection = FrustumNearPlaneIntersection::default();
        return;
    };

    // Calculate plane intersection
    let plane_normal = probe_transform.forward();
    let near_plane = InfinitePlane3d::new(plane_normal.as_vec3());

    // Use match for the intersection result - clearer than if-else
    match camera_ray.intersect_plane(near_plane_center, near_plane) {
        Some(distance) => {
            let point = camera_ray.get_point(distance);
            let is_within_bounds =
                probe_camera.is_within_near_plane_bounds(probe_transform, projection, point);

            *intersection = FrustumNearPlaneIntersection {
                point: is_within_bounds.then_some(point),
                ray: Some(camera_ray),
                is_within_bounds,
                intersection_world_point: Some(point),
            };
        }
        None => {
            *intersection = FrustumNearPlaneIntersection::default();
        }
    }
}

/// System to handle click/drag interaction on the near plane.
/// Updates the kernel sampling point only when user is clicking/dragging.
fn handle_near_plane_click_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut drag_state: ResMut<NearPlaneDragState>,
    mut probe_camera_query: Query<
        (
            &Camera,
            &GlobalTransform,
            &Projection,
            &FrustumNearPlaneIntersection,
            &mut KernelBindGroup,
        ),
        With<ProbeCamera>,
    >,
) {
    let Ok((camera, transform, projection, intersection, mut kernel_bind_group)) =
        probe_camera_query.single_mut()
    else {
        return;
    };

    let drag_event = DragEvent::from_input(&mouse_button, intersection.is_within_bounds);

    // Handle drag state transitions with match
    match drag_event {
        DragEvent::Started => {
            drag_state.is_dragging = true;
        }
        DragEvent::Ended => {
            drag_state.is_dragging = false;
        }
        DragEvent::Dragging if drag_state.is_dragging => {
            // Update sampling point while dragging
            if let Some(point) = intersection.point {
                if let Some(uv) = camera.world_to_near_plane_uv(transform, projection, point) {
                    kernel_bind_group.settings.center_coords = uv;
                    drag_state.locked_uv = Some(uv);
                }
            }
        }
        DragEvent::Dragging | DragEvent::Idle => {}
    }
}

/// System to draw visualization based on intersection and drag state.
fn draw_near_plane_visualization(
    mut gizmos: Gizmos<MainCameraGizmos>,
    drag_state: Res<NearPlaneDragState>,
    probe_camera_query: Query<
        (
            &Camera,
            &GlobalTransform,
            &Projection,
            &FrustumNearPlaneIntersection,
        ),
        With<ProbeCamera>,
    >,
) {
    let Ok((camera, transform, projection, intersection)) = probe_camera_query.single() else {
        return;
    };

    // Get near plane dimensions for proportional sizing
    let (near, _, half_height) = camera
        .near_plane_dimensions(projection)
        .unwrap_or((1.0, 0.5, 0.5));

    // Draw locked sampling point if set
    if let Some(uv) = drag_state.locked_uv {
        draw_locked_point(
            &mut gizmos,
            camera,
            transform,
            projection,
            uv,
            near,
            half_height,
        );
    }

    // Draw hover visualization based on state
    draw_hover_visualization(
        &mut gizmos,
        camera,
        transform,
        projection,
        intersection,
        &drag_state,
        half_height,
    );

    // Draw drag indicator when actively dragging
    if drag_state.is_dragging {
        if let Some(point) = intersection.point {
            draw_drag_indicator(&mut gizmos, transform, point, half_height);
        }
    }
}

// =============================================================================
// Visualization Helpers
// =============================================================================

/// Draws the locked sampling point with crosshair and probe ray.
fn draw_locked_point(
    gizmos: &mut Gizmos<MainCameraGizmos>,
    camera: &Camera,
    transform: &GlobalTransform,
    projection: &Projection,
    uv: Vec2,
    near: f32,
    half_height: f32,
) {
    // Compute world position from UV + current transform (moves with frustum)
    let Some(world_point) = camera.uv_to_near_plane_world(transform, projection, uv) else {
        return;
    };

    // Line from camera origin to sampling point on near plane
    gizmos.line(transform.translation(), world_point, GREEN);

    // Draw 2D cross flat on the near plane (proportional to near plane size)
    let cross_size = half_height * 0.1;
    let right = transform.right() * cross_size;
    let up = transform.up() * cross_size;

    gizmos.line(world_point - right, world_point + right, GREEN);
    gizmos.line(world_point - up, world_point + up, GREEN);

    // Draw the probe ray from sampling point into the scene
    let forward = transform.forward();
    let ray_length = near * 5.0;
    gizmos.line(
        world_point,
        world_point + forward * ray_length,
        Color::srgba(0.0, 1.0, 0.0, 0.7),
    );
}

/// Draws hover visualization based on current state.
fn draw_hover_visualization(
    gizmos: &mut Gizmos<MainCameraGizmos>,
    camera: &Camera,
    transform: &GlobalTransform,
    projection: &Projection,
    intersection: &FrustumNearPlaneIntersection,
    drag_state: &NearPlaneDragState,
    half_height: f32,
) {
    // Determine what hover visualization to show using match
    match (
        intersection.is_within_bounds,
        intersection.point,
        intersection.intersection_world_point,
    ) {
        // Within bounds - show hover point if different from locked
        (true, Some(hover_point), _) => {
            let should_draw =
                should_draw_hover_point(camera, transform, projection, drag_state, hover_point);

            if should_draw {
                draw_hover_point(gizmos, transform, hover_point, half_height);
            }
        }
        // Out of bounds - show faded indicator
        (false, _, Some(out_of_bounds_point)) => {
            gizmos.sphere(
                out_of_bounds_point,
                half_height * 0.02,
                Color::srgba(1.0, 0.5, 0.5, 0.3),
            );
        }
        // No intersection - nothing to draw
        _ => {}
    }
}

/// Determines if hover point should be drawn (different from locked point).
fn should_draw_hover_point(
    camera: &Camera,
    transform: &GlobalTransform,
    projection: &Projection,
    drag_state: &NearPlaneDragState,
    hover_point: Vec3,
) -> bool {
    match drag_state.locked_uv {
        None => true,
        Some(uv) => camera
            .uv_to_near_plane_world(transform, projection, uv)
            .map_or(true, |locked| hover_point.distance(locked) > 0.01),
    }
}

/// Draws the hover point indicator.
fn draw_hover_point(
    gizmos: &mut Gizmos<MainCameraGizmos>,
    transform: &GlobalTransform,
    point: Vec3,
    half_height: f32,
) {
    // Hover point sphere (red, smaller)
    gizmos.sphere(point, half_height * 0.04, RED);

    // Small cross at hover point (proportional to near plane)
    let cross_size = half_height * 0.08;
    let right = transform.right() * cross_size;
    let up = transform.up() * cross_size;

    gizmos.line(point - right, point + right, RED);
    gizmos.line(point - up, point + up, RED);
}

/// Draws the drag indicator ring.
fn draw_drag_indicator(
    gizmos: &mut Gizmos<MainCameraGizmos>,
    transform: &GlobalTransform,
    point: Vec3,
    half_height: f32,
) {
    let rotation = Quat::from_rotation_arc(Vec3::Z, transform.forward().as_vec3());
    let isometry = Isometry3d::new(point, rotation);
    gizmos.circle(isometry, half_height * 0.15, YELLOW);
}
