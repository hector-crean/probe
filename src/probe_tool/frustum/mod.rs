pub mod mesh_builder;
pub mod near_plane_interaction;
use bevy::prelude::*;

use crate::probe_tool::{
    KernelBindGroup, ProbeCamera,
    frustum::mesh_builder::{ProbeFrustum, ProbeFrustumMeshBuilder},
};

/// Marker component for frustum wireframe entities
#[derive(Component)]
pub struct FrustumMeshMarker;

/// Marker component for near plane entities
#[derive(Component)]
pub struct NearPlaneMarker;

/// Component that stores references to child entities for easy management
#[derive(Component)]
#[derive(Default)]
pub struct ProbeVisualizationChildren {
    pub frustum_entity: Option<Entity>,
    pub near_plane_entity: Option<Entity>,
}


pub struct FrustumPlugin;

impl Plugin for FrustumPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(near_plane_interaction::FrustumNearPlaneInteractionPlugin);
        app.add_systems(Update, (draw_frustum, update_probe_visualizations));
    }
}

pub fn draw_frustum(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    probe_query: Query<
        (Entity, &Projection, &KernelBindGroup),
        Added<ProbeCamera>,
    >,
) {
    for (probe_entity, projection, kernel_bind_group) in probe_query.iter() {
        let perspective = match projection {
            Projection::Perspective(perspective) => perspective,
            _ => continue,
        };

        let frustum = ProbeFrustum::new(perspective.clone());
        let mesh = meshes.add(frustum.mesh().build());

        // Calculate near plane dimensions from perspective projection
        let near = perspective.near;
        let near_half_height = near * (perspective.fov / 2.0).tan();
        let near_half_width = near_half_height * perspective.aspect_ratio;

        // Spawn frustum wireframe with marker component
        let frustum_entity = commands
            .spawn((
                FrustumMeshMarker,
                Mesh3d::from(mesh),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 1.0, 0.0, 1.0),
                    unlit: true,
                    ..default()
                })),
            ))
            .id();

        // Spawn near plane with marker component
        // Use -Vec3::Z normal so the plane faces towards the camera origin
        // Position at -near on Z axis (in front of camera, which looks down -Z)
        let near_plane_entity = commands
            .spawn((
                NearPlaneMarker,
                Mesh3d::from(meshes.add(Plane3d::new(-Vec3::Z, Vec2::new(near_half_width, near_half_height)))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(kernel_bind_group.source_texture.clone()),
                    double_sided: true,
                    emissive: LinearRgba::WHITE,
                    unlit: true,
                    cull_mode: None,
                    ..default()
                })),
                Transform::from_translation(Vec3::new(0.0, 0.0, -near)),
            ))
            .id();

        // Add children and store references
        commands
            .entity(probe_entity)
            .insert(ProbeVisualizationChildren {
                frustum_entity: Some(frustum_entity),
                near_plane_entity: Some(near_plane_entity),
            })
            .add_child(frustum_entity)
            .add_child(near_plane_entity);
    }
}

fn update_probe_visualizations(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    probe_query: Query<
        (&Projection, &KernelBindGroup, &ProbeVisualizationChildren),
        (
            With<ProbeCamera>,
            Or<(Changed<Projection>, Changed<KernelBindGroup>)>,
        ),
    >,
    mut frustum_query: Query<&mut Mesh3d, (With<FrustumMeshMarker>, Without<NearPlaneMarker>)>,
    mut near_plane_query: Query<
        (&mut Mesh3d, &mut Transform, Option<&MeshMaterial3d<StandardMaterial>>),
        (With<NearPlaneMarker>, Without<FrustumMeshMarker>),
    >,
) {
    for (projection, kernel_bind_group, children) in &probe_query {
        let Projection::Perspective(perspective) = projection else {
            continue;
        };

        let near = perspective.near;
        let near_half_height = near * (perspective.fov / 2.0).tan();
        let near_half_width = near_half_height * perspective.aspect_ratio;

        // Create new meshes - use -Vec3::Z so plane faces towards camera origin
        let new_near_plane_mesh = meshes.add(Plane3d::new(
            -Vec3::Z,
            Vec2::new(near_half_width, near_half_height),
        ));

        let frustum_mesh_builder =
            ProbeFrustumMeshBuilder::from_perspective_projection(perspective);
        let new_frustum_mesh = meshes.add(frustum_mesh_builder.build());

        // Use the source_texture from KernelBindGroup (same as the render target)
        let new_texture = Some(kernel_bind_group.source_texture.clone());

        // Update frustum mesh
        if let Some(frustum_entity) = children.frustum_entity {
            if let Ok(mut mesh_handle) = frustum_query.get_mut(frustum_entity) {
                *mesh_handle = Mesh3d::from(new_frustum_mesh);
            }
        }

        // Update near plane mesh, transform, and material
        if let Some(near_plane_entity) = children.near_plane_entity {
            if let Ok((mut mesh_handle, mut transform, material_handle)) =
                near_plane_query.get_mut(near_plane_entity)
            {
                *mesh_handle = Mesh3d::from(new_near_plane_mesh);
                transform.translation = Vec3::new(0.0, 0.0, -near);

                if let Some(material_handle) = material_handle {
                    if let Some(material) = materials.get_mut(material_handle) {
                        material.base_color_texture = new_texture;
                    }
                }
            }
        }
    }
}
