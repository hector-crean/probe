use bevy::{
    ecs::system::ParamSet,
    prelude::*,
};

use crate::probe::{
    frustum::{spawn_frustum_for_probe, ProbeFrustum, ProbeFrustumMeshBuilder},
    gizmo::draw_intersection_gizmo,
    interaction::{ toggle_probing_state},
    monitor::{spawn_monitor_for_probe, update_monitor_highlight, ProbeMonitor},
    state::ProbeState,
    ProbeCamera,
};

pub struct ProbeVisualizationPlugin;

impl Plugin for ProbeVisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                // Setup systems for new probes
                spawn_monitor_for_probe,
                spawn_frustum_for_probe,
                // Interaction systems
                // emit_monitor_interaction_events.run_if(in_state(ProbeState::Probing)),
                toggle_probing_state,
                // Visual feedback systems
                update_monitor_highlight,
                draw_intersection_gizmo,
                // Update systems for changed probes
                update_probe_visualizations,
            ),
        );
    }
}

/// Updates the visualization meshes and material if the probe's `Projection` or `Camera` component changes.
fn update_probe_visualizations(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    probe_query: Query<
        (&Projection, &Camera, &Children),
        (With<ProbeCamera>, Or<(Changed<Projection>, Changed<Camera>)>),
    >,
    mut visual_query: ParamSet<(
        Query<
            (
                &mut Mesh3d,
                &mut Transform,
                &MeshMaterial3d<StandardMaterial>,
            ),
            With<ProbeMonitor>,
        >,
        Query<&mut Mesh3d, With<ProbeFrustum>>,
    )>,
) {
    for (projection, camera, children) in &probe_query {
        let Projection::Perspective(perspective) = projection else {
            continue;
        };

        let near = perspective.near;
        let near_half_height = near * (perspective.fov / 2.0).tan();
        let near_half_width = near_half_height * perspective.aspect_ratio;

        let new_monitor_mesh = meshes.add(Plane3d::new(
            Vec3::Z,
            Vec2::new(near_half_width, near_half_height),
        ));

        let frustum_mesh_builder = ProbeFrustumMeshBuilder::from_perspective_projection(perspective);
        let new_wireframe_mesh = meshes.add(frustum_mesh_builder.build());

        let new_texture = camera.target.as_image().cloned();

        for &child in children {
            // Update the monitor's mesh, transform, and material texture
            if let Ok((mut mesh_handle, mut transform, material_handle)) =
                visual_query.p0().get_mut(child)
            {
                *mesh_handle = Mesh3d::from(new_monitor_mesh.clone());
                transform.translation = Vec3::new(0.0, 0.0, -near);

                if let Some(material) = materials.get_mut(material_handle) {
                    material.base_color_texture = new_texture.clone();
                }
            }
            // Update the frustum's mesh
            if let Ok(mut mesh_handle) = visual_query.p1().get_mut(child) {
                *mesh_handle = Mesh3d::from(new_wireframe_mesh.clone());
            }
        }
    }
}
