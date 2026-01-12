//! Frustum visualization and interaction for probe cameras.

mod components;
pub mod interaction;
mod mesh;
mod visualization;

pub use components::{
    CameraAxisXMarker, CameraAxisYMarker, CameraAxisZMarker, FrustumMeshMarker,
    FrustumNearPlaneIntersection, NearPlaneMarker, ProbeVisualizationChildren, UpChevronMarker,
};
pub use interaction::NearPlaneDragState;
pub use mesh::{
    CameraAxisMeshBuilder, ProbeFrustum, ProbeFrustumMeshBuilder, UpChevronMeshBuilder,
};

use bevy::prelude::*;

/// Plugin for frustum visualization and interaction.
pub struct FrustumPlugin;

impl Plugin for FrustumPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(interaction::FrustumNearPlaneInteractionPlugin)
            .add_systems(
                Update,
                (
                    visualization::draw_frustum,
                    visualization::update_probe_visualizations,
                ),
            );
    }
}
