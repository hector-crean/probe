use bevy::{
    ecs::system::ParamSet, pbr::{NotShadowCaster, NotShadowReceiver}, prelude::*, render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    }
};

#[derive(Component)]
#[require(Camera)]
pub struct RecorderCamera;

#[derive(Component)]
pub struct FrustumWireframeEntity;

#[derive(Component)]
pub struct FrustumMonitor;

#[derive(Component)]
pub struct CameraGizmo;

/// A wireframe representation of a camera frustum
#[derive(Debug, Clone)]
pub struct FrustumWireframe {
    pub near: f32,
    pub far: f32,
    pub near_half_width: f32,
    pub near_half_height: f32,
    pub far_half_width: f32,
    pub far_half_height: f32,
}

impl FrustumWireframe {
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

        Self::new(near, far, near_half_width, near_half_height, far_half_width, far_half_height)
    }
}

impl From<FrustumWireframe> for Mesh {
    fn from(frustum: FrustumWireframe) -> Self {
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::LineList,
            RenderAssetUsages::default(),
        );

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Near plane corners
        let near_corners = [
            Vec3::new(-frustum.near_half_width, -frustum.near_half_height, -frustum.near),
            Vec3::new(frustum.near_half_width, -frustum.near_half_height, -frustum.near),
            Vec3::new(frustum.near_half_width, frustum.near_half_height, -frustum.near),
            Vec3::new(-frustum.near_half_width, frustum.near_half_height, -frustum.near),
        ];

        // Far plane corners
        let far_corners = [
            Vec3::new(-frustum.far_half_width, -frustum.far_half_height, -frustum.far),
            Vec3::new(frustum.far_half_width, -frustum.far_half_height, -frustum.far),
            Vec3::new(frustum.far_half_width, frustum.far_half_height, -frustum.far),
            Vec3::new(-frustum.far_half_width, frustum.far_half_height, -frustum.far),
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

/// Creates a 3D cross/gizmo mesh to represent the camera's position and orientation
fn create_camera_gizmo_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::LineList,
        RenderAssetUsages::default(),
    );

    let axis_length = 1.0;
    let vertices = vec![
        // X-axis - Right
        Vec3::ZERO,
        Vec3::new(axis_length, 0.0, 0.0),
        // Y-axis - Up  
        Vec3::ZERO,
        Vec3::new(0.0, axis_length, 0.0),
        // Z-axis - Forward (negative Z in camera space)
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, -axis_length),
        // Additional cross lines for better visibility
        Vec3::new(-0.3, -0.3, 0.0),
        Vec3::new(0.3, 0.3, 0.0),
        Vec3::new(-0.3, 0.3, 0.0),
        Vec3::new(0.3, -0.3, 0.0),
        // Center indicator
        Vec3::new(-0.1, 0.0, 0.0),
        Vec3::new(0.1, 0.0, 0.0),
        Vec3::new(0.0, -0.1, 0.0),
        Vec3::new(0.0, 0.1, 0.0),
    ];

    let indices = vec![
        0, 1,   // X-axis
        2, 3,   // Y-axis
        4, 5,   // Z-axis
        6, 7,   // Cross line 1
        8, 9,   // Cross line 2
        10, 11, // Center horizontal
        12, 13, // Center vertical
    ];

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

pub struct RecorderCameraPlugin;

impl Plugin for RecorderCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::setup_recorder_camera);
        app.add_systems(Update, Self::visualize_recorder_camera);
        // Also run the visualization system on startup to ensure initial setup
        app.add_systems(PostStartup, Self::visualize_recorder_camera);
    }
}

