use std::thread::Scope;

use common::{protocol::Protocol, LogLevel};
use tracing::error;

use crate::{event::Event, events::EventHandle, systems::System};

pub struct ErrorSystem;

impl System for ErrorSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        spawner.spawn(move || {
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
