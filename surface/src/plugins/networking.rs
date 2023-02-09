use crate::plugins::robot::RobotEvent;
use crate::plugins::MateStage;
use bevy::prelude::*;
use common::protocol::Protocol;
use common::LogLevel;
use crossbeam::channel::{bounded, Receiver};
use networking::{Event, Messenger, Networking};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::thread;
use std::time::SystemTime;

use super::notification::Notification;

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NetworkEvent>();
        app.add_startup_system(setup_network);
        app.add_system_to_stage(MateStage::NetworkRead, updates_to_events);
        app.add_system_to_stage(MateStage::NetworkRead, events_to_notifs);
        app.add_system_to_stage(MateStage::NetworkWrite, events_to_packets);
    }
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    SendPacket(Protocol),
    ConnectTo(SocketAddr),
}

struct NetworkLink(Messenger<Protocol>, Receiver<RobotEvent>);

fn setup_network(mut commands: Commands, mut errors: EventWriter<Notification>) {
    let (tx, rx) = bounded(30);

    let network = Networking::new();
    let network = match network {
        Ok(network) => network,
        Err(err) => {
            errors.send(Notification::Error(
                "Could start networking".to_owned(),
                anyhow::Error::new(err),
            ));
            return;
        }
    };

    let messenger = network.messenger();

    {
        let messenger = network.messenger();
        thread::spawn(move || {
            let mut clients = HashMap::new();

            network.start(|event| match event {
                Event::Conected(token, addrs) => {
                    info!("Peer connected at {addrs}");

                    clients.insert(token, addrs);

                    let _ = messenger.send_packet(token, Protocol::RequestSync);
                    let _ = tx.send(RobotEvent::Connected(addrs));
                }
                Event::Data(token, packet) => match packet {
                    Protocol::RobotState(updates) => {
                        for update in updates {
                            let _ = tx.send(RobotEvent::StateChanged(update));
                        }
                    }
                    Protocol::KVUpdate(value) => {
                        let _ = tx.send(RobotEvent::KVChanged(value));
                    }
                    Protocol::RequestSync => {
                        let packet =
                            Protocol::Log(LogLevel::Warn, "RequestSync not implemented".to_owned());
                        let _ = messenger.send_packet(token, packet);
                    }
                    Protocol::Log(level, msg) => match level {
                        LogLevel::Debug => debug!("Peer logged: `{msg}`"),
                        LogLevel::Info => info!("Peer logged: `{msg}`"),
                        LogLevel::Warn => warn!("Peer logged: `{msg}`"),
                        LogLevel::Error => error!("Peer logged: `{msg}`"),
                    },
                    Protocol::Ping(ping) => {
                        let response = Protocol::Pong(ping, SystemTime::now());
                        let _ = messenger.send_packet(token, response);
                    }
                    Protocol::Pong(ping, pong) => {
                        let _ = tx.send(RobotEvent::Ping(ping, pong));
                    }
                },
                Event::Error(token, error) => {
                    let addrs = token.and_then(|token| clients.remove(&token));
                    if let Some(addrs) = addrs {
                        error!("Network error, addrs: {addrs}, {error:?}");
                        let _ = tx.send(RobotEvent::Disconnected(addrs));
                    } else {
                        error!("Network error, {error:?}");
                    }
                }
                _ => unreachable!(),
            });
        });
    }

    commands.insert_resource(NetworkLink(messenger, rx));
}

fn updates_to_events(mut events: EventWriter<RobotEvent>, net_link: Res<NetworkLink>) {
    events.send_batch(net_link.1.try_iter());
}

fn events_to_packets(
    mut events: EventReader<NetworkEvent>,
    net_link: Res<NetworkLink>,
    _errors: EventWriter<Notification>,
) {
    for event in events.iter() {
        match event.to_owned() {
            NetworkEvent::SendPacket(packet) => {
                let _ = net_link.0.brodcast_packet(packet);
            }
            NetworkEvent::ConnectTo(peer) => {
                let _ = net_link.0.connect_to(peer);
            }
        }
    }
}

fn events_to_notifs(mut events: EventReader<RobotEvent>, mut notifs: EventWriter<Notification>) {
    for event in events.iter() {
        match event {
            RobotEvent::Connected(addr) => {
                notifs.send(Notification::Info(
                    "Robot Connected".to_owned(),
                    format!("Peer: {addr}"),
                ));
            }
            RobotEvent::Disconnected(addr) => {
                notifs.send(Notification::Info(
                    "Robot Disconnected".to_owned(),
                    format!("Peer: {addr}"),
                ));
            }
            _ => {}
        }
    }
}
