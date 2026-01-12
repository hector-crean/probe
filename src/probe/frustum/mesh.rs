//! Mesh building for probe frustum visualization.

use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_mesh::Indices;

/// A probe frustum based on a perspective projection.
pub struct ProbeFrustum {
    pub projection: PerspectiveProjection,
}

impl ProbeFrustum {
    pub fn new(projection: PerspectiveProjection) -> Self {
        Self { projection }
    }
}

/// Mesh builder for creating frustum wireframes.
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
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
}

impl Meshable for ProbeFrustum {
    type Output = ProbeFrustumMeshBuilder;

    fn mesh(&self) -> Self::Output {
        ProbeFrustumMeshBuilder::from_perspective_projection(&self.projection)
    }
}

/// Mesh builder for creating a single axis line (used for camera orientation axes).
pub struct CameraAxisMeshBuilder {
    pub direction: Vec3,
    pub length: f32,
    pub radius: f32,
}

impl CameraAxisMeshBuilder {
    pub fn new(direction: Vec3, length: f32, radius: f32) -> Self {
        Self {
            direction,
            length,
            radius,
        }
    }

    /// Create an X axis (right, red).
    pub fn x_axis(length: f32) -> Self {
        Self::new(Vec3::X, length, 0.015) // Thicker radius for visibility
    }

    /// Create a Y axis (up, green).
    pub fn y_axis(length: f32) -> Self {
        Self::new(Vec3::Y, length, 0.015) // Thicker radius for visibility
    }

    /// Create a Z axis (forward, blue).
    pub fn z_axis(length: f32) -> Self {
        Self::new(Vec3::Z, length, 0.015) // Thicker radius for visibility
    }
}

impl MeshBuilder for CameraAxisMeshBuilder {
    fn build(&self) -> Mesh {
        // Create a thin cuboid (rectangular prism) oriented along the axis direction
        // This gives us thick lines that are simpler than rotating cylinders
        let normalized_dir = self.direction.normalize();

        // Create a thin cuboid - determine dimensions based on axis direction
        let (width, height, depth) = if normalized_dir.dot(Vec3::X).abs() > 0.99 {
            // X axis: thin in Y and Z, long in X
            (self.length, self.radius * 2.0, self.radius * 2.0)
        } else if normalized_dir.dot(Vec3::Y).abs() > 0.99 {
            // Y axis: thin in X and Z, long in Y
            (self.radius * 2.0, self.length, self.radius * 2.0)
        } else {
            // Z axis: thin in X and Y, long in Z
            (self.radius * 2.0, self.radius * 2.0, self.length)
        };

        // Create cuboid mesh - it's centered at origin, so we need to offset by half length
        let cuboid = Cuboid::new(width, height, depth);
        let cuboid_mesh = cuboid.mesh().build();

        // Offset the mesh so it starts at origin and extends along the axis
        let offset = normalized_dir * (self.length * 0.5);
        let positions: Vec<Vec3> = cuboid_mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|attr| attr.as_float3())
            .map(|positions| positions.iter().map(|p| Vec3::from(*p) + offset).collect())
            .unwrap_or_default();

        // Create new mesh with offset vertices
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

        // Copy other attributes
        if let Some(normals) = cuboid_mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());
        }

        if let Some(uvs) = cuboid_mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.clone());
        }

        if let Some(indices) = cuboid_mesh.indices() {
            mesh.insert_indices(indices.clone());
        }

        mesh
    }
}

/// Mesh builder for creating an up direction chevron on the near plane.
pub struct UpChevronMeshBuilder {
    pub near_half_width: f32,
    pub near_half_height: f32,
    pub chevron_size: f32,
    pub vertical_offset: f32,
}

impl UpChevronMeshBuilder {
    pub fn new(
        near_half_width: f32,
        near_half_height: f32,
        chevron_size: f32,
        vertical_offset: f32,
    ) -> Self {
        Self {
            near_half_width,
            near_half_height,
            chevron_size,
            vertical_offset,
        }
    }

    /// Create a chevron positioned on the near plane.
    /// `near_half_width` and `near_half_height` are the half-dimensions of the near plane.
    /// `chevron_size` is the size of the chevron (half-width of the base).
    /// `vertical_offset` is how far up from center to position the chevron (positive = up).
    pub fn from_near_plane(near_half_width: f32, near_half_height: f32) -> Self {
        // Size the chevron proportionally to the near plane (about 10% of height)
        let chevron_size = near_half_height * 0.1;
        // Position it near the top of the near plane (offset upward)
        let vertical_offset = near_half_height * 0.7;
        Self::new(
            near_half_width,
            near_half_height,
            chevron_size,
            vertical_offset,
        )
    }
}

impl MeshBuilder for UpChevronMeshBuilder {
    fn build(&self) -> Mesh {
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::LineList,
            RenderAssetUsages::default(),
        );

        // Chevron is an inverted V pointing upward
        // Positioned on the near plane at z = -near (which is 0.0 in camera-local space before translation)
        // But since the chevron will be a child with Transform, we position it at z=0 in local space
        let z = 0.0;

        // Center of chevron horizontally, offset vertically
        let center_x = 0.0;
        let center_y = self.vertical_offset;

        // Bottom-left point of chevron (left side of V)
        let left_bottom = Vec3::new(center_x - self.chevron_size, center_y, z);
        // Top point of chevron (tip of V)
        let top = Vec3::new(center_x, center_y + self.chevron_size, z);
        // Bottom-right point of chevron (right side of V)
        let right_bottom = Vec3::new(center_x + self.chevron_size, center_y, z);

        let vertices = vec![left_bottom, top, right_bottom];

        // Two lines: left_bottom -> top, top -> right_bottom
        let indices = vec![0u32, 1u32, 1u32, 2u32];

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
}
