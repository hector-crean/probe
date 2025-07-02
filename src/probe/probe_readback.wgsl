// This struct must match the `ProbeSettings` struct in Rust.
// Bevy's `ShaderType` derive macro handles padding and alignment (std140),
// so we just need to match the fields in order.
struct ProbeSettings {
    // Size of the kernel to read, in pixels.
    kernel_size: vec2<f32>,
    // Normalized coordinates [0, 1] of the probe's center.
    center_coords: vec2<f32>,
};

// These bindings must match the `ProbeBindGroup` struct in Rust.
@group(0) @binding(0) var<uniform> settings: ProbeSettings;

// A regular texture and a sampler for reading from it.
@group(0) @binding(1) var source_texture: texture_2d<f32>;
@group(0) @binding(2) var source_sampler: sampler;

@group(0) @binding(3) var<storage, read_write> output_buffer: array<vec4<f32>>;

// We dispatch one workgroup per pixel in the kernel.
// A workgroup size of (1, 1, 1) is simplest here, making the
// `global_invocation_id` directly map to a pixel in the kernel.
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let kernel_dims = vec2<u32>(settings.kernel_size);
    // Prevent reading/writing out of bounds if the dispatch size doesn't
    // perfectly match the buffer size.


    // if (id.x >= kernel_dims.x || id.y >= kernel_dims.y) {
    //     return;
    // }

    // Get the full dimensions of the source texture.
    let texture_dims = vec2<f32>(textureDimensions(source_texture));

    // Calculate the pixel coordinate of the probe's center.
    let center_pixel = texture_dims * settings.center_coords;

    // Calculate the top-left corner of the kernel area in the source texture.
    let kernel_top_left = center_pixel - settings.kernel_size / 2.0;

    // Calculate the specific source pixel to read for this invocation.
    let sample_coord = kernel_top_left + vec2<f32>(id.xy);

    // Convert to normalized UV coordinates for `textureSample`.
    // We add 0.5 to sample from the center of the texel.
    let sample_uv = (sample_coord + vec2(0.5)) / texture_dims;

    // Sample the color from the source texture at the calculated UV.
    let color = textureSample(source_texture, source_sampler, sample_uv);

    // Calculate the 1D index for the output buffer.
    let output_index = id.y * kernel_dims.x + id.x;

    // Write the color to the output buffer.
    output_buffer[output_index] = color;
}