//! Kernel popup panel for displaying probe results.

use bevy::prelude::*;

use super::worldspace_node::WorldspaceUiNodePlugin;

/// Plugin for kernel popup panels.
pub struct KernelPopupPlugin;

impl Plugin for KernelPopupPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(WorldspaceUiNodePlugin::<KernelPopupPanel>::new());
    }
}

/// Component to mark kernel popup panels and associate them with cameras.
#[derive(Component, Default)]
pub struct KernelPopupPanel;
