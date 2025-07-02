use bevy::{
    ecs::system::ParamSet,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::{
        RenderApp,
        camera::RenderTarget,
        extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
        gpu_readback::{GpuReadbackPlugin, Readback, ReadbackComplete},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{self, RenderLabel},
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BufferUsages,
            CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor, Extent3d,
            PipelineCache, ShaderStages, ShaderType, TextureDimension, TextureFormat,
            TextureUsages, binding_types::storage_buffer,
        },
        renderer::{RenderContext, RenderDevice},
        storage::{GpuShaderStorageBuffer, ShaderStorageBuffer},
    },
};

use crate::probe::{Probe, ProbeBindGroup};

pub struct ProbeVisualizationPlugin;

impl Plugin for ProbeVisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Self::setup, update_probe_visualizations));
    }
}

#[derive(Component)]
pub struct ProbeMonitor;


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
        for (entity, projection, camera, bind_group) in probe_query.iter() {
            let perspective = match projection {
                Projection::Perspective(perspective) => perspective,
                _ => continue,
            };
            let frustum_mesh_builder =
                ProbeFrustumMeshBuilder::from_perspective_projection(perspective);
            let mesh = meshes.add(frustum_mesh_builder.build());

            let frustum_entity = commands
                .spawn((
                    ProbeFrustum { projection: perspective.clone() },
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
            (&mut Mesh3d, &mut Transform, &MeshMaterial3d<StandardMaterial>),
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

        let new_monitor_mesh =
            meshes.add(Plane3d::new(Vec3::Z, Vec2::new(near_half_width, near_half_height)));

        let frustum_mesh_builder = ProbeFrustumMeshBuilder::from_perspective_projection(perspective);
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


