// Enhanced pixel probe compute shader
// Reads pixel colors from the main render target at specified probe positions

struct PixelProbe {
    position: vec2<f32>,
}

struct ProbeResult {
    color: vec4<f32>,
    position: vec2<f32>,
}

// Input bindings
@group(0) @binding(0) var<storage, read_write> probe_results: array<ProbeResult>;
@group(0) @binding(1) var<storage, read> probes: array<PixelProbe>;
@group(0) @binding(2) var screen_texture: texture_2d<f32>;
@group(0) @binding(3) var screen_sampler: sampler;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let probe_index = global_id.x;

    // Boundary check
    if probe_index >= arrayLength(&probes) {
        return;
    }

    let probe = probes[probe_index];
    let texture_size = vec2<f32>(textureDimensions(screen_texture));

    // Convert screen space position (0-1 range) to pixel coordinates
    let pixel_coords = vec2<i32>(probe.position * texture_size);

    var result_color = vec4<f32>(0.0, 0.0, 0.0, 1.0); // Default to black

    // Boundary check for texture access
    if pixel_coords.x >= 0 && pixel_coords.x < i32(texture_size.x) && pixel_coords.y >= 0 && pixel_coords.y < i32(texture_size.y) {

        // Sample the texture at the probe position using both methods for flexibility

        // Method 1: Direct texture load (exact pixel)
        result_color = textureLoad(screen_texture, pixel_coords, 0);

        // Method 2: Sampled texture read (with filtering) - uncomment to use instead
        // let uv = probe.position;
        // result_color = textureSample(screen_texture, screen_sampler, uv);

        // Optional: Sample surrounding pixels for anti-aliasing or average color
        /*
        let offset = 1.0 / texture_size;
        let tl = textureSample(screen_texture, screen_sampler, probe.position + vec2<f32>(-offset.x, -offset.y));
        let tr = textureSample(screen_texture, screen_sampler, probe.position + vec2<f32>(offset.x, -offset.y));
        let bl = textureSample(screen_texture, screen_sampler, probe.position + vec2<f32>(-offset.x, offset.y));
        let br = textureSample(screen_texture, screen_sampler, probe.position + vec2<f32>(offset.x, offset.y));
        result_color = (tl + tr + bl + br + result_color) / 5.0;
        */
    }

    // Store the result
    probe_results[probe_index] = ProbeResult(
        result_color,
        probe.position
    );
}

// Alternative compute shader for packed color output (if you need u32 packed colors)
@compute @workgroup_size(64, 1, 1)
fn main_packed(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let probe_index = global_id.x;

    if probe_index >= arrayLength(&probes) {
        return;
    }

    let probe = probes[probe_index];
    let texture_size = vec2<f32>(textureDimensions(screen_texture));
    let pixel_coords = vec2<i32>(probe.position * texture_size);

    var packed_color: u32 = 0u;

    if pixel_coords.x >= 0 && pixel_coords.x < i32(texture_size.x) && pixel_coords.y >= 0 && pixel_coords.y < i32(texture_size.y) {

        let color = textureLoad(screen_texture, pixel_coords, 0);

        // Convert float color (0.0-1.0) to 8-bit values and pack into u32
        // Format: RGBA (R in lowest 8 bits)
        let color_u8 = vec4<u32>(clamp(color * 255.0, vec4<f32>(0.0), vec4<f32>(255.0)));
        packed_color = (color_u8.a << 24u) | (color_u8.b << 16u) | (color_u8.g << 8u) | color_u8.r;
    }

    // If using packed colors, you'd need a different output buffer structure
    // @group(0) @binding(0) var<storage, read_write> packed_results: array<u32>;
    // packed_results[probe_index] = packed_color;
}

// Utility function for color space conversions (if needed)
fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
    let cutoff = linear < vec3<f32>(0.0031308);
    let higher = vec3<f32>(1.055) * pow(linear, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    let lower = linear * vec3<f32>(12.92);
    return select(higher, lower, cutoff);
}

fn srgb_to_linear(srgb: vec3<f32>) -> vec3<f32> {
    let cutoff = srgb < vec3<f32>(0.04045);
    let higher = pow((srgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    let lower = srgb / vec3<f32>(12.92);
    return select(higher, lower, cutoff);
}
