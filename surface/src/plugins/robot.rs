use crate::plugins::networking::NetworkEvent;
use crate::plugins::MateStage;
use bevy::prelude::*;
use common::kvdata::{Store, Value};
use common::protocol::Packet;
use common::state::{RobotState, RobotStateUpdate};
use message_io::network::Endpoint;
use std::time::SystemTime;

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
pub struct Robot(RobotState, Store);
impl Robot {
    pub fn state(&self) -> &RobotState {
        &self.0
    }

    pub fn store(&self) -> &Store {
        &self.1
    }
}

#[derive(Debug, Clone)]
pub enum RobotEvent {
    StateChanged(RobotStateUpdate),
    KVChanged(Value),
    Ping(SystemTime, SystemTime),

    Connected(Endpoint),
    ConnectionFailed(Endpoint),
    Disconnected(Endpoint),
}

fn update_robot(mut robot: ResMut<Robot>, mut events: EventReader<RobotEvent>) {
    for event in events.iter() {
        match event {
            RobotEvent::StateChanged(update) => {
                robot.0.update(update);
            }
            RobotEvent::KVChanged(value) => {
                robot.1.insert(value.to_key(), value.clone());
            }
            RobotEvent::Connected(..) | RobotEvent::Disconnected(..) => {
                *robot = Default::default();
            }
            _ => {}
        }
    }
}

fn updates_to_packets(
    mut updates: EventReader<RobotStateUpdate>,
    mut net: EventWriter<NetworkEvent>,
) {
    for update in updates.iter() {
        net.send(NetworkEvent::SendPacket(Packet::RobotState(vec![
            update.clone()
        ])));
    }
}
