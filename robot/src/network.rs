use std::collections::HashMap;
use message_io::network::{Endpoint, NetEvent, SendStatus, Transport};
use message_io::node::{NodeEvent, NodeHandler, NodeTask};
use tracing::{error, info};
use common::*;
use common::robot_bound::Packet;

pub struct Server {
    handler: NodeHandler<WorkerEvent>,
    task: NodeTask
}

impl Server {
    pub fn start() -> anyhow::Result<Self> {
        let (handler, listener) = message_io::node::split::<WorkerEvent>();

        let mut clients = HashMap::new();
        handler.network().listen(Transport::FramedTcp, "0.0.0.0:44444")?;

        let task = {
            let handler = handler.clone();
            listener.for_each_async(|event| {
                match event {
                    NodeEvent::Network(event) => {
                        match event {
                            NetEvent::Connected(_, _) => unreachable!(),
                            NetEvent::Accepted(endpoint, _resource_id) => {
                                info!("Got connection from {}", endpoint);
                                clients.insert(endpoint, Connection {
                                    endpoint: endpoint.clone(),
                                    handler: handler.clone()
                                });
                            }
                            NetEvent::Message(endpoint, data) => {
                                if let Some(connection) = clients.get_mut(&endpoint) {
                                    let packet = data.try_into()?;
                                    handle_packet(connection, packet)?;
                                } else {
                                    error!("Received packet from unknown endpoint: {:?}", endpoint);
                                }
                            }
                            NetEvent::Disconnected(endpoint) => {
                                clients.remove(&endpoint);
                            }
                        }
                    }
                    NodeEvent::Signal(event) => {
                        match event {
                            WorkerEvent::Broadcast(packet) => {
                                let buffer: Vec<u8> = (&packet).try_into()?;
                                for client in clients.keys().copied() {
                                    match handler.network().send(client, &buffer) {
                                        SendStatus::Sent => {}
                                        err => error!("Error sending packet: {:?}", err)
                                    }
                                }
                            }
                        }
                    }
                }
            })
        };

        Ok(Server {
            handler,
            task
        })
    }
}

pub struct Connection {
    endpoint: Endpoint,
    handler: NodeHandler<WorkerEvent>
}

pub enum WorkerEvent {
    Broadcast(surface_bound::Packet)
    // TODO
}

fn handle_packet(client: &mut Connection, packet: robot_bound::Packet) -> anyhow::Result<()> {
    match packet {
        Packet::Arm => todo!(),
        Packet::Disarm => todo!(),
        Packet::MovementCommand(_, _) => todo!(),
        Packet::DepthPid(_, _) => todo!(),
        Packet::SetFilter(_) => todo!(),
        Packet::Ping(_) => todo!(),
    }
}
