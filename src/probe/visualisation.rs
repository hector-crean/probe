use bevy::{
    ecs::system::ParamSet,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::{camera::RenderTarget, render_asset::RenderAssetUsages},
    window::PrimaryWindow,
    gizmos::gizmos::Gizmos,
};

use crate::probe::{Probe, ProbeBindGroup, ProbeSettings};

pub struct ProbeVisualizationPlugin;

impl Plugin for ProbeVisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::setup,
                update_probe_visualizations,
                handle_monitor_interaction,
                update_monitor_highlight,
            ),
        );
    }
}

#[derive(Component)]
pub struct ProbeMonitor {
    pub probe_entity: Entity,
    pub is_hovered: bool,
}

#[derive(Component)]
pub struct ProbeFrustum {
    pub projection: PerspectiveProjection,
}

impl ProbeFrustum {
    pub fn new(projection: PerspectiveProjection) -> Self {
        Self { projection }
    }
}

pub struct ProbeFrustumMeshBuilder {
    pub near: f32,
    pub far: f32,
    pub near_half_width: f32,
    pub near_half_height: f32,
    pub far_half_width: f32,
    pub far_half_height: f32,
}

impl ProbeFrustumMeshBuilder {
    pub fn new(
        near: f32,
        far: f32,
        near_half_width: f32,
        near_half_height: f32,
        far_half_width: f32,
        far_half_height: f32,
    ) -> Self {
        Self {
            near,
            far,
            near_half_width,
            near_half_height,
            far_half_width,
            far_half_height,
        }
    }

    pub fn from_perspective_projection(projection: &PerspectiveProjection) -> Self {
        let near = projection.near;
        let far = projection.far;
        let fov = projection.fov;
        let aspect = projection.aspect_ratio;

        let near_half_height = near * (fov / 2.0).tan();
        let near_half_width = near_half_height * aspect;
        let far_half_height = far * (fov / 2.0).tan();
        let far_half_width = far_half_height * aspect;

        Self::new(
            near,
            far,
            near_half_width,
            near_half_height,
            far_half_width,
            far_half_height,
        )
    }
}

impl MeshBuilder for ProbeFrustumMeshBuilder {
    fn build(&self) -> Mesh {
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::LineList,
            RenderAssetUsages::default(),
        );

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Near plane corners
        let near_corners = [
            Vec3::new(-self.near_half_width, -self.near_half_height, -self.near),
            Vec3::new(self.near_half_width, -self.near_half_height, -self.near),
            Vec3::new(self.near_half_width, self.near_half_height, -self.near),
            Vec3::new(-self.near_half_width, self.near_half_height, -self.near),
        ];

        // Far plane corners
        let far_corners = [
            Vec3::new(-self.far_half_width, -self.far_half_height, -self.far),
            Vec3::new(self.far_half_width, -self.far_half_height, -self.far),
            Vec3::new(self.far_half_width, self.far_half_height, -self.far),
            Vec3::new(-self.far_half_width, self.far_half_height, -self.far),
        ];

        // Add vertices
        vertices.extend_from_slice(&near_corners);
        vertices.extend_from_slice(&far_corners);

        // Near plane edges
        for i in 0..4 {
            indices.push(i as u32);
            indices.push(((i + 1) % 4) as u32);
        }

        // Far plane edges
        for i in 0..4 {
            indices.push((i + 4) as u32);
            indices.push((((i + 1) % 4) + 4) as u32);
        }

        // Connecting edges
        for i in 0..4 {
            indices.push(i as u32);
            indices.push((i + 4) as u32);
        }

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
        mesh
    }
}

impl Meshable for ProbeFrustum {
    type Output = ProbeFrustumMeshBuilder;

    fn mesh(&self) -> Self::Output {
        ProbeFrustumMeshBuilder::from_perspective_projection(&self.projection)
    }
}

impl ProbeVisualizationPlugin {
    pub fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        probe_query: Query<(Entity, &Projection, &Camera, &ProbeBindGroup), Added<ProbeBindGroup>>,
    ) {
        for (entity, projection, _camera, bind_group) in probe_query.iter() {
            let perspective = match projection {
                Projection::Perspective(perspective) => perspective,
                _ => continue,
            };
            let frustum_mesh_builder =
                ProbeFrustumMeshBuilder::from_perspective_projection(perspective);
            let mesh = meshes.add(frustum_mesh_builder.build());

            let frustum_entity = commands
                .spawn((
                    ProbeFrustum {
                        projection: perspective.clone(),
                    },
                    Mesh3d::from(mesh),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgba(1.0, 1.0, 0.0, 1.0), // Yellow wireframe
                        unlit: true,
                        ..default()
                    })),
                    NotShadowCaster,
                    NotShadowReceiver,
                ))
                .id();

            // Create monitor entity
            let monitor_entity = commands
                .spawn((
                    ProbeMonitor {
                        probe_entity: entity,
                        is_hovered: false,
                    },
                    Mesh3d::from(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(1.0, 1.0)))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        base_color_texture: Some(bind_group.source_texture.clone()),
                        double_sided: true,
                        emissive: LinearRgba::WHITE,
                        unlit: true,
                        cull_mode: None, // Explicitly disable culling for double-sided rendering
                        ..default()
                    })),
                    NotShadowCaster,
                    NotShadowReceiver,
                ))
                .id();

            // Set children of camera
            commands
                .entity(entity)
                .add_children(&[frustum_entity, monitor_entity]);
        }
    }
}

