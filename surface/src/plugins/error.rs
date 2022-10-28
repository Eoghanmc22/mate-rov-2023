use bevy::prelude::*;
use crate::plugins::MateStage;

pub struct ErrorPlugin;

impl Plugin for ErrorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<anyhow::Error>();
        app.add_system_to_stage(MateStage::ErrorHandling, handle_error);
    }
}

fn handle_error(mut errors: EventReader<anyhow::Error>) {
    for error in errors.iter() {
        error!("An error occurred: {:?}", error);
    }
}


