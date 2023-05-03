use std::thread::Scope;

use common::{protocol::Protocol, types::LogLevel};
use tracing::{error, span, Level};

use crate::{event::Event, events::EventHandle, systems::System, SystemId};

/// Handles error events
pub struct ErrorSystem;

impl System for ErrorSystem {
    const ID: SystemId = SystemId::Error;

    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        spawner.spawn(move || {
            span!(Level::ERROR, "Error handler");

            for event in listner {
                match &*event {
                    Event::Error(err) => {
                        error!("Encountered error: {err:?}");
                        events.send(Event::PacketTx(Protocol::Log(
                            LogLevel::Error,
                            format!("Robot encountered error: {err}"),
                        )));
                    }
                    Event::Exit => {
                        return;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }
}
