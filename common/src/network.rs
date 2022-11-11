//! A wrapper around the message-io crate that allows for more convenient messaging

use crate::protocol::Packet;
use anyhow::{bail, Context};
use message_io::network::{Endpoint, NetEvent, SendStatus, ToRemoteAddr, Transport};
use message_io::node::{NodeEvent, NodeHandler, NodeTask};
use std::fmt::{Debug, Formatter};
use std::net::ToSocketAddrs;
use std::time::{Duration, Instant};
use tracing::{error, info, trace};

const TIMEOUT: Duration = Duration::from_secs(10);

/// Representation of network handler
pub struct Network {
    handler: NodeHandler<WorkerEvent>,
    task: NodeTask,
}

impl Debug for Network {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Network")
            .field("handler", &"NodeHandler { .. }")
            .field("task", &"NodeTask { .. }")
            .finish()
    }
}

struct NetworkContext<EventHandler> {
    handler: NodeHandler<WorkerEvent>,
    connection: Option<Connection>,
    events: EventHandler,
}

impl<EventHandler: Debug> Debug for NetworkContext<EventHandler> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkContext")
            .field("handler", &"NodeHandler { .. }")
            .field("connection", &self.connection)
            .field("events", &self.events)
            .finish()
    }
}

pub trait EventHandler: Sized + Debug {
    /// Callback for handling received packets
    fn handle_packet(
        &mut self,
        handler: &NodeHandler<WorkerEvent>,
        connection: &Connection,
        packet: Packet,
    ) -> anyhow::Result<()>;

