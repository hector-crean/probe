//! Core components for the probe camera system.
//!
//! # ML Framework Integration
//!
//! The [`ProbeKernelData`] type provides a framework-agnostic tensor representation
//! that can be converted to various ML frameworks like Burn or ndarray.
//!
//! ```rust,ignore
//! // Convert kernel data to Burn tensor
//! let kernel_data = ProbeKernelData::from_rgba(&pixels, kernel_size);
//! let tensor = kernel_data.to_burn_tensor::<Wgpu>(&device);
//! ```

use bevy::{
    ecs::{component::Component, entity::Entity, query::QueryData},
    math::{UVec2, Vec2, Vec4},
    prelude::*,
    render::render_resource::ShaderType,
};

use crate::camera::ProbeCameraController;
use crate::probe::frustum::FrustumNearPlaneIntersection;
use crate::probe::pipeline::KernelBindGroup;
use bevy::render::gpu_readback::Readback;
use bevy_camera::visibility::RenderLayers;
use bevy_camera::{Camera3d, Projection};

/// Bundle for spawning a complete probe camera entity.
///
/// This bundle contains all components required for a functional probe camera.
/// Use [`spawn_probe_camera`](crate::probe::systems::spawn_probe_camera) or
/// the [`ProbeCameraCommands`] trait extension to spawn probe cameras.
#[derive(Bundle)]
pub struct ProbeCameraBundle {
    pub probe_camera: ProbeCamera,
    pub transform: Transform,
    pub camera: Camera,
    pub camera_3d: Camera3d,
    pub projection: Projection,
    pub kernel_bind_group: KernelBindGroup,
    pub near_plane_intersection: FrustumNearPlaneIntersection,
    pub readback: Readback,
    pub render_layers: RenderLayers,
    pub controller: ProbeCameraController,
}

/// A component for a 3D camera that should be probed.
///
/// This component requires several other components to function properly.
/// Use [`ProbeCameraBundle`] or [`spawn_probe_camera`](crate::probe::systems::spawn_probe_camera)
/// to spawn a complete probe camera entity.
#[derive(Component)]
#[require(Camera, Camera3d, Transform, KernelBindGroup)]
pub struct ProbeCamera {
    pub resolution: UVec2,
}

impl Default for ProbeCamera {
    fn default() -> Self {
        Self {
            resolution: UVec2::new(512, 512),
        }
    }
}

/// Component to store kernel size information for probe cameras.
#[derive(Component)]
pub struct ProbeKernelConfig {
    pub kernel_size: Vec2,
}

/// Component to cache previous kernel data for change detection.
///
/// This is automatically managed by the probe system - you don't need
/// to interact with it directly.
#[derive(Component, Default)]
pub struct PreviousKernelData {
    pub data: Option<Vec<Vec4>>,
}

/// Size of a probe kernel in pixels.
///
/// This newtype wrapper makes kernel size parameters more explicit and
/// type-safe when used in function signatures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KernelSize(pub Vec2);

impl KernelSize {
    /// Creates a new kernel size with the given width and height in pixels.
    pub fn new(width: f32, height: f32) -> Self {
        Self(Vec2::new(width, height))
    }

    /// Returns the width of the kernel.
    pub fn width(self) -> f32 {
        self.0.x
    }

    /// Returns the height of the kernel.
    pub fn height(self) -> f32 {
        self.0.y
    }

    /// Returns the total number of pixels in the kernel.
    pub fn pixel_count(self) -> usize {
        (self.0.x * self.0.y) as usize
    }
}

impl From<Vec2> for KernelSize {
    fn from(value: Vec2) -> Self {
        Self(value)
    }
}

impl From<KernelSize> for Vec2 {
    fn from(value: KernelSize) -> Self {
        value.0
    }
}

