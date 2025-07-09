use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ProbeState {
    #[default]
    Idle,
    Probing,
} 