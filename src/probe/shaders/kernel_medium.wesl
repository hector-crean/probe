// ===== SHARED TYPE DEFINITIONS =====

// This struct must match the `ProbeSettings` struct in Rust.
// Bevy's `ShaderType` derive macro handles padding and alignment (std140),
// so we just need to match the fields in order.
struct ProbeSettings {
    // Size of the kernel to read, in pixels.
    kernel_size: vec2<f32>,
    // Normalized coordinates [0, 1] of the probe's center.
    center_coords: vec2<f32>,
}

// ===== SHARED BINDINGS =====

// These bindings must match the `ProbeBindGroup` struct in Rust.
@group(0) @binding(0) var<uniform> settings: ProbeSettings;
@group(0) @binding(1) var source_texture: texture_2d<f32>;
@group(0) @binding(2) var<storage, read_write> output_buffer: array<vec4<f32>>;

// ===== UTILITY FUNCTIONS =====

// Checks if the given coordinates are within the kernel bounds
fn is_within_kernel_bounds(id: vec3<u32>, kernel_dims: vec2<u32>) -> bool {
    return id.x < kernel_dims.x && id.y < kernel_dims.y;
}

// Calculates the texture coordinates for sampling
fn calculate_sample_coords(id: vec3<u32>, settings: ProbeSettings, texture_dims: vec2<f32>) -> vec2<f32> {
    // Calculate the pixel coordinate of the probe's center.
    let center_pixel = texture_dims * settings.center_coords;

    // Calculate the top-left corner of the kernel area in the source texture.
    let kernel_top_left = center_pixel - settings.kernel_size / 2.0;

    // Calculate the specific source pixel to read for this invocation.
    return kernel_top_left + vec2<f32>(id.xy);
}

// Converts 2D coordinates to 1D buffer index
fn coords_to_buffer_index(id: vec3<u32>, kernel_dims: vec2<u32>) -> u32 {
    return id.y * kernel_dims.x + id.x;
}

// Validates buffer index to prevent out-of-bounds access
fn is_valid_buffer_index(index: u32) -> bool {
    return index < arrayLength(&output_buffer);
}

// ===== SAMPLING FUNCTIONS =====

// Samples a color from the texture using integer coordinates
fn sample_texture_at_coord(coord: vec2<f32>) -> vec4<f32> {
    let pixel_coord = vec2<i32>(coord);
    return textureLoad(source_texture, pixel_coord, 0);
}

// Enhanced sampling with bounds checking
fn sample_texture_safe(coord: vec2<f32>, texture_dims: vec2<f32>) -> vec4<f32> {
    let pixel_coord = vec2<i32>(coord);

    // Clamp coordinates to texture bounds
    let clamped_coord = vec2<i32>(
        clamp(pixel_coord.x, 0, i32(texture_dims.x) - 1),
        clamp(pixel_coord.y, 0, i32(texture_dims.y) - 1)
    );

    return textureLoad(source_texture, clamped_coord, 0);
}

// ===== CORE PROBE LOGIC =====

// Main probe processing function - the core logic shared across all variants
fn process_probe_pixel(id: vec3<u32>) {
    let kernel_dims = vec2<u32>(settings.kernel_size);

    // Early exit for out-of-bounds threads
    if !is_within_kernel_bounds(id, kernel_dims) {
        return;
    }

    // Get texture dimensions
    let texture_dims = vec2<f32>(textureDimensions(source_texture));

    // Calculate sampling coordinates
    let sample_coord = calculate_sample_coords(id, settings, texture_dims);

    // Sample the color (using safe sampling for robustness)
    let color = sample_texture_safe(sample_coord, texture_dims);

    // Calculate output buffer index
    let output_index = coords_to_buffer_index(id, kernel_dims);

    // Validate buffer bounds before writing
    if !is_valid_buffer_index(output_index) {
        return;
    }

    // Write the color to the output buffer
    output_buffer[output_index] = color;
}

// Small workgroup variant optimized for tiny kernels (1-16 pixels)
// Uses 1x1 workgroups for perfect pixel-to-thread mapping with zero waste
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    process_probe_pixel(global_id);
}


// Shader Variant: Small Workgroup (WESL)
// Workgroup Size: 1 x 1 x 1
// Optimization Target: Minimal kernels (1x1 to 4x4)

// Advantages:
// - Zero thread waste for small kernels
// - Simple dispatch calculation (1:1 mapping)
// - Minimal GPU resource usage
// - Perfect for precise single-pixel probes

// Best Use Cases:
// - Single pixel color picking
// - Small kernel convolutions
// - Precise measurements
// - Debug/diagnostic sampling

// WESL Features Used:
// - Module imports for code sharing
// - Shared function definitions
// - Centralized type definitions
