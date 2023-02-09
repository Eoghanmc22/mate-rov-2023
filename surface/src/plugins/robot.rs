use crate::plugins::networking::NetworkEvent;
use crate::plugins::MateStage;
use bevy::prelude::*;
use common::kvdata::Key::Cameras;
use common::kvdata::{Store, Value};
use common::protocol::Protocol;
use common::state::{RobotState, RobotStateUpdate};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::SystemTime;

pub struct RobotPlugin;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RobotEvent>();
        app.add_event::<RobotStateUpdate>();
        app.init_resource::<Robot>();
        // app.add_startup_system(mock_data);
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

    Connected(SocketAddr),
    Disconnected(SocketAddr),
}

fn mock_data(mut robot: ResMut<Robot>) {
    robot.1.insert(
        Cameras,
        Value::Cameras(vec![
            (
                "Test A".to_owned(),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4444),
            ),
            (
                "Test B".to_owned(),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4444),
            ),
            (
                "Test C".to_owned(),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4444),
            ),
            (
                "Test D".to_owned(),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4444),
            ),
        ]),
    );
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
        net.send(NetworkEvent::SendPacket(Protocol::RobotState(vec![
            update.clone()
        ])));
    }
}
