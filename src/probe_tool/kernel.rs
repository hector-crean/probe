use bevy::prelude::*;
use bevy::ecs::system::Commands;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureDescriptor, TextureUsages};
use crate::probe_tool::events::ProbeCameraOutputMessage;
use crate::worldspace_ui_node::{WorldspaceUiNode, WorldspaceUiNodePlugin};

pub struct KernelPopupPlugin;

impl Plugin for KernelPopupPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(WorldspaceUiNodePlugin::<KernelPopupPanel>::new());
    
    }
}



/// Component to mark kernel popup panels and associate them with cameras
#[derive(Component, Default)]
pub struct KernelPopupPanel;

