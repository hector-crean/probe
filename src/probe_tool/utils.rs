use bevy::prelude::*;

/// Intersect a ray with a plane defined by the monitor's transform
/// `plane_size` is the actual dimensions of the plane (width, height)
pub fn intersect_ray_with_plane(
    ray: Ray3d, 
    plane_transform: &GlobalTransform,
    plane_size: Vec2,
) -> Option<Vec3> {
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

    // Check if intersection is within the plane bounds using actual plane size
    let local_pos = plane_transform
        .affine()
        .inverse()
        .transform_point3(intersection);
    
    let half_width = plane_size.x / 2.0;
    let half_height = plane_size.y / 2.0;
    
    if local_pos.x.abs() <= half_width && local_pos.y.abs() <= half_height {
        Some(intersection)
    } else {
        None
    }
}

/// Convert a world space intersection point to texture coordinates
/// `plane_size` is the actual dimensions of the plane (width, height)
pub fn world_to_texture_coords(
    intersection: Vec3,
    plane_transform: &GlobalTransform,
    plane_size: Vec2,
) -> Vec2 {
    let local_pos = plane_transform
        .affine()
        .inverse()
        .transform_point3(intersection);
    
    let half_width = plane_size.x / 2.0;
    let half_height = plane_size.y / 2.0;
    
    Vec2::new(
        ((local_pos.x / half_width) * 0.5 + 0.5).clamp(0.0, 1.0),
        ((-local_pos.y / half_height) * 0.5 + 0.5).clamp(0.0, 1.0),
    )
} 