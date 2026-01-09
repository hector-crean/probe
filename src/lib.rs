//! Probe - GPU-based render target probing for Bevy.
//!
//! This crate provides a system for creating secondary cameras that render
//! to textures and allow GPU-based pixel sampling with kernel operations.
//!
//! # Features
//!
//! - **Probe Cameras**: Secondary cameras that render to textures
//! - **Kernel Sampling**: GPU compute shaders for efficient pixel sampling
//! - **Frustum Visualization**: Visual debugging of probe camera frusta
//! - **Near Plane Interaction**: Click/drag to control sampling points
//! - **Camera Controllers**: Orbit and probe camera movement controls
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use probe::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((
//!             DefaultPlugins,
//!             OrbitCameraControllerPlugin,
//!             ProbeCameraControllerPlugin,
//!             ProbeToolPlugin,
//!         ))
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: MessageWriter<ProbeCameraCommand>) {
//!     commands.write(ProbeCameraCommand::Add {
//!         transform: Transform::from_xyz(4.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
//!         resolution: UVec2::new(1024, 1024),
//!     });
//! }
//! ```

pub mod camera;
pub mod probe;
pub mod ui;
pub mod utils;

pub mod prelude;

// Re-export main plugins at crate root for convenience
pub use camera::{OrbitCameraControllerPlugin, ProbeCameraControllerPlugin};
pub use probe::ProbeToolPlugin;