    /// Callback for handling new connections
    fn connected(
        &mut self,
        _endpoint: Endpoint,
        _handler: &NodeHandler<WorkerEvent>,
        _connection: &Connection,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    /// Callback for handling connections that failed
    fn connection_failed(&mut self, _endpoint: Endpoint) -> anyhow::Result<()> {
        Ok(())
    }
    /// Callback for handling when a peer disconnects
    fn disconnected(&mut self, _endpoint: Endpoint) -> anyhow::Result<()> {
        Ok(())
    }
}

impl EventHandler for () {
    fn handle_packet(
        &mut self,
        _handler: &NodeHandler<WorkerEvent>,
        _connection: &Connection,
        _packet: Packet,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Network {
    /// Create a network handler
    #[tracing::instrument]
    pub fn create<Events: EventHandler + Send + 'static>(events: Events) -> Self {
        trace!("Create Network");

        let (handler, listener) = message_io::node::split::<WorkerEvent>();

        let task = {
            let mut ctx = NetworkContext {
                handler: handler.clone(),
                connection: None,
                events,
            };

            listener.for_each_async(move |event| {
                handle_event(&mut ctx, event);
            })
        };

        Network { handler, task }
    }

    /// Start a server
    #[tracing::instrument]
    pub fn listen(&self, addrs: impl ToSocketAddrs + Debug) -> anyhow::Result<()> {
        info!("Starting server on {:?}", addrs);

        self.handler
            .network()
            .listen(Transport::FramedTcp, addrs)
            .context("Bind to port")?;

        Ok(())
    }

    /// Create connect to a peer
    #[tracing::instrument]
    pub fn connect(&self, addrs: impl ToRemoteAddr + Debug) -> anyhow::Result<()> {
        info!("Connecting to server on {:?}", addrs);

        self.handler
            .network()
            .connect(Transport::FramedTcp, addrs)
            .context("Connect to peer")?;

        Ok(())
    }

    /// Stops the network threads associated with this network handler
    #[tracing::instrument]
    pub fn stop(&mut self) {
        info!("Stopping handler");
        self.handler.stop();
        self.task.wait();
    }

    /// Sends a packet to all peers connected to this network handler
    #[tracing::instrument]
    pub fn send_packet(&self, packet: Packet) {
        trace!("Sending packet");
        self.handler.signals().send(WorkerEvent::Broadcast(packet));
    }

    pub fn handler(&self) -> &NodeHandler<WorkerEvent> {
        &self.handler
    }
}

/// Represents a connection with a peer
#[derive(Debug)]
pub struct Connection {
    endpoint: Endpoint,
    last_packet: Instant,
}

impl Connection {
    /// Serializes a packet and sends the packet to the connected peer
    #[tracing::instrument(skip(handler))]
    pub fn write_packet(
        &self,
        handler: &NodeHandler<WorkerEvent>,
        packet: Packet,
    ) -> anyhow::Result<()> {
        trace!(?packet);

        let data: Vec<u8> = (&packet).try_into().context("Encode packet")?;

        let ret = handler.network().send(self.endpoint, &data);
        match ret {
            SendStatus::Sent => {}
            err => bail!("Could not send packet: {:?}", err),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum WorkerEvent {
    Broadcast(Packet),
}

#[tracing::instrument(skip(network))]
fn handle_event<Events: EventHandler>(
    network: &mut NetworkContext<Events>,
    event: NodeEvent<WorkerEvent>,
) {
    trace!(?event);
    match event {
        NodeEvent::Network(event) => {
            let ret = handle_network_event(network, event);
            if let Err(err) = ret {
                error!("Error handling packet: {:?}", err)
            }
        }
        NodeEvent::Signal(event) => {
            let ret = handle_signal_event(network, event);
            if let Err(err) = ret {
                error!("Error handling signal: {:?}", err)
            }
        }
    }
}

#[tracing::instrument(skip(network))]
fn handle_network_event<Events: EventHandler>(
    network: &mut NetworkContext<Events>,
    event: NetEvent,
) -> anyhow::Result<()> {
    trace!(?event);
    match event {
        NetEvent::Accepted(endpoint, _resource_id) => {
            info!("Got connection from {}", endpoint);

            let new = Connection {
                endpoint,
                last_packet: Instant::now(),
            };
            let previous = network.connection.take();

            if let Some(previous) = previous {
                if previous.last_packet.elapsed() > TIMEOUT {
                    network
                        .events
                        .connected(endpoint, &network.handler, &new)
                        .context("Connected event")?;
                    network.connection = Some(new);
                } else {
                    network.connection = Some(previous);
                }
            } else {
                network
                    .events
                    .connected(endpoint, &network.handler, &new)
                    .context("Connected event")?;
                network.connection = Some(new);
            }
        }
        NetEvent::Connected(endpoint, success) => {
            if success {
                info!("Connected to {}", endpoint);

                let connection = Connection {
                    endpoint,
                    last_packet: Instant::now(),
                };

                network
                    .events
                    .connected(endpoint, &network.handler, &connection)
                    .context("Connected event")?;

                network.connection = Some(connection);
            } else {
                error!("Could not connect to endpoint: {}", endpoint);
                network
                    .events
                    .connection_failed(endpoint)
                    .context("Connection failed event")?;
            }
        }
        NetEvent::Message(endpoint, data) => {
            trace!("Message from endpoint: {}", endpoint);
            let packet = data.try_into().context("Decode packet")?;

            let Some(connection) = &mut network.connection else {
                bail!("Got packet from unknown endpoint");
            };

            trace!(?packet);

            connection.last_packet = Instant::now();

            network
                .events
                .handle_packet(&network.handler, connection, packet)
                .context("Handle packet event")?;
        }
        NetEvent::Disconnected(endpoint) => {
            info!("Endpoint {} disconnected", endpoint);
            network.connection = None;
            network
                .events
                .disconnected(endpoint)
                .context("Disconnected event")?;
        }
    }

    Ok(())
}

#[tracing::instrument(skip(network))]
fn handle_signal_event<Events: EventHandler>(
    network: &mut NetworkContext<Events>,
    event: WorkerEvent,
) -> anyhow::Result<()> {
    trace!(?event);
    match event {
        WorkerEvent::Broadcast(packet) => {
            if let Some(ref connection) = network.connection {
                connection
                    .write_packet(&network.handler, packet)
                    .context("Send packet")?;
            }
        }
    }

    Ok(())
}
