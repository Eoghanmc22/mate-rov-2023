use crate::event::Event as RobotEvent;
use crate::events::EventHandle;
use crate::systems::System;
use anyhow::{Context, Error};
use common::protocol::Protocol;
use common::types::LogLevel;
use fxhash::FxHashMap as HashMap;
use networking::{Event as NetEvent, Networking};
use std::net::ToSocketAddrs;
use std::thread::Scope;
use std::time::SystemTime;
use tracing::{debug, error, info, span, warn, Level};

const ADDRS: &str = "0.0.0.0:44444";

/// Handles the robot side of robot <-> surface communication.
pub struct NetworkSystem;

impl System for NetworkSystem {
    fn start<'scope>(
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
            let mut peers = HashMap::default();

            spawner.spawn(move || {
                let messenger = net.messenger();
                net.start(|event| match event {
                    NetEvent::Accepted(token, addrs) => {
                        info!("Accepted peer at {addrs}");

                        events.send(RobotEvent::PeerConnected(addrs));
                        peers.insert(token, addrs);
                    }
                    NetEvent::Data(token, packet) => {
                        // TODO Should any of this happen here?
                        match &packet {
                            Protocol::Log(level, msg) => match level {
                                LogLevel::Debug => debug!("Peer logged: `{msg}`"),
                                LogLevel::Info => info!("Peer logged: `{msg}`"),
                                LogLevel::Warn => warn!("Peer logged: `{msg}`"),
                                LogLevel::Error => error!("Peer logged: `{msg}`"),
                            },
                            Protocol::Ping(ping) => {
                                let response = Protocol::Pong(*ping, SystemTime::now());
                                let res = messenger
                                    .send_packet(token, response)
                                    .context("Send packet");
                                if let Err(err) = res {
                                    events.send(RobotEvent::Error(err));
                                }
                            }
                            _ => {}
                        }

                        events.send(RobotEvent::PacketRx(packet));
                    }
                    NetEvent::Error(token, err) => {
                        // TODO filter some errors
                        if let Some(token) = token {
                            events.send(RobotEvent::PeerDisconnected(peers.remove(&token)));
                        }

                        events.send(RobotEvent::Error(
                            Error::new(err).context("Networking error"),
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
                    if let RobotEvent::PacketTx(packet) = &*event {
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
