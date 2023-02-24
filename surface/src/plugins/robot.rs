use crate::plugins::networking::NetworkEvent;
use crate::plugins::MateStage;
use bevy::prelude::*;
use common::error::LogError;
use common::protocol::Protocol;
use common::store::adapters::{BackingType, TypeAdapter};
use common::store::{self, tokens, Key, Store, Token, Update, UpdateCallback};
use common::types::Camera;
use crossbeam::channel::{bounded, Receiver, Sender};
use fxhash::FxHashMap as HashMap;
use std::any::Any;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::SystemTime;

pub struct RobotPlugin;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RobotEvent>();
        app.add_event::<Update>();
        app.init_resource::<Robot>();
        app.init_resource::<Adapters>();
        // app.add_startup_system(mock_data);
        app.add_system_to_stage(MateStage::UpdateStateEarly, update_robot);
        app.add_system_to_stage(MateStage::UpdateStateLate, updates_to_packets);
    }
}

pub struct Adapters(HashMap<Key, Box<dyn TypeAdapter<BackingType> + Send + Sync>>);
impl Default for Adapters {
    fn default() -> Self {
        Self(tokens::generate_adaptors())
    }
}

pub struct Robot(Store<NotificationHandler>, Sender<Update>, Receiver<Update>);
impl Robot {
    pub fn store(&self) -> &Store<NotificationHandler> {
        &self.0
    }
}

impl Default for Robot {
    fn default() -> Self {
        let (tx, rx) = bounded(50);

        Robot(Store::new(NotificationHandler(tx.clone())), tx, rx)
    }
}

pub struct Updater(Sender<Update>);
impl Updater {
    pub fn emit_update<V: Any + Send + Sync>(&self, token: &Token<V>, value: V) {
        let update = store::create_update(token, value);
        self.0.send(update).log_error("Emit update failed");
    }
}

impl FromWorld for Updater {
    fn from_world(world: &mut World) -> Self {
        let robot = world.get_resource::<Robot>().expect("No `Robot` resource");

        Self(robot.1.clone())
    }
}

#[derive(Debug, Clone)]
pub enum RobotEvent {
    Store(Update),
    Ping(SystemTime, SystemTime),

    Connected(SocketAddr),
    Disconnected(SocketAddr),
}

fn mock_data(mut robot: ResMut<Robot>) {
    robot.0.insert(
        &tokens::CAMERAS,
        vec![
            Camera {
                name: "Test A".to_owned(),
                location: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4444),
            },
            Camera {
                name: "Test B".to_owned(),
                location: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4444),
            },
            Camera {
                name: "Test C".to_owned(),
                location: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4444),
            },
            Camera {
                name: "Test D".to_owned(),
                location: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4444),
            },
        ],
    );
}

fn update_robot(mut robot: ResMut<Robot>, mut events: EventReader<RobotEvent>) {
    for event in events.iter() {
        match event {
            RobotEvent::Store(update) => {
                robot.0.handle_update_shared(update);
            }
            RobotEvent::Connected(..) | RobotEvent::Disconnected(..) => {
                robot.0.reset();
            }
            _ => {}
        }
    }
}

fn updates_to_packets(
    adapters: Res<Adapters>,
    mut robot: ResMut<Robot>,
    mut net: EventWriter<NetworkEvent>,
) {
    // Bypass rust ownership issue
    let robot = &mut *robot;

    for update in robot.2.try_iter() {
        robot.0.handle_update_owned(&update);

        let (key, data) = update;
        let adapter = adapters.0.get(&key);

        if let Some(adapter) = adapter {
            match data {
                Some(data) => {
                    let data = adapter.serialize(&*data);

                    if let Some(data) = data {
                        net.send(NetworkEvent::SendPacket(Protocol::Store(
                            key.into(),
                            Some(data),
                        )));
                    }
                }
                None => {
                    net.send(NetworkEvent::SendPacket(Protocol::Store(key.into(), None)));
                }
            }
        }
    }
}

pub struct NotificationHandler(Sender<Update>);

impl UpdateCallback for NotificationHandler {
    fn call(&mut self, update: Update) {
        self.0
            .send(update)
            .log_error("NotificationHandler send failed");
    }
}