/// Framework-agnostic kernel data in CHW (Channels, Height, Width) format.
///
/// This is the standard layout for ML frameworks like Burn and PyTorch.
/// The data is stored as `[C, H, W]` where:
/// - C = 4 (RGBA channels)
/// - H = kernel height in pixels
/// - W = kernel width in pixels
///
/// # ML Framework Compatibility
///
/// | Framework | Expected Format | Conversion |
/// |-----------|-----------------|------------|
/// | Burn      | `[C, H, W]`     | Direct use |
/// | PyTorch   | `[N, C, H, W]`  | Add batch dimension |
/// | TensorFlow| `[N, H, W, C]`  | Permute to NHWC |
///
/// # Example
///
/// ```rust,ignore
/// use probe::prelude::*;
///
/// // From probe output message
/// let kernel_data = ProbeKernelData::from_rgba(&pixels, kernel_size);
///
/// // Access raw data
/// let shape = kernel_data.shape(); // [4, 3, 3] for 3x3 RGBA
/// let flat = kernel_data.as_slice();
///
/// // Convert to Burn (with `burn` feature)
/// #[cfg(feature = "burn")]
/// let tensor = kernel_data.to_burn_tensor::<Wgpu>(&device);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ProbeKernelData {
    /// Raw pixel data in CHW format (channel-major order).
    data: Vec<f32>,
    /// Shape: [channels, height, width]
    shape: [usize; 3],
}

impl ProbeKernelData {
    /// Creates kernel data from RGBA pixel data in row-major (HWC) order.
    ///
    /// Converts from Bevy's `Vec<Vec4>` format to ML-standard CHW format.
    pub fn from_rgba(pixels: &[Vec4], kernel_size: KernelSize) -> Self {
        let width = kernel_size.width() as usize;
        let height = kernel_size.height() as usize;
        let channels = 4;

        debug_assert_eq!(
            pixels.len(),
            width * height,
            "Pixel count {} doesn't match kernel size {}x{}",
            pixels.len(),
            width,
            height
        );

        // Convert from HWC (row-major Vec4s) to CHW (channel-major)
        let mut data = vec![0.0f32; channels * height * width];

        for y in 0..height {
            for x in 0..width {
                let pixel = pixels[y * width + x];
                // Place each channel contiguously
                data[0 * height * width + y * width + x] = pixel.x; // R
                data[1 * height * width + y * width + x] = pixel.y; // G
                data[2 * height * width + y * width + x] = pixel.z; // B
                data[3 * height * width + y * width + x] = pixel.w; // A
            }
        }

        Self {
            data,
            shape: [channels, height, width],
        }
    }

    /// Creates kernel data from grayscale values.
    ///
    /// Useful when only luminance is needed for ML processing.
    pub fn from_grayscale(pixels: &[Vec4], kernel_size: KernelSize) -> Self {
        let width = kernel_size.width() as usize;
        let height = kernel_size.height() as usize;

        let data: Vec<f32> = pixels
            .iter()
            .map(|p| (p.x + p.y + p.z) / 3.0) // Simple luminance
            .collect();

        Self {
            data,
            shape: [1, height, width],
        }
    }

    /// Returns the shape as `[channels, height, width]`.
    #[inline]
    pub fn shape(&self) -> [usize; 3] {
        self.shape
    }

    /// Returns the number of channels.
    #[inline]
    pub fn channels(&self) -> usize {
        self.shape[0]
    }

    /// Returns the height in pixels.
    #[inline]
    pub fn height(&self) -> usize {
        self.shape[1]
    }

    /// Returns the width in pixels.
    #[inline]
    pub fn width(&self) -> usize {
        self.shape[2]
    }

    /// Returns the raw data as a slice in CHW order.
    #[inline]
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    /// Consumes self and returns the raw data vector.
    #[inline]
    pub fn into_vec(self) -> Vec<f32> {
        self.data
    }

    /// Gets a specific channel as a flat slice.
    ///
    /// Returns `None` if channel index is out of bounds.
    pub fn channel(&self, c: usize) -> Option<&[f32]> {
        if c >= self.shape[0] {
            return None;
        }
        let plane_size = self.shape[1] * self.shape[2];
        let start = c * plane_size;
        Some(&self.data[start..start + plane_size])
    }

    /// Gets a pixel value at (x, y) for all channels.
    ///
    /// Returns `None` if coordinates are out of bounds.
    pub fn pixel(&self, x: usize, y: usize) -> Option<Vec4> {
        if x >= self.shape[2] || y >= self.shape[1] {
            return None;
        }
        let plane_size = self.shape[1] * self.shape[2];
        let idx = y * self.shape[2] + x;

        Some(Vec4::new(
            self.data[0 * plane_size + idx],
            self.data.get(1 * plane_size + idx).copied().unwrap_or(0.0),
            self.data.get(2 * plane_size + idx).copied().unwrap_or(0.0),
            self.data.get(3 * plane_size + idx).copied().unwrap_or(1.0),
        ))
    }

