# WGSL Pixel Probing Guide

This guide explains how to use WGSL compute shaders to read pixel colors from render targets at specific probe positions in Bevy.

## Overview

Pixel probing allows you to:
- Read pixel colors from the final render output
- Sample colors at specific screen positions in real-time
- Process multiple probe points efficiently on the GPU
- Get color data back to the CPU for game logic

## Key Concepts

### 1. Render Target Capture

Before you can probe pixels, you need to capture the render target:

```rust
// Mark a camera for render target capture
commands.spawn((
    Camera3dBundle { ... },
    ProbeableCamera,  // This component enables capture
));
```

The system automatically:
- Creates a capture texture matching the render target
- Copies the main render target after rendering
- Makes the texture available for compute shader access

### 2. Probe Data Structure

Probes are defined with screen-space positions (0.0 to 1.0 range):

```rust
#[derive(Component, ShaderType)]
pub struct PixelProbe {
    pub position: Vec2,  // Screen space: (0,0) = bottom-left, (1,1) = top-right
}
```

In WGSL:
```wgsl
struct PixelProbe {
    position: vec2<f32>,
}
```

### 3. Compute Shader Architecture

The compute shader reads from the render target and writes results:

```wgsl
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
    
    // Convert normalized position to pixel coordinates
    let pixel_coords = vec2<i32>(probe.position * texture_size);
    
    // Sample the texture
    let color = textureLoad(screen_texture, pixel_coords, 0);
    
    // Store result
    probe_results[probe_index] = ProbeResult(color, probe.position);
}
```

## Implementation Steps

### Step 1: Set Up Render Target Capture

```rust
use bevy::render::view::ViewTarget;

// Add the render capture plugin
app.add_plugins(RenderCapturePlugin);

// Mark cameras for capture
commands.spawn((
    Camera3dBundle::default(),
    ProbeableCamera,
));
```

### Step 2: Create Probe Components

```rust
// Spawn probes in your scene
commands.spawn(PixelProbe {
    position: Vec2::new(0.5, 0.5), // Center of screen
});
```

### Step 3: Set Up GPU Buffers

```rust
// Create storage buffers for GPU communication
let probe_buffer = ShaderStorageBuffer::from(vec![PixelProbe::default(); 64]);
let result_buffer = ShaderStorageBuffer::from(vec![ProbeResult::default(); 64]);

// Enable CPU readback for results
result_buffer.buffer_description.usage |= BufferUsages::COPY_SRC;
```

### Step 4: Create Compute Pipeline

```rust
let layout = render_device.create_bind_group_layout(
    "pixel_probe_layout",
    &BindGroupLayoutEntries::sequential(
        ShaderStages::COMPUTE,
        (
            storage_buffer::<Vec<ProbeResult>>(false), // Output
            storage_buffer::<Vec<PixelProbe>>(true),   // Input
            texture_2d(TextureSampleType::Float { filterable: true }),
            sampler(SamplerBindingType::Filtering),
        ),
    ),
);
```

### Step 5: Execute Compute Shader

```rust
// In the render graph node
pass.set_bind_group(0, &bind_group, &[]);
pass.set_pipeline(compute_pipeline);
pass.dispatch_workgroups(num_probes, 1, 1);
```

### Step 6: Read Results Back to CPU

```rust
// Set up readback
commands.spawn(Readback::buffer(result_buffer.clone()))
    .observe(|trigger: Trigger<ReadbackComplete>| {
        let results: Vec<ProbeResult> = trigger.event().to_shader_type();
        
        for result in results {
            if result.color != Vec4::ZERO {
                println!("Probe at {:?}: color {:?}", result.position, result.color);
            }
        }
    });
```

## Advanced Techniques

### 1. Multi-Sample Probing

Sample surrounding pixels for anti-aliasing:

```wgsl
let offset = 1.0 / texture_size;
let samples = array<vec2<f32>, 5>(
    probe.position,
    probe.position + vec2<f32>(-offset.x, -offset.y),
    probe.position + vec2<f32>(offset.x, -offset.y),
    probe.position + vec2<f32>(-offset.x, offset.y),
    probe.position + vec2<f32>(offset.x, offset.y)
);

var avg_color = vec4<f32>(0.0);
for var i = 0; i < 5; i++ {
    avg_color += textureSample(screen_texture, screen_sampler, samples[i]);
}
result_color = avg_color / 5.0;
```

### 2. Packed Color Format

