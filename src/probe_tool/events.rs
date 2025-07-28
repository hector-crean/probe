use bevy::prelude::*;



#[derive(Event)]
pub enum ProbeCameraEvent {
    Add {
        transform: Transform,
        resolution: UVec2,
    },
    // Remove {
    //     entity: Entity,
    // }
}


