use bevy::{
    prelude::*,
    render::camera::RenderTarget,
    window::PrimaryWindow,
};

use crate::probe::{
    events::{ProbeClickEvent, ProbeHoverEvent},
    monitor::ProbeMonitor,
    state::ProbeState,
    utils::{intersect_ray_with_plane, world_to_texture_coords},
    ProbeSettings,
};

/// System to detect mouse interactions with probe monitors and emit events
pub fn emit_monitor_interaction_events(
    monitor_query: Query<(&ProbeMonitor, &GlobalTransform)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    probe_camera_query: Query<&Projection, With<crate::probe::Probe>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut hover_events: EventWriter<ProbeHoverEvent>,
    mut click_events: EventWriter<ProbeClickEvent>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query
        .iter()
        .find(|(cam, _)| matches!(cam.target, RenderTarget::Window(_)))
        .ok_or("No main camera found")
    else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    for (monitor, monitor_transform) in monitor_query.iter() {
        // Get the probe camera's projection to calculate the correct monitor size
        let Ok(projection) = probe_camera_query.get(monitor.probe_entity) else {
            continue;
        };
        
        let Projection::Perspective(perspective) = projection else {
            continue;
        };
        
        // Calculate the actual monitor size based on the probe camera's projection
        let near = perspective.near;
        let near_half_height = near * (perspective.fov / 2.0).tan();
        let near_half_width = near_half_height * perspective.aspect_ratio;
        let monitor_size = Vec2::new(near_half_width * 2.0, near_half_height * 2.0);
        
        if let Some(intersection) = intersect_ray_with_plane(ray, monitor_transform, monitor_size) {
            let texture_coords = world_to_texture_coords(intersection, monitor_transform, monitor_size);

            // Always emit hover event when hovering over a monitor
            hover_events.write(ProbeHoverEvent {
                probe_entity: monitor.probe_entity,
                texture_coords,
                world_position: intersection,
            });

            // Emit click event when left mouse button is pressed
            if mouse_button_input.just_pressed(MouseButton::Left) {
                click_events.write(ProbeClickEvent {
                    probe_entity: monitor.probe_entity,
                    texture_coords,
                });
            }
            
            // Only handle the first intersection
            return;
        }
    }
}

/// System to update probe settings when a monitor is hovered (temporary update)
pub fn update_probe_settings_on_hover(
    mut probe_query: Query<&mut ProbeSettings>,
    mut hover_events: EventReader<ProbeHoverEvent>,
) {
    for event in hover_events.read() {
        if let Ok(mut probe_settings) = probe_query.get_mut(event.probe_entity) {
            probe_settings.center_coords = event.texture_coords;
            // Don't log hover updates as they're continuous
        }
    }
}

/// System to update probe settings when a monitor is clicked (permanent update)
pub fn update_probe_settings_on_click(
    mut probe_query: Query<&mut ProbeSettings>,
    mut click_events: EventReader<ProbeClickEvent>,
) {
    for event in click_events.read() {
        if let Ok(mut probe_settings) = probe_query.get_mut(event.probe_entity) {
            probe_settings.center_coords = event.texture_coords;
            info!(
                "Updated probe center coordinates to: ({:.3}, {:.3})",
                event.texture_coords.x, event.texture_coords.y
            );
        }
    }
}

/// System to toggle the probing state with keyboard input
pub fn toggle_probing_state(
    mut next_state: ResMut<NextState<ProbeState>>,
    current_state: Res<State<ProbeState>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyP) {
        let new_state = match current_state.get() {
            ProbeState::Idle => ProbeState::Probing,
            ProbeState::Probing => ProbeState::Idle,
        };
        info!("Switching probe state to: {:?}", new_state);
        next_state.set(new_state);
    }
} 