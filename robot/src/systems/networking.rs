use crate::event::Event as RobotEvent;
use crate::events::EventHandle;
use crate::systems::System;
use anyhow::Context;
use common::protocol::Protocol;
use common::state::RobotState;
use common::LogLevel;
use networking::{Event as NetEvent, Networking};
use std::net::ToSocketAddrs;
use std::time::SystemTime;
use std::{sync::RwLock, thread::Scope};
use tracing::{debug, error, info, span, warn, Level};

const ADDRS: &str = "localhost:44444";

pub struct NetworkSystem;

impl System for NetworkSystem {
    fn start<'scope>(
        _robot: &'scope RwLock<RobotState>,
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        let net = Networking::<Protocol>::new().context("Create Networking")?;
        let messenger = net.messenger();

        let addresses = ADDRS.to_socket_addrs().context("Resolve bind")?;
        for address in addresses {
            info!("Binding at {}", address);
            messenger.bind_at(address).context("Bind address")?;
        }

        {
            let mut events = events.clone();

            spawner.spawn(move || {
                let messenger = net.messenger();
                net.start(|event| match event {
                    NetEvent::Accepted(_token, addrs) => {
                        info!("Accepted peer at {addrs}");
                    }
                    NetEvent::Data(token, packet) => {
                        match packet {
                            Protocol::RobotState(updates) => {
                                events.send(RobotEvent::StateUpdate(updates));
                            }
                            Protocol::KVUpdate(_) => {
                                // Currently not used on the robot
                            }
                            Protocol::RequestSync => {
                                events.send(RobotEvent::StateRefresh);
                            }
                            Protocol::Log(level, msg) => match level {
                                LogLevel::Debug => debug!("Peer logged: `{msg}`"),
                                LogLevel::Info => info!("Peer logged: `{msg}`"),
                                LogLevel::Warn => warn!("Peer logged: `{msg}`"),
                                LogLevel::Error => error!("Peer logged: `{msg}`"),
                            },
                            Protocol::Ping(ping) => {
                                let response = Protocol::Pong(ping, SystemTime::now());
                                let res = messenger
                                    .send_packet(token, response)
                                    .context("Send packet");
                                if let Err(err) = res {
                                    events.send(RobotEvent::Error(err));
                                }
                            }
                            Protocol::Pong(_, _) => {
                                // Currently not used on the robot
                            }
                        }
                    }
                    NetEvent::Error(_token, err) => {
                        // TODO filter some errors
                        events.send(RobotEvent::Error(
                            anyhow::Error::new(err).context("Networking error"),
                        ));
                    }
                    _ => unreachable!(),
                })
            });
        }

        {
            let mut events = events.clone();
            spawner.spawn(move || {
                span!(Level::INFO, "Net forward thread");
                for event in listner.into_iter() {
                    if let RobotEvent::PacketSend(packet) = &*event {
                        let res = messenger
                            .brodcast_packet(packet.clone())
                            .context("Brodcast Packet");
                        if let Err(err) = res {
                            events.send(RobotEvent::Error(err));
                        }
                    }
                }
            });
        }

        Ok(())
    }
}
