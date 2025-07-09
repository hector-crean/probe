use bevy::prelude::*;

/// Event fired when the cursor is hovering over a probe monitor.
#[derive(Event)]
pub struct ProbeHoverEvent {
    /// The entity of the probe being hovered.
    pub probe_entity: Entity,
    /// The 2D texture coordinates of the hover position on the monitor.
    pub texture_coords: Vec2,
    /// The 3D world space position of the intersection.
    pub world_position: Vec3,
}

/// Event fired when a probe monitor is clicked.
#[derive(Event)]
pub struct ProbeClickEvent {
    /// The entity of the probe that was clicked.
    pub probe_entity: Entity,
    /// The 2D texture coordinates of the click position on the monitor.
    pub texture_coords: Vec2,
} 