/// System to handle mouse interaction with probe monitors
fn handle_monitor_interaction(
    mut probe_query: Query<&mut ProbeSettings>,
    monitor_query: Query<(&ProbeMonitor, &GlobalTransform), With<ProbeMonitor>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
) {
    // Process on mouse click or drag
    if !mouse_button_input.pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Get the main camera (not the probe camera)
    let Ok((camera, camera_transform)) = camera_query
        .iter()
        .find(|(cam, _)| matches!(cam.target, RenderTarget::Window(_)))
        .ok_or("No main camera found")
    else {
        return;
    };

    // Convert cursor position to world ray
    let Some(ray) = camera
        .viewport_to_world(camera_transform, cursor_position)
        .ok()
    else {
        return;
    };

    // Check intersection with each monitor
    for (monitor, monitor_transform) in monitor_query.iter() {
        if let Some(intersection) = intersect_ray_with_plane(ray, monitor_transform) {
            // Convert intersection point to texture coordinates
            let local_pos = monitor_transform
                .affine()
                .inverse()
                .transform_point3(intersection);

            // Convert from local plane coordinates (-0.5 to 0.5) to texture coordinates (0 to 1)
            let texture_coords = Vec2::new(
                (local_pos.x + 0.5).clamp(0.0, 1.0),
                (0.5 - local_pos.y).clamp(0.0, 1.0), // Flip Y coordinate
            );

            // Update the probe's center coordinates
            if let Ok(mut probe_settings) = probe_query.get_mut(monitor.probe_entity) {
                probe_settings.center_coords = texture_coords;
                info!(
                    "Updated probe center coordinates to: ({:.3}, {:.3})",
                    texture_coords.x, texture_coords.y
                );
            }

            // Only handle the first intersection
            break;
        }
    }
}

/// Intersect a ray with a plane defined by the monitor's transform
fn intersect_ray_with_plane(ray: Ray3d, plane_transform: &GlobalTransform) -> Option<Vec3> {
    let plane_normal = *plane_transform.forward();
    let plane_point = plane_transform.translation();

    // Calculate ray-plane intersection
    let denom = ray.direction.dot(plane_normal);

    // Ray is parallel to plane
    if denom.abs() < 1e-6 {
        return None;
    }

    let t = (plane_point - ray.origin).dot(plane_normal) / denom;

    // Intersection is behind the ray origin
    if t < 0.0 {
        return None;
    }

    let intersection = ray.origin + t * ray.direction;

    // Check if intersection is within the plane bounds (assuming 1x1 plane)
    let local_pos = plane_transform
        .affine()
        .inverse()
        .transform_point3(intersection);
    if local_pos.x.abs() <= 0.5 && local_pos.y.abs() <= 0.5 {
        Some(intersection)
    } else {
        None
    }
}

/// System to update visual feedback for monitor interaction
fn update_monitor_highlight(
    mut monitor_query: Query<(
        &mut ProbeMonitor,
        &GlobalTransform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut gizmos: Gizmos,
) {
    let Ok(window) = window_query.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        // No cursor, unhighlight all monitors
        for (mut monitor, _, material_handle) in monitor_query.iter_mut() {
            if monitor.is_hovered {
                monitor.is_hovered = false;
                if let Some(material) = materials.get_mut(material_handle) {
                    material.emissive = LinearRgba::WHITE;
                }
            }
        }
        return;
    };

    // Get the main camera
    let Ok((camera, camera_transform)) = camera_query
        .iter()
        .find(|(cam, _)| matches!(cam.target, RenderTarget::Window(_)))
        .ok_or("No main camera found")
    else {
        return;
    };

    // Convert cursor position to world ray
    let Some(ray) = camera
        .viewport_to_world(camera_transform, cursor_position)
        .ok()
    else {
        return;
    };

    // Check which monitor is being hovered
    for (mut monitor, monitor_transform, material_handle) in monitor_query.iter_mut() {
        let intersection_point = intersect_ray_with_plane(ray, monitor_transform);
        let is_intersecting = intersection_point.is_some();

        // Draw cross gizmo at intersection point
        if let Some(intersection) = intersection_point {
            let cross_size = 0.05; // Size of the cross arms
            let cross_color = Color::srgb(1.0, 0.0, 0.0); // Red color for visibility
            
            // Draw horizontal line
            gizmos.line(
                intersection - monitor_transform.right() * cross_size,
                intersection + monitor_transform.right() * cross_size,
                cross_color,
            );
            
            // Draw vertical line
            gizmos.line(
                intersection - monitor_transform.up() * cross_size,
                intersection + monitor_transform.up() * cross_size,
                cross_color,
            );
        }

        if is_intersecting != monitor.is_hovered {
            monitor.is_hovered = is_intersecting;

            if let Some(material) = materials.get_mut(material_handle) {
                if is_intersecting {
                    // Highlight when hovered
                    material.emissive = LinearRgba::new(1.2, 1.2, 1.2, 1.0);
                } else {
                    // Normal state
                    material.emissive = LinearRgba::WHITE;
                }
            }
        }
    }
}

/// Updates the visualization meshes and material if the probe's `Projection` or `Camera` component changes.
fn update_probe_visualizations(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    probe_query: Query<
        (&Projection, &Camera, &Children),
        (With<Probe>, Or<(Changed<Projection>, Changed<Camera>)>),
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

        // --- Recalculate mesh data based on the new projection ---
        let near = perspective.near;
        let near_half_height = near * (perspective.fov / 2.0).tan();
        let near_half_width = near_half_height * perspective.aspect_ratio;

        let new_monitor_mesh = meshes.add(Plane3d::new(
            Vec3::Z,
            Vec2::new(near_half_width, near_half_height),
        ));

        let frustum_mesh_builder =
            ProbeFrustumMeshBuilder::from_perspective_projection(perspective);
        let new_wireframe_mesh = meshes.add(frustum_mesh_builder.build());

        // --- Get the new render target texture from the camera ---
        let new_texture = camera.target.as_image().cloned();

        // --- Apply all updates to the child entities ---
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
