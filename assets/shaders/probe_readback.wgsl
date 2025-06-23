

struct Probe {
    position: vec2<f32>,
};


@group(0) @binding(0) var<storage, read_write> readback_data: array<u32>;
@group(0) @binding(1) var<storage, read> probes: array<Probe>;
@group(0) @binding(2) var screen_texture: texture_2d<f32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let probe_index = global_id.x;

    // Boundary check
    if probe_index >= arrayLength(&probes) {
        return;
    }

    let probe = probes[probe_index];
    let texture_size = vec2<f32>(textureDimensions(screen_texture));
    let pixel_coords = vec2<i32>(floor(probe.position));

    // Boundary check for texture access
    if pixel_coords.x < 0 || pixel_coords.x >= i32(texture_size.x) || pixel_coords.y < 0 || pixel_coords.y >= i32(texture_size.y) {
        readback_data[probe_index] = 0u; // Write black for out-of-bounds probes
        return;
    }

    // Load the color from the texture at the probe's pixel coordinates
    let color_f32 = textureLoad(screen_texture, pixel_coords, 0); // 0 is mip level

    // Pack the f32 color (0.0-1.0 range) into a u32 (8 bits per channel: RGBA)
    let color_u8 = vec4<u32>(color_f32 * 255.0);
    let packed_color = (color_u8.a << 24) | (color_u8.b << 16) | (color_u8.g << 8) | color_u8.r;

    readback_data[probe_index] = packed_color;
}
