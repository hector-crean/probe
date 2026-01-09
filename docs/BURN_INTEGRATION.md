# Burn ML Integration Guide

This crate provides seamless integration with the [Burn](https://burn.dev) ML framework through the `ProbeKernelData` type.

## Quick Start

### Without ML Features (Default)

The kernel data is always available in a framework-agnostic format:

```rust
use probe::prelude::*;

fn handle_kernel_output(mut reader: MessageReader<ProbeCameraOutputMessage>) {
    for message in reader.read() {
        // Convert to ML-ready format
        if let Some(kernel_data) = message.to_kernel_data() {
            // Shape is [channels, height, width]
            let shape = kernel_data.shape(); // e.g., [4, 3, 3] for 3x3 RGBA
            
            // Access raw data
            let flat_data: &[f32] = kernel_data.as_slice();
            
            // Get individual channels
            let red_channel = kernel_data.channel(0);
            let green_channel = kernel_data.channel(1);
            
            // Or get a specific pixel
            if let Some(pixel) = kernel_data.pixel(1, 1) {
                println!("Center pixel: {:?}", pixel);
            }
        }
    }
}
```

### With Burn Integration

Enable the `burn` feature in your `Cargo.toml`:

```toml
[dependencies]
probe = { version = "0.1", features = ["burn"] }
burn = { version = "0.16", features = ["wgpu"] }
```

Then convert directly to Burn tensors:

```rust
use burn::backend::Wgpu;
use probe::prelude::*;

fn process_with_burn(
    mut reader: MessageReader<ProbeCameraOutputMessage>,
) {
    let device = Default::default();
    
    for message in reader.read() {
        if let Some(kernel_data) = message.to_kernel_data() {
            // Convert to Burn tensor [C, H, W]
            let tensor = kernel_data.to_burn_tensor::<Wgpu>(&device);
            
            // Or with batch dimension [1, C, H, W] for neural networks
            let batched = kernel_data.to_burn_tensor_batched::<Wgpu>(&device);
            
            // Now use with your Burn model
            // let prediction = model.forward(batched);
        }
    }
}
```

### With ndarray Integration

Enable the `ndarray` feature:

```toml
[dependencies]
probe = { version = "0.1", features = ["ndarray"] }
```

```rust
use probe::prelude::*;

fn process_with_ndarray(mut reader: MessageReader<ProbeCameraOutputMessage>) {
    for message in reader.read() {
        if let Some(kernel_data) = message.to_kernel_data() {
            // Convert to ndarray [C, H, W]
            let array = kernel_data.to_ndarray();
            
            // Access with ndarray syntax
            let red_channel = array.slice(s![0, .., ..]);
            
            // With batch dimension [1, C, H, W]
            let batched = kernel_data.to_ndarray_batched();
        }
    }
}
```

## Data Layout

### Probe's Raw Format

Bevy's GPU readback returns pixels as `Vec<Vec4>` in row-major (HWC) order:

```
pixels[y * width + x] = Vec4(R, G, B, A)
```

### ML Format (CHW)

`ProbeKernelData` converts to channel-first format, which is standard for ML:

```
data[c * height * width + y * width + x] = channel_value
```

Where:
- `c` = channel index (0=R, 1=G, 2=B, 3=A)
- `y` = row (0 = top)
- `x` = column (0 = left)

### Shape Conventions

| Format | Shape | Used By |
|--------|-------|---------|
| CHW | `[C, H, W]` | Burn, PyTorch |
| NCHW | `[N, C, H, W]` | Neural networks (batched) |
| NHWC | `[N, H, W, C]` | TensorFlow |

`ProbeKernelData` outputs CHW natively. Use `to_burn_tensor_batched()` for NCHW.

## ProbeKernelData API

### Creation

```rust
// From RGBA pixels (full color)
let data = ProbeKernelData::from_rgba(&pixels, kernel_size);

// From RGBA pixels (grayscale - single channel)
let gray = ProbeKernelData::from_grayscale(&pixels, kernel_size);
```

### Accessors

```rust
// Shape information
let shape: [usize; 3] = data.shape();     // [C, H, W]
let channels: usize = data.channels();     // 4 for RGBA
let height: usize = data.height();
let width: usize = data.width();

// Raw data access
let slice: &[f32] = data.as_slice();
let vec: Vec<f32> = data.into_vec();

// Channel/pixel access
let red: Option<&[f32]> = data.channel(0);
let pixel: Option<Vec4> = data.pixel(x, y);
```

### Normalization

```rust
// In-place normalization to [0, 1]
data.normalize();

// Non-mutating version
let normalized = data.normalized();
```

### ML Framework Conversion

```rust
// Burn (requires `burn` feature)
let tensor = data.to_burn_tensor::<Backend>(&device);        // [C, H, W]
let batched = data.to_burn_tensor_batched::<Backend>(&device); // [1, C, H, W]

// ndarray (requires `ndarray` feature)
let array = data.to_ndarray();        // Array3<f32>
let batched = data.to_ndarray_batched(); // Array4<f32>
```

## Example: Simple Color Classifier

```rust
use burn::backend::Wgpu;
use burn::nn::{Linear, LinearConfig};
use burn::tensor::Tensor;
use probe::prelude::*;

// A simple model that classifies kernel colors
struct ColorClassifier<B: burn::tensor::backend::Backend> {
    linear: Linear<B>,
}

impl<B: burn::tensor::backend::Backend> ColorClassifier<B> {
    fn forward(&self, input: Tensor<B, 4>) -> Tensor<B, 2> {
        // Flatten [N, C, H, W] -> [N, C*H*W]
        let [n, c, h, w] = input.dims();
        let flat = input.reshape([n, c * h * w]);
        self.linear.forward(flat)
    }
}

fn classify_kernel(
    mut reader: MessageReader<ProbeCameraOutputMessage>,
    model: Res<ColorClassifier<Wgpu>>,
) {
    let device = Default::default();
    
    for message in reader.read() {
        if let Some(kernel_data) = message.to_kernel_data() {
            let input = kernel_data.to_burn_tensor_batched::<Wgpu>(&device);
            let prediction = model.forward(input);
            println!("Classification: {:?}", prediction.to_data());
        }
    }
}
```

## Performance Considerations

1. **Conversion Cost**: `to_kernel_data()` performs a memory layout transformation. For high-frequency updates, consider caching or batching.

2. **GPU-to-CPU-to-GPU**: The current pipeline reads from GPU → CPU → optionally back to GPU for ML. For maximum performance, consider keeping data on GPU using Burn's WGPU backend (see below).

3. **Batch Processing**: When processing multiple kernels, batch them together:

```rust
// Less efficient: one tensor per kernel
for message in reader.read() {
    let tensor = message.to_kernel_data()?.to_burn_tensor::<B>(&device);
    model.forward(tensor.unsqueeze());
}

// More efficient: batch multiple kernels
let tensors: Vec<_> = reader.read()
    .filter_map(|m| m.to_kernel_data())
    .map(|d| d.to_burn_tensor::<B>(&device))
    .collect();

if !tensors.is_empty() {
    let batched = Tensor::stack(tensors, 0);
    model.forward(batched);
}
```

## GPU-to-GPU Integration (Advanced)

The current data flow is:

```
[Bevy GPU] → gpu_readback → [CPU Vec<Vec4>] → to_kernel_data() → [CPU Vec<f32>] → to_burn_tensor() → [Burn GPU]
```

For maximum performance, we want:

```
[Bevy GPU] → (zero-copy or minimal copy) → [Burn GPU]
```

### Why It's Challenging

1. **Separate wgpu Instances**: Bevy and Burn each create their own `wgpu::Device` and `wgpu::Queue`. GPU buffers can't be directly shared between different devices.

2. **Buffer Ownership**: Bevy's render pipeline owns the SSBO where kernel results are written. Burn would need access to this buffer.

3. **Synchronization**: GPU operations are asynchronous. We'd need careful fence/barrier management to ensure Bevy's compute shader finishes before Burn reads.

### Potential Solutions

#### Option 1: Shared wgpu Device (Invasive)

Force Bevy and Burn to share the same `wgpu::Device`:

```rust
// Hypothetical - would require significant changes to both libraries
let shared_device = wgpu::Device::new(...);
let bevy_renderer = BevyRenderer::with_device(shared_device.clone());
let burn_device = BurnWgpuDevice::with_device(shared_device.clone());
```

**Status**: Not currently supported by either library.

#### Option 2: Buffer Export/Import via Vulkan/DX12 (Platform-specific)

Use external memory extensions to share GPU memory:

```rust
// Export buffer handle from Bevy
let external_handle = bevy_buffer.export_handle(); // VkExternalMemoryHandleTypeFlagBits

// Import into Burn's device
let burn_buffer = burn_device.import_external_buffer(external_handle);
```

**Status**: Theoretically possible via `wgpu` external memory extensions, but complex and platform-specific.

#### Option 3: CPU Staging with Mapped Buffers (Minimal Overhead)

Use persistent mapped buffers to reduce copy overhead:

```rust
// Bevy side: create buffer with MAP_READ
let staging_buffer = device.create_buffer(&BufferDescriptor {
    usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
    mapped_at_creation: false,
    ..
});

// After compute: copy SSBO → staging, then map
// The mapped pointer can be read directly without full copy
```

**Status**: Current implementation. Can be optimized with persistent mapping.

#### Option 4: Burn Custom Backend (Best Long-term)

Create a custom Burn backend that wraps Bevy's renderer:

```rust
// Hypothetical: a Burn backend that uses Bevy's wgpu device
pub struct BevyBurnBackend {
    bevy_device: Res<RenderDevice>,
    bevy_queue: Res<RenderQueue>,
}

impl Backend for BevyBurnBackend {
    // Implement tensor operations using Bevy's device
}
```

**Status**: Would require significant work, but provides true zero-copy.

### Recommended Path Forward

For now, the CPU path works and is reasonably fast for small kernels (3x3, 5x5). The overhead is:
- ~1-10μs for small kernel conversion
- Memory bandwidth limited for larger kernels

When you need true GPU-to-GPU:

1. **Short term**: Optimize the CPU path with batching and async mapping
2. **Medium term**: Investigate `wgpu` external memory extensions for your target platform
3. **Long term**: Consider contributing a shared-device feature to Bevy/Burn

### Profiling the Current Path

```rust
use std::time::Instant;

fn process_with_timing(message: &ProbeCameraOutputMessage) {
    let t0 = Instant::now();
    let kernel_data = message.to_kernel_data().unwrap();
    let t1 = Instant::now();
    
    let tensor = kernel_data.to_burn_tensor::<Wgpu>(&device);
    let t2 = Instant::now();
    
    let output = model.forward(tensor.unsqueeze());
    let t3 = Instant::now();
    
    info!(
        "Timings: layout={:?}, upload={:?}, inference={:?}",
        t1 - t0, t2 - t1, t3 - t2
    );
}
```
