use crate::plugins::networking::NetworkEvent;
use bevy::prelude::*;
use common::error::LogError;
use common::protocol::Protocol;
use common::store::adapters::{BackingType, TypeAdapter};
use common::store::{self, tokens, Key, Store, Token, Update, UpdateCallback};
use crossbeam::channel::{bounded, Receiver, Sender};
use fxhash::FxHashMap as HashMap;
use std::any::Any;
use std::net::SocketAddr;
use std::time::SystemTime;

pub struct RobotPlugin;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RobotEvent>();
        app.add_event::<Update>();
        app.init_resource::<Robot>();
        app.init_resource::<Adapters>();
        app.add_system(update_robot.in_base_set(CoreSet::PreUpdate));
        app.add_system(updates_to_packets.in_base_set(CoreSet::PostUpdate));
    }
}

#[derive(Resource)]
pub struct Adapters(HashMap<Key, Box<dyn TypeAdapter<BackingType> + Send + Sync>>);

impl Default for Adapters {
    fn default() -> Self {
        Self(tokens::generate_adaptors())
    }
}

#[derive(Resource)]
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

/// Way for systems to update store
/// For use with bevy's `Local` system argurment
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

/// Handle `RobotEvent`s
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

/// Handle writes to store and send the corresponding packets to the robot
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
