//! Probe tool state management.

use bevy::prelude::*;

/// State for the probe tool.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ProbeToolState {
    #[default]
    Idle,
    Probing,
}

/// Handle state transitions for the probe tool.
pub fn handle_state_transition(
    mut state_reader: MessageReader<StateTransitionEvent<ProbeToolState>>,
) {
    for event in state_reader.read() {
        info!(
            "ProbeToolPlugin state changed from {:?} to {:?}",
            event.exited, event.entered
        );
    }
}

/// Toggle probing state with keyboard input.
pub fn toggle_probing_state(
    mut next_state: ResMut<NextState<ProbeToolState>>,
    current_state: Res<State<ProbeToolState>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyP) {
        let new_state = match current_state.get() {
            ProbeToolState::Idle => ProbeToolState::Probing,
            ProbeToolState::Probing => ProbeToolState::Idle,
        };
        next_state.set(new_state);
    }
}
