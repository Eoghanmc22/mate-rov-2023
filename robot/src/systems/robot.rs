use std::{sync::RwLock, thread::Scope};

use common::{protocol::Packet, state::RobotState};
use tracing::{span, Level};

use crate::{event::Event, events::EventHandle};

use super::System;

pub struct RobotSystem;

impl System for RobotSystem {
    fn start(
        robot: &RwLock<RobotState>,
        mut events: EventHandle,
        _spawner: &Scope,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        span!(Level::INFO, "Robot update thread");
        for event in listner.into_iter() {
            match &*event {
                Event::StateUpdate(updates) => {
                    let mut packets = Vec::new();
                    {
                        let mut robot = robot.write().expect("Accquire write");
                        for update in updates {
                            if robot.update(update) {
                                packets.push(update.to_owned())
                            }
                        }
                    }

                    events.send(Event::PacketSend(Packet::RobotState(packets)));
                }
                Event::StateRefresh => {
                    let robot = robot.read().expect("Accquire read");
                    let updates = robot.to_updates();
                    events.send(Event::PacketSend(Packet::RobotState(updates)));
                }
                _ => {}
            }
        }

        Ok(())
    }
}