For memory efficiency, pack RGBA into u32:

```wgsl
fn pack_color(color: vec4<f32>) -> u32 {
    let color_u8 = vec4<u32>(clamp(color * 255.0, vec4<f32>(0.0), vec4<f32>(255.0)));
    return (color_u8.a << 24u) | (color_u8.b << 16u) | (color_u8.g << 8u) | color_u8.r;
}

fn unpack_color(packed: u32) -> vec4<f32> {
    let r = f32(packed & 0xFFu) / 255.0;
    let g = f32((packed >> 8u) & 0xFFu) / 255.0;
    let b = f32((packed >> 16u) & 0xFFu) / 255.0;
    let a = f32((packed >> 24u) & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, a);
}
```

### 3. Conditional Probing

Only probe when conditions are met:

```wgsl
struct ConditionalProbe {
    position: vec2<f32>,
    enabled: u32,
    threshold: f32,
}

@compute @workgroup_size(64, 1, 1)
fn conditional_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let probe_index = global_id.x;
    let probe = probes[probe_index];
    
    if probe.enabled == 0u {
        return;
    }
    
    let color = sample_texture(probe.position);
    
    // Only store if brightness exceeds threshold
    let brightness = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    if brightness > probe.threshold {
        probe_results[probe_index] = ProbeResult(color, probe.position);
    }
}
```

## Performance Considerations

### 1. Workgroup Size
- Use power-of-2 workgroup sizes (64, 128, 256)
- Match your GPU's warp/wavefront size for best performance
- Typical choice: `@workgroup_size(64, 1, 1)`

### 2. Memory Access Patterns
- Coalesce memory access when possible
- Use storage buffers for large arrays
- Consider using uniform buffers for small, frequently accessed data

### 3. Texture Sampling
- `textureLoad()` for exact pixel access (faster)
- `textureSample()` for filtered access (smoother, but slower)
- Use appropriate sampler settings for your use case

### 4. Buffer Management
- Reuse buffers across frames
- Only update buffers when probe data changes
- Use staging buffers for frequent CPU->GPU updates

## Common Use Cases

### 1. UI Color Picking
```rust
// Click to sample color under cursor
fn handle_color_picking(
    mut probe_manager: ResMut<ProbeManager>,
    mouse_input: Res<Input<MouseButton>>,
    windows: Query<&Window>,
) {
    if mouse_input.just_pressed(MouseButton::Left) {
        if let Some(cursor_pos) = windows.single().cursor_position() {
            let normalized = cursor_pos / windows.single().size();
            probe_manager.add_probe(normalized);
        }
    }
}
```

### 2. Minimap Generation
```rust
// Sample regular grid for minimap
fn create_minimap_probes() -> Vec<PixelProbe> {
    let mut probes = Vec::new();
    for y in 0..32 {
        for x in 0..32 {
            probes.push(PixelProbe {
                position: Vec2::new(x as f32 / 32.0, y as f32 / 32.0),
            });
        }
    }
    probes
}
```

### 3. Collision Detection
```rust
// Check if specific screen areas have objects
fn probe_collision_areas(areas: &[Rect]) -> Vec<PixelProbe> {
    areas.iter().map(|rect| PixelProbe {
        position: rect.center(),
    }).collect()
}
```

## Troubleshooting

### Common Issues

1. **Black/Zero Colors**: Check texture binding and format compatibility
2. **Flipped Y Coordinates**: Remember texture Y is flipped from screen Y
3. **Performance Issues**: Reduce probe count or optimize workgroup size
4. **Missing Results**: Ensure proper buffer synchronization

### Debug Tips

1. **Visualize Probes**: Draw debug markers at probe positions
2. **Log Coordinates**: Print probe positions to verify correct mapping
3. **Test with Known Colors**: Place colored quads at probe positions
4. **Check Buffer Sizes**: Ensure buffers can hold all probe data

## Best Practices

1. **Batch Probes**: Process multiple probes in one compute dispatch
2. **Update Efficiently**: Only update probe positions when they change
3. **Validate Bounds**: Always check texture coordinates are in valid range
4. **Handle Edge Cases**: Deal with probes outside texture bounds gracefully
5. **Profile Performance**: Measure GPU timing for optimization

## Example Integration

See `examples/enhanced_pixel_probe.rs` for a complete working example that demonstrates:
- Real-time probe movement with arrow keys
- Mouse-based probe placement
- Multiple probe management
- Color result visualization
- Performance optimization techniques