use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use anyhow::Context;
use message_io::network::Endpoint;
use message_io::node::NodeHandler;
use tracing::{error, info};
use crate::network::{Connection, EventHandler, WorkerEvent};
use crate::protocol::Packet;
use crate::state::RobotState;

impl EventHandler for () {
    fn handle_packet(&mut self, _handler: &NodeHandler<WorkerEvent>, _connection: &Connection, _packet: Packet) -> anyhow::Result<()> { Ok(()) }
}

#[derive(Debug)]
pub struct RobotHandler<Inner: EventHandler + Debug> {
    robot: Arc<RwLock<RobotState>>,
    inner: Inner
}

impl<Inner: EventHandler + Debug> RobotHandler<Inner> {
    pub fn new(robot: Arc<RwLock<RobotState>>, inner: Inner) -> Self {
        Self {
            robot,
            inner
        }
    }
}

impl<Inner: EventHandler + Debug> EventHandler for RobotHandler<Inner> {
    #[tracing::instrument(skip(handler))]
    fn handle_packet(&mut self, handler: &NodeHandler<WorkerEvent>, connection: &Connection, packet: Packet) -> anyhow::Result<()> {
        match packet.clone() {
            Packet::Ping(ping) => {
                let response = Packet::Pong(ping, SystemTime::now());
                connection.write_packet(handler, response).context("Send packet")?;
            }
            Packet::Pong(ping, pong) => {
                // TODO
            }
            Packet::StateUpdate(updates) => {
                match self.robot.write() {
                    Ok(mut robot) => {
                        for update in updates {
                            robot.update(update);
                        }
                    }
                    Err(error) => {
                        error!("Can't acquire lock: {error:?}");
                    }
                }
            }
            Packet::RequestSync => {
                match self.robot.read() {
                    Ok(robot) => {
                        let response = Packet::StateUpdate(robot.to_updates());
                        connection.write_packet(handler, response).context("Send packet")?;
                    }
                    Err(error) => {
                        error!("Can't acquire lock: {error:?}");
                    }
                }
            }
            Packet::Log(msg) => {
                info!("Peer logged: `{msg}`")
            }
        }

        self.inner.handle_packet(handler, connection, packet)
    }

    #[tracing::instrument]
    fn connected(&mut self, endpoint: Endpoint) -> anyhow::Result<()> {
        self.inner.connected(endpoint)
    }

    #[tracing::instrument]
    fn connection_failed(&mut self, endpoint: Endpoint) -> anyhow::Result<()> {
        self.inner.connection_failed(endpoint)
    }

    #[tracing::instrument]
    fn disconnected(&mut self, endpoint: Endpoint) -> anyhow::Result<()> {
        self.inner.disconnected(endpoint)
    }
}