use std::sync::{Arc, RwLock};
use anyhow::Context;
use rppal::gpio::Gpio;
use common::handler::RobotHandler;
use common::network::Network;
use common::protocol::Packet;
use common::state::{RobotState, RobotStateUpdate};
use crate::systems::RobotSystem;

const ADDRS: &str = "0.0.0.0:44444";

pub struct NetworkSystem(Network);

impl RobotSystem for NetworkSystem {
    fn start(robot: Arc<RwLock<RobotState>>, _gpio: Gpio) -> anyhow::Result<Self> {
        let handler = ();
        let network = Network::create(RobotHandler::new(robot, handler));
        network.listen(ADDRS).context("Start server")?;

        Ok(NetworkSystem(network))
    }

    fn on_update(&self, update: RobotStateUpdate, _robot: &mut RobotState) {
        self.0.send_packet(Packet::StateUpdate(vec![update]));
    }
}