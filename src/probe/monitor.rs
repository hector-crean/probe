use bevy::{
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};

use crate::probe::ProbeBindGroup;

/// Component that represents a monitor display for a probe camera
#[derive(Component)]
pub struct ProbeMonitor {
    pub probe_entity: Entity,
    pub is_hovered: bool,
}

impl ProbeMonitor {
    pub fn new(probe_entity: Entity) -> Self {
        Self {
            probe_entity,
            is_hovered: false,
        }
    }
}

/// System to create monitor entities for newly added probes
pub fn spawn_monitor_for_probe(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    probe_query: Query<(Entity, &ProbeBindGroup), Added<ProbeBindGroup>>,
) {
    for (probe_entity, bind_group) in probe_query.iter() {
        let monitor_entity = commands
            .spawn((
                ProbeMonitor::new(probe_entity),
                Mesh3d::from(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(1.0, 1.0)))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(bind_group.source_texture.clone()),
                    double_sided: true,
                    emissive: LinearRgba::WHITE,
                    unlit: true,
                    cull_mode: None,
                    ..default()
                })),
                NotShadowCaster,
                NotShadowReceiver,
            ))
            .id();

        commands.entity(probe_entity).add_child(monitor_entity);
    }
}

/// System to update monitor materials when hovered
pub fn update_monitor_highlight(
    mut last_hovered_probe: Local<Option<Entity>>,
    mut hover_events: EventReader<crate::probe::events::ProbeHoverEvent>,
    monitor_query: Query<(&ProbeMonitor, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(event) = hover_events.read().last() {
        let currently_hovered = Some(event.probe_entity);

        if currently_hovered != *last_hovered_probe {
            // Reset previously hovered monitor
            if let Some(last_entity) = *last_hovered_probe {
                for (monitor, material_handle) in monitor_query.iter() {
                    if monitor.probe_entity == last_entity {
                        if let Some(material) = materials.get_mut(material_handle) {
                            material.emissive = LinearRgba::WHITE;
                        }
                        break;
                    }
                }
            }
            
            // Highlight new monitor
            for (monitor, material_handle) in monitor_query.iter() {
                if monitor.probe_entity == event.probe_entity {
                    if let Some(material) = materials.get_mut(material_handle) {
                        material.emissive = LinearRgba::new(1.2, 1.2, 1.2, 1.0);
                    }
                    break;
                }
            }

            *last_hovered_probe = currently_hovered;
        }
    } else {
        // Reset when nothing is hovered
        if let Some(last_entity) = last_hovered_probe.take() {
            for (monitor, material_handle) in monitor_query.iter() {
                if monitor.probe_entity == last_entity {
                    if let Some(material) = materials.get_mut(material_handle) {
                        material.emissive = LinearRgba::WHITE;
                    }
                    break;
                }
            }
        }
    }
} 