    /// Normalizes pixel values to [0, 1] range.
    ///
    /// Useful for data that may have values outside the typical range.
    pub fn normalize(&mut self) {
        let min = self.data.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = self.data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;

        if range > f32::EPSILON {
            for v in &mut self.data {
                *v = (*v - min) / range;
            }
        }
    }

    /// Creates a copy with values normalized to [0, 1] range.
    pub fn normalized(&self) -> Self {
        let mut copy = self.clone();
        copy.normalize();
        copy
    }
}

// ============================================================================
// Burn ML Framework Integration (requires `burn` feature)
// ============================================================================

#[cfg(feature = "burn")]
impl ProbeKernelData {
    /// Converts to a Burn tensor with shape `[C, H, W]`.
    ///
    /// # Type Parameters
    ///
    /// - `B`: The Burn backend to use (e.g., `Wgpu`, `NdArray`, `Candle`)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use burn::backend::Wgpu;
    /// use probe::prelude::*;
    ///
    /// let kernel_data = ProbeKernelData::from_rgba(&pixels, kernel_size);
    /// let device = Default::default();
    /// let tensor = kernel_data.to_burn_tensor::<Wgpu>(&device);
    /// // tensor shape: [4, height, width]
    /// ```
    pub fn to_burn_tensor<B: burn::tensor::backend::Backend>(
        &self,
        device: &B::Device,
    ) -> burn::tensor::Tensor<B, 3> {
        let tensor_data = burn::tensor::TensorData::new(self.data.clone(), self.shape);
        burn::tensor::Tensor::from_data(tensor_data, device)
    }

    /// Converts to a Burn tensor with batch dimension: `[1, C, H, W]`.
    ///
    /// This is the format expected by most neural network models.
    pub fn to_burn_tensor_batched<B: burn::tensor::backend::Backend>(
        &self,
        device: &B::Device,
    ) -> burn::tensor::Tensor<B, 4> {
        self.to_burn_tensor::<B>(device).unsqueeze::<4>()
    }
}

// ============================================================================
// ndarray Integration (requires `ndarray` feature)
// ============================================================================

#[cfg(feature = "ndarray")]
impl ProbeKernelData {
    /// Converts to an ndarray `Array3<f32>` with shape `[C, H, W]`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let kernel_data = ProbeKernelData::from_rgba(&pixels, kernel_size);
    /// let array = kernel_data.to_ndarray();
    /// // array.shape() == [4, height, width]
    /// ```
    pub fn to_ndarray(&self) -> ndarray::Array3<f32> {
        ndarray::Array3::from_shape_vec(
            (self.shape[0], self.shape[1], self.shape[2]),
            self.data.clone(),
        )
        .expect("Shape mismatch - this should never happen")
    }

    /// Converts to an ndarray with batch dimension: `[1, C, H, W]`.
    pub fn to_ndarray_batched(&self) -> ndarray::Array4<f32> {
        ndarray::Array4::from_shape_vec(
            (1, self.shape[0], self.shape[1], self.shape[2]),
            self.data.clone(),
        )
        .expect("Shape mismatch - this should never happen")
    }
}

/// Settings for the kernel compute shader.
#[derive(Clone, PartialEq, ShaderType, Default)]
pub struct KernelSettings {
    /// Size of the kernel in pixels.
    pub kernel_size: Vec2,
    /// Normalized coordinates of the probe center on the render target.
    pub center_coords: Vec2,
    // Note: Std140 layout requires fields to be 16-byte aligned.
    // The `Vec2`s are padded automatically by the `ShaderType` derive.
}

/// A query data structure for all components required by ProbeCamera.
/// This makes it easy to query for entities that have all the probe camera components.
#[derive(QueryData)]
#[query_data(mutable)]
pub struct ProbeCameraQuery {
    pub entity: Entity,
    pub probe_camera: &'static ProbeCamera,
    pub transform: &'static mut Transform,
    pub camera_3d: &'static Camera3d,
    pub camera: &'static mut Camera,
    pub projection: &'static mut Projection,
    pub kernel_bind_group: &'static mut KernelBindGroup,
    pub near_plane_intersection: &'static mut FrustumNearPlaneIntersection,
    pub readback: &'static mut bevy::render::gpu_readback::Readback,
}
