use std::collections::HashMap;
use std::sync::RwLock;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{NodeEvent, NodeHandler, NodeTask};
use tracing::{error, info};
use common::*;

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
                                if ok {
                                    info!("Got connection from {}", endpoint);
                                    clients.insert(endpoint, Connection {
                                        endpoint: endpoint.clone(),
                                        handler: handler.clone()
                                    });
                                }
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
                                clients.remove(&Endpoint);
                            }
                        }
                    }
                    NodeEvent::Signal(event) => {
                        match event {
                            WorkerEvent::Broadcast(packet) => {
                                let buffer = packet.into();
                                for client in clients.keys().copied() {
                                    if let Err(error) = handler.network().send(client, buffer) {
                                        error!("Error sending packet: {:?}", error);
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

#[derive(Debug)]
pub struct Connection {
    endpoint: Endpoint,
    handler: NodeHandler<WorkerEvent>
}

pub enum WorkerEvent {
    Broadcast(surface_bound::Packet)
}

fn handle_packet(client: &mut Connection, packet: robot_bound::Packet) -> anyhow::Result<()> {
    match packet {

    }
}
