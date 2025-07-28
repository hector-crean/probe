use bevy::{
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::render_asset::RenderAssetUsages,
};

use crate::probe::KernelBindGroup;

/// Component that represents a visual frustum for a probe camera
#[derive(Component)]
pub struct ProbeFrustum {
    pub projection: PerspectiveProjection,
}

impl ProbeFrustum {
    pub fn new(projection: PerspectiveProjection) -> Self {
        Self { projection }
    }
}

/// Mesh builder for creating frustum wireframes
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





/// System to create frustum entities for newly added probes
pub fn spawn_frustum_for_probe(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    probe_query: Query<(Entity, &Projection), Added<KernelBindGroup>>,
) {
    for (probe_entity, projection) in probe_query.iter() {
        let perspective = match projection {
            Projection::Perspective(perspective) => perspective,
            _ => continue,
        };

        let frustum_mesh_builder = ProbeFrustumMeshBuilder::from_perspective_projection(perspective);
        let mesh = meshes.add(frustum_mesh_builder.build());

        let frustum_entity = commands
            .spawn((
                ProbeFrustum::new(perspective.clone()),
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

        commands.entity(probe_entity).add_child(frustum_entity);
    }
} 