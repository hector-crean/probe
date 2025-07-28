use bevy::{
    prelude::*,
    render::camera::RenderTarget,
    window::PrimaryWindow,
};

use crate::probe::{
    events::{ProbeClickEvent, ProbeHoverEvent},
    monitor::ProbeMonitor,
    state::ProbeState,
    utils::{intersect_ray_with_plane, world_to_texture_coords},
    KernelSettings,
};


/// System to toggle the probing state with keyboard input
pub fn toggle_probing_state(
    mut next_state: ResMut<NextState<ProbeState>>,
    current_state: Res<State<ProbeState>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyP) {
        let new_state = match current_state.get() {
            ProbeState::Idle => ProbeState::Probing,
            ProbeState::Probing => ProbeState::Idle,
        };
        info!("Switching probe state to: {:?}", new_state);
        next_state.set(new_state);
    }
} 