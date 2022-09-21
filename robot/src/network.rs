use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::time::Instant;
use anyhow::Context;
use message_io::network::{Endpoint, NetEvent, SendStatus, Transport};
use message_io::node::{NodeEvent, NodeHandler, NodeTask};
use tracing::{error, info};
use common::*;
use common::types::{Filter, Role};

pub struct Server {
    handler: NodeHandler<WorkerEvent>,
    task: NodeTask
}

struct ServerContext {
    handler: NodeHandler<WorkerEvent>,
    clients: HashMap<Endpoint, Connection>,
}

impl Debug for ServerContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <HashMap<Endpoint, Connection> as Debug>::fmt(&self.clients, f)
    }
}

impl Server {
    pub fn start() -> anyhow::Result<Self> {
        let (handler, listener) = message_io::node::split::<WorkerEvent>();

        handler.network().listen(Transport::FramedTcp, "0.0.0.0:44444")?;

        let task = {
            let mut server = ServerContext {
                handler: handler.clone(),
                clients: HashMap::new(),
            };

            listener.for_each_async(move |event| {
                handle_event(&mut server, event);
            })
        };

        Ok(Server {
            handler,
            task
        })
    }
}

#[derive(Debug)]
pub struct Connection {
    endpoint: Endpoint,
    role: Role,
    filter: Filter,
    last_ping: Instant,
}

#[derive(Debug)]
pub enum WorkerEvent {
    Broadcast(surface_bound::Packet)
    // TODO
}

#[tracing::instrument]
fn handle_event(server: &mut ServerContext, event: NodeEvent<WorkerEvent>) {
    match event {
        NodeEvent::Network(event) => {
            let rst = handle_network_event(server, event);
            if let Err(err) = rst {
                error!("Error handling packet: {:?}", err)
            }
        }
        NodeEvent::Signal(event) => {
            let rst = handle_signal_event(server, event);
            if let Err(err) = rst {
                error!("Error handling signal: {:?}", err)
            }
        }
    }
}

#[tracing::instrument]
fn handle_network_event(server: &mut ServerContext, event: NetEvent) -> anyhow::Result<()> {
    match event {
        NetEvent::Connected(_, _) => unreachable!(),
        NetEvent::Accepted(endpoint, _resource_id) => {
            info!("Got connection from {}", endpoint);
            server.clients.insert(endpoint, Connection {
                endpoint: endpoint.clone(),
                role: Role::Monitor,
                filter: Filter::empty(),
                last_ping: Instant::now(),
            });
        }
        NetEvent::Message(endpoint, data) => {
            if let Some(connection) = server.clients.get_mut(&endpoint) {
                let packet = data.try_into()?;
                handle_packet(connection, packet)?;
            } else {
                error!("Received packet from unknown endpoint: {:?}", endpoint);
            }
        }
        NetEvent::Disconnected(endpoint) => {
            server.clients.remove(&endpoint);
        }
    }

    Ok(())
}

#[tracing::instrument]
fn handle_signal_event(server: &mut ServerContext, event: WorkerEvent) -> anyhow::Result<()> {
    match event {
        WorkerEvent::Broadcast(packet) => {
            let buffer: Vec<u8> = (&packet).try_into().context("Could not encode packet")?;
            for client in server.clients.keys().copied() {
                match server.handler.network().send(client, &buffer) {
                    SendStatus::Sent => {}
                    err => error!("Error sending packet: {:?}", err)
                }
            }
        }
    }

    Ok(())
}

#[tracing::instrument]
fn handle_packet(client: &mut Connection, packet: robot_bound::Packet) -> anyhow::Result<()> {
    match packet {
        robot_bound::Packet::Arm => todo!(),
        robot_bound::Packet::Disarm => todo!(),
        robot_bound::Packet::MovementCommand(_, _) => todo!(),
        robot_bound::Packet::DepthPid(_, _) => todo!(),
        robot_bound::Packet::SetFilter(_) => todo!(),
        robot_bound::Packet::Ping(_) => todo!(),
    }
}
