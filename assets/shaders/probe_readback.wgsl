

struct Probe {
    kernel: vec2<f32>,
    coords: vec2<f32>,
};


@group(0) @binding(0) var<uniform> probe: Probe;
@group(0) @binding(1) var<storage, read_write> readback_data: array<vec4<f32>>;
@group(0) @binding(2) var probe_render_target: texture_2d<f32>;

 @compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = i32(id.x);
    let y = i32(id.y);
    let kernel_size = probe.kernel.x * probe.kernel.y;
    if x >= kernel_size || y >= kernel_size { return; }
    let kernel_idx = y * kernel_size + x;
    let half_size = kernel_size / 2;
    let offset_x = x - half_size;
    let offset_y = y - half_size;

    let texture_dimensions = vec2f(textureDimensions(probe_render_target));
    let centerPixel = probe.coords * texture_dimensions;
    let pixel_pos = vec2i(centerPixel) + vec2i(offset_x, offset_y);
    let clamped_pos = vec2i(clamp(pixel_pos.x, 0, i32(texture_dimensions.x) - 1), clamp(pixel_pos.y, 0, i32(texture_dimensions.y) - 1));
    readback_data[kernel_idx] = textureLoad(probe_render_target, clamped_pos, 0);
}
