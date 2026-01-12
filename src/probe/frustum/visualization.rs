//! Frustum visualization systems.

use bevy::prelude::*;

use crate::probe::{
    components::ProbeCamera,
    frustum::{
        components::{
            CameraAxisXMarker, CameraAxisYMarker, CameraAxisZMarker, FrustumMeshMarker,
            NearPlaneMarker, ProbeVisualizationChildren, UpChevronMarker,
        },
        mesh::{
            CameraAxisMeshBuilder, ProbeFrustum, ProbeFrustumMeshBuilder, UpChevronMeshBuilder,
        },
    },
    pipeline::KernelBindGroup,
};

/// System to draw the frustum for newly added probe cameras.
pub fn draw_frustum(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    probe_query: Query<(Entity, &Projection, &KernelBindGroup), Added<ProbeCamera>>,
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
                Mesh3d::from(meshes.add(Plane3d::new(
                    -Vec3::Z,
                    Vec2::new(near_half_width, near_half_height),
                ))),
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

        // Spawn camera axes at origin (X=red, Y=green, Z=blue)
        const AXIS_LENGTH: f32 = 0.4;
        let axis_x_entity = commands
            .spawn((
                CameraAxisXMarker,
                Mesh3d::from(meshes.add(CameraAxisMeshBuilder::x_axis(AXIS_LENGTH).build())),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.0, 0.0), // RED
                    unlit: true,
                    ..default()
                })),
                Transform::IDENTITY,
            ))
            .id();

        let axis_y_entity = commands
            .spawn((
                CameraAxisYMarker,
                Mesh3d::from(meshes.add(CameraAxisMeshBuilder::y_axis(AXIS_LENGTH).build())),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.0, 1.0, 0.0), // GREEN
                    unlit: true,
                    ..default()
                })),
                Transform::IDENTITY,
            ))
            .id();

        let axis_z_entity = commands
            .spawn((
                CameraAxisZMarker,
                Mesh3d::from(meshes.add(CameraAxisMeshBuilder::z_axis(AXIS_LENGTH).build())),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.0, 0.0, 1.0), // BLUE
                    unlit: true,
                    ..default()
                })),
                Transform::IDENTITY,
            ))
            .id();

        // Spawn up direction chevron on near plane
        let chevron_mesh = meshes
            .add(UpChevronMeshBuilder::from_near_plane(near_half_width, near_half_height).build());
        let up_chevron_entity = commands
            .spawn((
                UpChevronMarker,
                Mesh3d::from(chevron_mesh),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.0, 1.0, 0.0), // GREEN to match up axis
                    unlit: true,
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
                axis_x_entity: Some(axis_x_entity),
                axis_y_entity: Some(axis_y_entity),
                axis_z_entity: Some(axis_z_entity),
                up_chevron_entity: Some(up_chevron_entity),
            })
            .add_child(frustum_entity)
            .add_child(near_plane_entity)
            .add_child(axis_x_entity)
            .add_child(axis_y_entity)
            .add_child(axis_z_entity)
            .add_child(up_chevron_entity);
    }
}

/// System to update frustum visualizations when projection or kernel settings change.
pub fn update_probe_visualizations(
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
        (
            &mut Mesh3d,
            &mut Transform,
            Option<&MeshMaterial3d<StandardMaterial>>,
        ),
        (With<NearPlaneMarker>, Without<FrustumMeshMarker>),
    >,
    mut chevron_query: Query<
        (&mut Mesh3d, &mut Transform),
        (
            With<UpChevronMarker>,
            Without<NearPlaneMarker>,
            Without<FrustumMeshMarker>,
        ),
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

        // Update up chevron mesh and transform (position changes with near plane dimensions)
        if let Some(chevron_entity) = children.up_chevron_entity {
            if let Ok((mut mesh_handle, mut transform)) = chevron_query.get_mut(chevron_entity) {
                let new_chevron_mesh = meshes.add(
                    UpChevronMeshBuilder::from_near_plane(near_half_width, near_half_height)
                        .build(),
                );
                *mesh_handle = Mesh3d::from(new_chevron_mesh);
                transform.translation = Vec3::new(0.0, 0.0, -near);
            }
        }

        // Axes don't need updating - their geometry is fixed and they're at the origin
        // (Transform::IDENTITY), so they automatically follow the camera transform via parent/child
    }
}
