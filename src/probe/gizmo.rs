use bevy::prelude::*;

use crate::probe::{
    events::ProbeHoverEvent,
    monitor::ProbeMonitor,
};

/// System to draw a crosshair gizmo at the intersection point
pub fn draw_intersection_gizmo(
    mut hover_events: EventReader<ProbeHoverEvent>,
    mut gizmos: Gizmos,
    monitor_query: Query<(&ProbeMonitor, &GlobalTransform)>,
) {
    if let Some(event) = hover_events.read().last() {
        for (monitor, transform) in monitor_query.iter() {
            if monitor.probe_entity == event.probe_entity {
                let cross_size = 0.05;
                let cross_color = Color::srgb(1.0, 0.0, 0.0);

                // Draw horizontal line
                gizmos.line(
                    event.world_position - transform.right() * cross_size,
                    event.world_position + transform.right() * cross_size,
                    cross_color,
                );
                
                // Draw vertical line
                gizmos.line(
                    event.world_position - transform.up() * cross_size,
                    event.world_position + transform.up() * cross_size,
                    cross_color,
                );
                
                break;
            }
        }
    }
}

/// System to draw depth information as a gizmo
pub fn draw_depth_gizmo(
    mut hover_events: EventReader<ProbeHoverEvent>,
    mut gizmos: Gizmos,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
) {
    if let Some(event) = hover_events.read().last() {
        if let Ok(camera_transform) = camera_query.single() {
            let camera_position = camera_transform.translation();
            let intersection_position = event.world_position;
            let depth = camera_position.distance(intersection_position);
            
            // Draw a line from camera to intersection point
            gizmos.line(
                camera_position,
                intersection_position,
                Color::srgb(0.0, 1.0, 0.0),
            );
            
            // Draw depth text (this would need a proper text rendering system)
            // For now, we'll just log the depth
            info!("Depth: {:.3}", depth);
        }
    }
} 