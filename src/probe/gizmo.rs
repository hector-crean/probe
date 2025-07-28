use bevy::prelude::*;

use crate::probe::{
    events::ProbeHoverEvent, monitor::ProbeMonitor, near_plane::MainCameraGizmos
};

/// System to draw a crosshair gizmo at the intersection point
pub fn draw_intersection_gizmo(
    mut hover_events: EventReader<ProbeHoverEvent>,
    mut gizmos: Gizmos<MainCameraGizmos>,
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

