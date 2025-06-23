// This shader is used for the gpu_readback example
// The actual work it does is not important for the example


struct Probe {
    position: vec3<f32>,
}

// This is the data that lives in the gpu only buffer
@group(0) @binding(0) var<storage, read_write> data: array<u32>;

@group(0) @binding(1) var<storage, read> probes: array<Probe>;



@vertex
fn main() -> VertexOutput {
    VertexOutput {
        position: vec4<f32>(0.0, 0.0, 0.0, 1.0),
    }
}

@fragment
fn main() -> FragmentOutput {
    FragmentOutput {
        color: vec4<f32>(0.0, 0.0, 0.0, 1.0),
    }
}

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let index = invocation_id.x;
    if index >= 16u {
        return;
    }

    data[index] = index * 2u;
}
