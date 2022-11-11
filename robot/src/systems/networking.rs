use crate::event::Event;
use crate::events::EventHandle;
use crate::systems::System;
use anyhow::Context;
use common::network::{Connection, EventHandler, Network, WorkerEvent};
use common::protocol::Packet;
use common::state::RobotState;
use message_io::node::NodeHandler;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::SystemTime;
use tracing::{info, span, Level};

const ADDRS: &str = "0.0.0.0:44444";

pub struct NetworkSystem(Network);

impl System for NetworkSystem {
    #[tracing::instrument]
    fn start(robot: Arc<RwLock<RobotState>>, mut events: EventHandle) -> anyhow::Result<()> {
        info!("Starting networking system");

        let listner = events.take_listner().unwrap();

        let network = Network::create(NetworkHandler(events));
        network.listen(ADDRS).context("Start server")?;

        let handler = network.handler().to_owned();
        thread::spawn(move || {
            span!(Level::INFO, "Net forward thread");
            for event in listner.into_iter() {
                if let Event::PacketSend(packet) = &*event {
                    handler
                        .signals()
                        .send(WorkerEvent::Broadcast(packet.clone()));
                }
            }
        });

        Ok(())
    }
}

#[derive(Debug)]
struct NetworkHandler(EventHandle);

impl EventHandler for NetworkHandler {
    #[tracing::instrument(skip(handler))]
    fn handle_packet(
        &mut self,
        handler: &NodeHandler<WorkerEvent>,
        connection: &Connection,
        packet: Packet,
    ) -> anyhow::Result<()> {
        match packet {
            Packet::RobotState(updates) => {
                self.0.send(Event::StateUpdate(updates));
            }
            Packet::KVUpdate(_) => {
                // Currently not used on the robot
            }
            Packet::RequestSync => {
                self.0.send(Event::StateRefresh);
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
            Packet::Pong(_, _) => {
                // Currently not used on the robot
            }
        }

        Ok(())
    }
}
