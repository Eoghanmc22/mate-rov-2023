use std::time::SystemTime;
use bevy::prelude::*;
use message_io::network::Endpoint;
use common::protocol::Packet;
use common::state::{RobotState, RobotStateUpdate};
use crate::plugins::MateStage;
use crate::plugins::networking::NetworkEvent;

pub struct RobotPlugin;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RobotEvent>();
        app.add_event::<RobotStateUpdate>();
        app.init_resource::<Robot>();
        app.add_system_to_stage(MateStage::UpdateStateEarly, update_robot);
        app.add_system_to_stage(MateStage::UpdateStateLate, updates_to_packets);
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

fn update_robot(mut robot: ResMut<Robot>, mut events: EventReader<RobotEvent>) {
    for event in events.iter() {
        if let RobotEvent::StateChanged(update) = event {
            robot.0.update(update);
        }
    }
}

fn updates_to_packets(mut updates: EventReader<RobotStateUpdate>, mut net: EventWriter<NetworkEvent>) {
    for update in updates.iter() {
        net.send(NetworkEvent::SendPacket(Packet::StateUpdate(vec![update.clone()])));
    }
}
