use crate::plugins::robot::RobotEvent;
use crate::plugins::MateStage;
use anyhow::Context;
use bevy::prelude::*;
use common::network::{Connection, EventHandler, Network, WorkerEvent};
use common::protocol::Packet;
use crossbeam::channel::{bounded, Receiver, Sender};
use message_io::network::{Endpoint, RemoteAddr};
use message_io::node::NodeHandler;
use std::time::SystemTime;

use super::notification::Notification;

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NetworkEvent>();
        app.add_startup_system(setup_network);
        app.add_system_to_stage(MateStage::NetworkRead, updates_to_events);
        app.add_system_to_stage(MateStage::NetworkRead, handle_connect_fail);
        app.add_system_to_stage(MateStage::NetworkWrite, events_to_packets);
    }
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    SendPacket(Packet),
    ConnectTo(RemoteAddr),
}

struct NetworkLink(Network, Receiver<RobotEvent>);

fn setup_network(mut commands: Commands) {
    let (tx, rx) = bounded(30);
    let network = Network::create(NetworkHandler(tx));
    commands.insert_resource(NetworkLink(network, rx));
}

fn updates_to_events(mut events: EventWriter<RobotEvent>, net_link: Res<NetworkLink>) {
    events.send_batch(net_link.1.try_iter());
}

fn events_to_packets(
    mut events: EventReader<NetworkEvent>,
    net_link: Res<NetworkLink>,
    mut errors: EventWriter<Notification>,
) {
    for event in events.iter() {
        match event.to_owned() {
            NetworkEvent::SendPacket(packet) => {
                net_link.0.send_packet(packet);
            }
            NetworkEvent::ConnectTo(peer) => {
                if let Err(error) = net_link.0.connect(peer) {
                    errors.send(Notification::Error(
                        "Could not connect to robot".to_owned(),
                        error,
                    ));
                }
            }
        }
    }
}

fn handle_connect_fail(mut events: EventReader<RobotEvent>, mut notifs: EventWriter<Notification>) {
    for event in events.iter() {
        match event {
            RobotEvent::ConnectionFailed(_) => {
                notifs.send(Notification::SimpleError("Connection Failed".to_owned()))
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
struct NetworkHandler(Sender<RobotEvent>);

impl EventHandler for NetworkHandler {
    fn handle_packet(
        &mut self,
        handler: &NodeHandler<WorkerEvent>,
        connection: &Connection,
        packet: Packet,
    ) -> anyhow::Result<()> {
        match packet {
            Packet::RobotState(updates) => {
                for update in updates {
                    self.0
                        .send(RobotEvent::StateChanged(update))
                        .context("Send state update")?;
                }
            }
            Packet::KVUpdate(value) => {
                self.0
                    .send(RobotEvent::KVChanged(value))
                    .context("Send kv update")?;
            }
            Packet::RequestSync => {
                connection
                    .write_packet(
                        handler,
                        Packet::Log("RequestSync not implemented".to_owned()),
                    )
                    .context("Send packet")?;
            }
            Packet::Log(msg) => {
                info!("Peer logged: `{msg}`");
            }
            Packet::Ping(ping) => {
                let response = Packet::Pong(ping, SystemTime::now());
                connection
                    .write_packet(handler, response)
                    .context("Send packet")?;
            }
            Packet::Pong(ping, pong) => {
                self.0
                    .send(RobotEvent::Ping(ping, pong))
                    .context("Send pong")?;
            }
        }

        Ok(())
    }

    fn connected(
        &mut self,
        endpoint: Endpoint,
        handler: &NodeHandler<WorkerEvent>,
        connection: &Connection,
    ) -> anyhow::Result<()> {
        connection
            .write_packet(handler, Packet::RequestSync)
            .context("Send packet")?;
        self.0
            .send(RobotEvent::Connected(endpoint))
            .context("Send update")
    }

    fn connection_failed(&mut self, endpoint: Endpoint) -> anyhow::Result<()> {
        self.0
            .send(RobotEvent::ConnectionFailed(endpoint))
            .context("Send update")
    }

    fn disconnected(&mut self, endpoint: Endpoint) -> anyhow::Result<()> {
        self.0
            .send(RobotEvent::Disconnected(endpoint))
            .context("Send update")
    }
}
