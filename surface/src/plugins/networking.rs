use crate::plugins::robot::RobotEvent;
use anyhow::Context;
use bevy::prelude::*;
use common::error::LogErrorExt;
use common::protocol::Protocol;
use common::store::tokens;
use common::types::LogLevel;
use crossbeam::channel::{bounded, Receiver};
use fxhash::FxHashMap as HashMap;
use networking::{Event, Messenger, Networking};
use std::net::SocketAddr;
use std::thread;
use std::time::SystemTime;

use super::notification::{create_error_handler, Notification};

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NetworkEvent>();
        app.add_startup_system(setup_network.pipe(create_error_handler("Setup network error")));
        app.add_system(updates_to_events.in_base_set(CoreSet::PreUpdate));
        app.add_system(events_to_notifs.after(updates_to_events));
        app.add_system(events_to_packets.in_base_set(CoreSet::PostUpdate));
    }
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    SendPacket(Protocol),
    ConnectTo(SocketAddr),
}

#[derive(Resource)]
struct NetworkLink(Messenger<Protocol>, Receiver<RobotEvent>);

/// Create network thread
fn setup_network(mut commands: Commands) -> anyhow::Result<()> {
    let (tx, rx) = bounded(30);

    let network = Networking::new().context("Could start networking")?;

    let messenger = network.messenger();

    {
        let messenger = network.messenger();
        thread::spawn(move || {
            let mut clients = HashMap::default();
            let adapters = tokens::generate_adaptors();

            network.start(|event| match event {
                Event::Conected(token, addrs) => {
                    info!("Peer connected at {addrs}");

                    clients.insert(token, addrs);

                    messenger
                        .send_packet(token, Protocol::RequestSync)
                        .log_error("Could not send Message");
                    tx.send(RobotEvent::Connected(addrs))
                        .log_error("Could not send RobotEvent");
                }
                Event::Data(token, packet) => match packet {
                    Protocol::Store(key, data) => {
                        let key = key.to_owned().into();
                        let adapter = adapters.get(&key);

                        // TODO handle in robot system
                        if let Some(adapter) = adapter {
                            match data {
                                Some(data) => {
                                    let data = adapter.deserialize(&data);

                                    if let Some(data) = data {
                                        tx.send(RobotEvent::Store((key, Some(data.into()))))
                                            .log_error("Could not send RobotEvent");
                                    } else {
                                        error!("Could not deserialize for {key:?}");
                                    }
                                }
                                None => {
                                    tx.send(RobotEvent::Store((key, None)))
                                        .log_error("Could not send RobotEvent");
                                }
                            }
                        } else {
                            error!("No adapter found for {key:?}");
                        }
                    }
                    Protocol::RequestSync => {
                        let packet =
                            Protocol::Log(LogLevel::Warn, "RequestSync not implemented".to_owned());
                        messenger
                            .send_packet(token, packet)
                            .log_error("Could not send Message");
                    }
                    Protocol::Log(level, msg) => match level {
                        LogLevel::Debug => debug!("Peer logged: `{msg}`"),
                        LogLevel::Info => info!("Peer logged: `{msg}`"),
                        LogLevel::Warn => warn!("Peer logged: `{msg}`"),
                        LogLevel::Error => error!("Peer logged: `{msg}`"),
                    },
                    Protocol::Ping(ping) => {
                        let response = Protocol::Pong(ping, SystemTime::now());
                        messenger
                            .send_packet(token, response)
                            .log_error("Could not send Message");
                    }
                    Protocol::Pong(ping, pong) => {
                        tx.send(RobotEvent::Ping(ping, pong))
                            .log_error("Could not send RobotEvent");
                    }
                },
                Event::Error(token, error) => {
                    let addrs = token.and_then(|token| clients.remove(&token));
                    if let Some(addrs) = addrs {
                        error!("Network error, addrs: {addrs}, {error:?}");
                        tx.send(RobotEvent::Disconnected(addrs))
                            .log_error("Could not send RobotEvent");
                    } else {
                        error!("Network error, {error:?}");
                    }
                }
                _ => unreachable!(),
            });
        });
    }

    commands.insert_resource(NetworkLink(messenger, rx));

    Ok(())
}

/// Publishes updates from network thread as `RobotEvent`s
fn updates_to_events(mut events: EventWriter<RobotEvent>, net_link: Res<NetworkLink>) {
    events.send_batch(net_link.1.try_iter());
}

/// Processes `NetworkEvent`s and tells the network thread to send the corosponding packets
fn events_to_packets(mut events: EventReader<NetworkEvent>, net_link: Res<NetworkLink>) {
    for event in events.iter() {
        match event.to_owned() {
            NetworkEvent::SendPacket(packet) => {
                net_link
                    .0
                    .brodcast_packet(packet)
                    .log_error("Brodcast packet failed");
            }
            NetworkEvent::ConnectTo(peer) => {
                net_link.0.connect_to(peer).log_error("Connect to failed");
            }
        }
    }
}

/// Generate notifications for some robot events
// TODO this should be in robot.rs
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
