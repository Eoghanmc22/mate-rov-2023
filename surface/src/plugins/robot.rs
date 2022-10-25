use std::time::SystemTime;
use bevy::prelude::*;
use message_io::network::Endpoint;
use common::state::{RobotState, RobotStateUpdate};
use crate::plugins::MateStage;

pub struct RobotPlugin;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RobotEvent>();
        app.init_resource::<Robot>();
        app.add_system_to_stage(MateStage::UpdateState, update_state);
    }
}

#[derive(Default)]
pub struct Robot(RobotState);
impl Robot {
    pub fn state(&self) -> &RobotState {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub enum RobotEvent {
    StateChanged(RobotStateUpdate),
    Ping(SystemTime, SystemTime),

    Connected(Endpoint),
    ConnectionFailed(Endpoint),
    Disconnected(Endpoint),
}

fn update_state(mut robot: ResMut<Robot>, mut events: EventReader<RobotEvent>) {
    for event in events.iter() {
        if let RobotEvent::StateChanged(update) = event {
            robot.0.update(update);
        }
    }
}
