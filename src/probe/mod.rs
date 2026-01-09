//! Probe camera system for GPU-based render target probing.
//!
//! This module provides a complete system for creating secondary cameras
//! that render to textures and allow GPU-based pixel sampling.

pub mod components;
pub mod events;
pub mod frustum;
pub mod pipeline;
pub mod state;
pub mod systems;

pub use components::{KernelSettings, KernelSize, ProbeCamera, ProbeKernelConfig, ProbeKernelData};
pub use events::{ProbeCameraCommand, ProbeCameraOutputMessage};
pub use frustum::{FrustumNearPlaneIntersection, FrustumPlugin, NearPlaneDragState};
pub use pipeline::{KernelBindGroup, ProbePipelinePlugin};
pub use state::ProbeToolState;

use bevy::prelude::*;

/// Main plugin for the probe camera system.
///
/// This plugin provides:
/// - GPU-based render target probing
/// - Frustum visualization
/// - Near plane interaction
/// - Kernel computation pipeline
pub struct ProbeToolPlugin;

impl Plugin for ProbeToolPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            pipeline::ProbePipelinePlugin,
            frustum::FrustumPlugin,
            crate::ui::KernelPopupPlugin,
        ))
        .add_message::<events::ProbeCameraCommand>()
        .add_message::<events::ProbeCameraOutputMessage>()
        .init_state::<state::ProbeToolState>()
        .add_systems(
            Update,
            state::handle_state_transition
                .run_if(on_message::<StateTransitionEvent<state::ProbeToolState>>),
        )
        .add_systems(Update, systems::handle_message)
        .add_observer(systems::handle_readback_complete);
    }
}
