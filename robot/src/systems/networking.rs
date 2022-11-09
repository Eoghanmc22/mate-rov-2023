use crate::systems::RobotSystem;
use anyhow::Context;
use common::network::{Connection, EventHandler, Network, WorkerEvent};
use common::protocol::Packet;
use common::state::{RobotState, RobotStateUpdate};
use message_io::node::NodeHandler;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use tracing::{error, info};

const ADDRS: &str = "0.0.0.0:44444";

pub struct NetworkSystem(Network);

impl RobotSystem for NetworkSystem {
    #[tracing::instrument]
    fn start(robot: Arc<RwLock<RobotState>>) -> anyhow::Result<Self> {
        info!("Starting networking system");
        let network = Network::create(NetworkHandler(robot));
        network.listen(ADDRS).context("Start server")?;

        Ok(NetworkSystem(network))
    }

    fn on_update(&self, update: &RobotStateUpdate, _robot: &mut RobotState) {
        self.0
            .send_packet(Packet::StateUpdate(vec![update.clone()]));
    }
}

#[derive(Debug)]
struct NetworkHandler(Arc<RwLock<RobotState>>);

impl EventHandler for NetworkHandler {
    #[tracing::instrument(skip(handler))]
    fn handle_packet(
        &mut self,
        handler: &NodeHandler<WorkerEvent>,
        connection: &Connection,
        packet: Packet,
    ) -> anyhow::Result<()> {
        match packet.clone() {
            Packet::StateUpdate(updates) => match self.0.write() {
                Ok(mut robot) => {
                    for update in updates {
                        robot.update(&update);
                    }
                }
                Err(error) => {
                    error!("Can't acquire lock: {error:?}");
                }
            },
            Packet::RequestSync => match self.0.read() {
                Ok(robot) => {
                    let response = Packet::StateUpdate(robot.to_updates());
                    connection
                        .write_packet(handler, response)
                        .context("Send packet")?;
                }
                Err(error) => {
                    error!("Can't acquire lock: {error:?}");
                }
            },
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
                // TODO
            }
        }

        Ok(())
    }
}
