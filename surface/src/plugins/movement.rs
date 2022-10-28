use bevy::app::{App, CoreStage};
use bevy::prelude::{EventReader, EventWriter, Plugin};
use common::state::RobotStateUpdate;
use common::types::Movement;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Movement>();
        app.add_system_to_stage(CoreStage::PostUpdate, send_movement);
    }
}

pub fn send_movement(mut movements: EventReader<Movement>, mut updates: EventWriter<RobotStateUpdate>) {
    let mut total_movement = Movement::default();

    for movement in movements.iter() {
        total_movement += *movement;
    }

    updates.send(RobotStateUpdate::Movement(total_movement));
}
