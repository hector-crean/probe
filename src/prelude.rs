//! Commonly used exports for the probe crate.
//!
//! This prelude re-exports the most commonly used types and plugins
//! to make it easy to get started with the probe system.

// Camera controllers
pub use crate::camera::{
    OrbitCameraController, OrbitCameraControllerPlugin, ProbeCameraController,
    ProbeCameraControllerPlugin,
};

// Probe system
pub use crate::probe::{
    FrustumNearPlaneIntersection, KernelBindGroup, KernelSettings, KernelSize,
    NearPlaneDragState, ProbeCamera, ProbeCameraCommand, ProbeCameraOutputMessage,
    ProbeKernelConfig, ProbeKernelData, ProbeToolPlugin, ProbeToolState,
};

// UI components
pub use crate::ui::{KernelPopupPanel, KernelPopupPlugin, WorldspaceUiNode, WorldspaceUiNodePlugin};

// Frustum interaction marker (needed for examples)
pub use crate::camera::MainCamera;
