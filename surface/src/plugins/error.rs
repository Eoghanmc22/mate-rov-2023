use bevy::prelude::*;
use crate::plugins::MateStage;

// TODO maybe keep a list of errors that have occurred

pub struct ErrorPlugin;

impl Plugin for ErrorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ErrorEvent>();
        app.add_system_to_stage(MateStage::ErrorHandling, handle_error);
    }
}

pub struct ErrorEvent(pub anyhow::Error);

fn handle_error(mut errors: EventReader<ErrorEvent>) {
    for error in errors.iter() {
        error!("An error occurred: {:?}", error.0);
    }
}


