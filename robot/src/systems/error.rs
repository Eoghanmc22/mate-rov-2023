use std::{sync::RwLock, thread::Scope};

use common::{protocol::Protocol, state::RobotState, LogLevel};
use tracing::error;

use crate::{event::Event, events::EventHandle, systems::System};

pub struct ErrorSystem;

impl System for ErrorSystem {
    fn start<'scope>(
        _robot: &'scope RwLock<RobotState>,
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        spawner.spawn(move || {
            for event in listner.into_iter() {
                match &*event {
                    Event::Error(err) => {
                        error!("Encountered error: {err:?}");
                        events.send(Event::PacketSend(Protocol::Log(
                            LogLevel::Error,
                            format!("Robot encountered error: {err}"),
                        )))
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }
}
