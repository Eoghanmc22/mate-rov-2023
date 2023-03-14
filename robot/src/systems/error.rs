use std::thread::Scope;

use common::{protocol::Protocol, LogLevel};
use tracing::{error, span, Level};

use crate::{event::Event, events::EventHandle, systems::System};

/// Handles error events
pub struct ErrorSystem;

impl System for ErrorSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        spawner.spawn(move || {
            span!(Level::ERROR, "Error handler");

            for event in listner.into_iter() {
                match &*event {
                    Event::Error(err) => {
                        error!("Encountered error: {err:?}");
                        events.send(Event::PacketTx(Protocol::Log(
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
