//! Events and messages for the probe camera system.

use super::components::{KernelSize, ProbeKernelData};
use bevy::prelude::*;
use std::fmt;

/// Commands for controlling probe cameras.
#[derive(Message)]
pub enum ProbeCameraCommand {
    Add {
        transform: Transform,
        resolution: UVec2,
    },
}

/// Output messages from probe cameras.
#[derive(Message, Debug)]
pub enum ProbeCameraOutputMessage {
    /// Kernel data changed since the last readback.
    ///
    /// This only fires when the pixel data actually differs from the
    /// previous frame, avoiding redundant processing for static scenes.
    KernelChanged {
        entity: Entity,
        data: Vec<Vec4>,
        kernel_size: Vec2,
    },
}

impl ProbeCameraOutputMessage {
    /// Converts the kernel data to ML-ready format (CHW layout).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(kernel_data) = message.to_kernel_data() {
    ///     // Shape is [4, height, width] for RGBA
    ///     let shape = kernel_data.shape();
    ///     
    ///     // Convert to Burn tensor (with `burn` feature)
    ///     #[cfg(feature = "burn")]
    ///     let tensor = kernel_data.to_burn_tensor::<Wgpu>(&device);
    /// }
    /// ```
    pub fn to_kernel_data(&self) -> Option<ProbeKernelData> {
        match self {
            ProbeCameraOutputMessage::KernelChanged {
                data, kernel_size, ..
            } => Some(ProbeKernelData::from_rgba(
                data,
                KernelSize::from(*kernel_size),
            )),
        }
    }

    /// Converts the kernel data to grayscale ML-ready format.
    ///
    /// Returns a single-channel tensor in `[1, H, W]` format.
    /// Useful when color information is not needed.
    pub fn to_grayscale_kernel_data(&self) -> Option<ProbeKernelData> {
        match self {
            ProbeCameraOutputMessage::KernelChanged {
                data, kernel_size, ..
            } => Some(ProbeKernelData::from_grayscale(
                data,
                KernelSize::from(*kernel_size),
            )),
        }
    }

    /// Returns the entity this message pertains to.
    pub fn entity(&self) -> Entity {
        match self {
            ProbeCameraOutputMessage::KernelChanged { entity, .. } => *entity,
        }
    }
}

impl fmt::Display for ProbeCameraOutputMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbeCameraOutputMessage::KernelChanged {
                entity,
                data,
                kernel_size,
            } => {
                let width = kernel_size.x as usize;
                let height = kernel_size.y as usize;

                if data.len() != width * height {
                    return write!(
                        f,
                        "⚠️  Kernel data size mismatch: expected {}x{} = {}, got {}",
                        width,
                        height,
                        width * height,
                        data.len()
                    );
                }

                writeln!(
                    f,
                    "🔍 Kernel Changed for Entity {:?} ({}x{})",
                    entity, width, height
                )?;
                writeln!(f, "┌{}┐", "─".repeat(width * 8 + 1))?;

                for row in 0..height {
                    write!(f, "│")?;
                    for col in 0..width {
                        let index = row * width + col;
                        let pixel = data[index];

                        // Convert RGBA to terminal color
                        let r = (pixel.x.clamp(0.0, 1.0) * 255.0) as u8;
                        let g = (pixel.y.clamp(0.0, 1.0) * 255.0) as u8;
                        let b = (pixel.z.clamp(0.0, 1.0) * 255.0) as u8;

                        // Use ANSI 24-bit color (RGB) as background
                        write!(f, "\x1b[48;2;{};{};{}m", r, g, b)?;
                        write!(f, " {:5.2} ", pixel.x)?; // Show red component value
                        write!(f, "\x1b[0m")?; // Reset color
                    }
                    writeln!(f, "│")?;
                }

                writeln!(f, "└{}┘", "─".repeat(width * 8 + 1))?;
                write!(
                    f,
                    "Background colors show RGB values, numbers show red component"
                )
            }
        }
    }
}

/// Wrapper for compact kernel display.
pub struct CompactKernel<'a>(pub &'a ProbeCameraOutputMessage);

impl fmt::Display for CompactKernel<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            ProbeCameraOutputMessage::KernelChanged {
                entity,
                data,
                kernel_size,
            } => {
                let width = kernel_size.x as usize;
                let height = kernel_size.y as usize;

                if data.len() != width * height {
                    return write!(
                        f,
                        "⚠️  Kernel mismatch: {}x{} ≠ {}",
                        width,
                        height,
                        data.len()
                    );
                }

                write!(f, "🔍 Kernel {:?} ({}x{}): ", entity, width, height)?;

                for (i, pixel) in data.iter().enumerate() {
                    let r = (pixel.x.clamp(0.0, 1.0) * 255.0) as u8;
                    let g = (pixel.y.clamp(0.0, 1.0) * 255.0) as u8;
                    let b = (pixel.z.clamp(0.0, 1.0) * 255.0) as u8;

                    // Show colored blocks with brightness indicator
                    write!(f, "\x1b[48;2;{};{};{}m", r, g, b)?;
                    let brightness = (pixel.x + pixel.y + pixel.z) / 3.0;
                    let symbol = if brightness > 0.7 {
                        "██"
                    } else if brightness > 0.3 {
                        "▓▓"
                    } else {
                        "░░"
                    };
                    write!(f, "{}", symbol)?;
                    write!(f, "\x1b[0m")?;

                    // Add row separator
                    if (i + 1) % width == 0 && i + 1 < data.len() {
                        write!(f, " ")?;
                    }
                }
                Ok(())
            }
        }
    }
}
