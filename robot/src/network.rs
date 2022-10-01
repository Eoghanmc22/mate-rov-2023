use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::time::{Instant, SystemTime};
use anyhow::{bail, Context};
use message_io::network::{Endpoint, NetEvent, NetworkController, SendStatus, Transport};
use message_io::node::{NodeEvent, NodeHandler, NodeTask};
use tracing::{error, info};
use common::*;
use common::types::{Filter, Role};
use crate::robot;

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

        handler.network().listen(Transport::FramedTcp, "0.0.0.0:44444").context("Could not bind to port")?;

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
    last_ping: SystemTime,
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
                last_ping: SystemTime::now(),
            });
        }
        NetEvent::Message(endpoint, data) => {
            if let Some(connection) = server.clients.get_mut(&endpoint) {
                let packet = data.try_into().context("Could not decode packet")?;
                handle_packet(server, connection, packet)?;
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
            // Only write the buffer once, cant use sent packet
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
fn handle_packet(server: &mut ServerContext, client: &mut Connection, packet: robot_bound::Packet) -> anyhow::Result<()> {
    match packet {
        robot_bound::Packet::Arm => {
            if let Role::Controller = client.role {
                robot::ROBOT.armed().store(true);
            }
        },
        robot_bound::Packet::Disarm => {
            if let Role::Controller = client.role {
                robot::ROBOT.armed().store(false);
            }
        },
        robot_bound::Packet::MovementCommand(absolute, movement) => {
            if let Role::Controller = client.role {
                if absolute {
                    unimplemented!();
                } else {
                    robot::ROBOT.movement().store(movement.zip(Some(Instant::now())));
                }
            }
        },
        robot_bound::Packet::DepthPid(depth) => {
            if let Role::Controller = client.role {
                robot::ROBOT.depth_target().store(depth.zip(Some(Instant::now())));
            }
        },
        robot_bound::Packet::SetFilter(filter) => {
            client.filter = filter;
        },
        robot_bound::Packet::Ping(ping) => {
            let time = SystemTime::now();
            client.last_ping = time;

            let response = surface_bound::Packet::Pong(ping, time.duration_since(SystemTime::UNIX_EPOCH).context("Could calculate wall clock time")?.as_millis());
            send_packet(client, server.handler.network(), response).context("Could send packet")?;
        },
    }

    Ok(())
}

#[tracing::instrument]
pub fn send_packet(client: &Connection, handler: &NetworkController, packet: surface_bound::Packet) -> anyhow::Result<()> {
    let data: Vec<u8> = (&packet).try_into().context("Could not encode packet")?;

    match handler.send(client.endpoint, &data) {
        SendStatus::Sent => {}
        err => bail!("Could not send packet")
    }

    Ok(())
}