impl RecorderCameraPlugin {
    pub fn setup_recorder_camera(
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        let size = Extent3d {
            width: 1920,
            height: 1080,
            ..default()
        };

        // This is the texture that will be rendered to.
        let mut image = Image::new_fill(
            size,
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Bgra8UnormSrgb,
            RenderAssetUsages::default(),
        );
        // You need to set these texture usage flags in order to use the image as a render target
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT;

        let image_handle = images.add(image);

        // Calculate aspect ratio from the render target size
        let aspect_ratio = size.width as f32 / size.height as f32;

        // Create the camera that will do the rendering
        let camera_entity = commands
            .spawn((
                RecorderCamera,
                Camera3d::default(),
                Camera {
                    target: image_handle.clone().into(),
                    clear_color: Color::srgb(0.1, 0.1, 0.1).into(),
                    ..default()
                },
                Projection::Perspective(PerspectiveProjection {
                    near: 2.0,
                    far: 4.0,
                    fov: std::f32::consts::PI / 3.0, // 60 degrees
                    aspect_ratio, // Explicitly set the aspect ratio
                }),
                Transform::from_translation(Vec3::new(5.0, 3.0, 5.0))
                    .looking_at(Vec3::ZERO, Vec3::Y),
            )).id();

        // Create monitor entity
        let monitor_entity = commands
            .spawn((
                FrustumMonitor,
                NotShadowCaster,
                NotShadowReceiver,
                Mesh3d::from(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(1.0, 1.0)))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(image_handle.clone()),
                    double_sided: true,
                    emissive: LinearRgba::WHITE,
                    unlit: true,
                    cull_mode: None, // Explicitly disable culling for double-sided rendering
                    ..default()
                })),
                Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)), // Position at near plane
            ))
            .id();

        // Create wireframe as child
        let wireframe_entity = commands
            .spawn((
                Mesh3d::default(),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 1.0, 0.0, 1.0), // Yellow wireframe
                    unlit: true,
                    ..default()
                })),
                NotShadowCaster,
                NotShadowReceiver,
                FrustumWireframeEntity,
            ))
            .id();

        // Create camera gizmo as child
        let gizmo_entity = commands
            .spawn((
                Mesh3d::from(meshes.add(create_camera_gizmo_mesh())),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    unlit: true,
                    ..default()
                })),
                NotShadowCaster,
                NotShadowReceiver,
                Transform::from_scale(Vec3::splat(0.5)), // Make gizmo smaller
                CameraGizmo,
            ))
            .id();

        // Set children of camera
        commands
            .entity(camera_entity)
            .add_children(&[monitor_entity, wireframe_entity, gizmo_entity]);
    }

    pub fn visualize_recorder_camera(
        mut recorder_camera_query: Query<
            (&Transform, &Projection),
            (With<RecorderCamera>, Or<(Added<RecorderCamera>, Changed<Transform>, Changed<Projection>)>)
        >,
        mut monitor_query: Query<
            (&mut Transform, &mut Mesh3d),
            (With<FrustumMonitor>, Without<RecorderCamera>)
        >,
        mut wireframe_query: Query<
            &mut Mesh3d,
            (With<FrustumWireframeEntity>, Without<FrustumMonitor>, Without<RecorderCamera>)
        >,
        mut gizmo_query: Query<
            &mut Transform,
            (With<CameraGizmo>, Without<FrustumMonitor>, Without<RecorderCamera>)
        >,
        mut meshes: ResMut<Assets<Mesh>>,
    ) {
        for (camera_transform, projection) in recorder_camera_query.iter() {
            if let Projection::Perspective(perspective) = projection {
                let near = perspective.near;
                let far = perspective.far;
                let fov = perspective.fov;
                let aspect = perspective.aspect_ratio;

                // Safety checks to prevent division by zero
                if near <= 0.0 || far <= near || fov <= 0.0 || aspect <= 0.0 {
                    warn!("Invalid camera projection parameters: near={}, far={}, fov={}, aspect={}", near, far, fov, aspect);
                    continue;
                }

                // Calculate near plane dimensions
                let near_half_height = near * (fov / 2.0).tan();
                let near_half_width = near_half_height * aspect;

                // Calculate far plane dimensions
                let far_half_height = far * (fov / 2.0).tan();
                let far_half_width = far_half_height * aspect;

                // Additional safety check for calculated dimensions
                if near_half_width <= 0.0 || near_half_height <= 0.0 || far_half_width <= 0.0 || far_half_height <= 0.0 {
                    warn!("Invalid frustum dimensions calculated: near_half_width={}, near_half_height={}, far_half_width={}, far_half_height={}", 
                          near_half_width, near_half_height, far_half_width, far_half_height);
                    continue;
                }

                // Update monitor (near plane)
                for (mut monitor_transform, mut monitor_mesh) in monitor_query.iter_mut() {
                    let new_mesh = meshes.add(Mesh::from(Plane3d {
                        normal: Dir3::Z, // Face away from the camera
                        half_size: Vec2::new(near_half_width, near_half_height),
                    }));

                    *monitor_mesh = Mesh3d::from(new_mesh);
                    monitor_transform.translation = Vec3::new(0.0, 0.0, -near);
                    // Ensure the plane is oriented correctly for double-sided viewing
                    monitor_transform.rotation = Quat::IDENTITY;
                }

                // Update wireframe
                for mut wireframe_mesh in wireframe_query.iter_mut() {
                    let wireframe = FrustumWireframe::from_perspective_projection(perspective);
                    *wireframe_mesh = Mesh3d::from(meshes.add(wireframe));
                }

                // Update gizmo (it's a child entity, so it automatically follows the camera)
                // We just need to ensure it maintains its scale
                for mut gizmo_transform in gizmo_query.iter_mut() {
                    gizmo_transform.scale = Vec3::splat(0.5);
                }
            }
        }
    }
}
