use std::thread::Scope;

use common::protocol::Protocol;
use tracing::{debug, span, Level};

use crate::{event::Event, events::EventHandle, systems::System};

/// System for debugging
/// Prings all messages on the event bus
pub struct LogEventSystem;

impl System for LogEventSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        spawner.spawn(move || {
            span!(Level::DEBUG, "Event logger");

            for event in listner.into_iter() {
                match &*event {
                    // The sensors emit thousands of events per second
                    // Hide this
                    Event::PacketTx(Protocol::Store(key, _)) if key.contains("sensor") => {}
                    Event::PacketRx(Protocol::Store(key, _)) if key.contains("sensor") => {}
                    Event::Store((key, _)) if key.as_str().contains("sensor") => {}

                    Event::PacketTx(Protocol::Store(key, _)) => {
                        debug!("PacketTx(Store({key}, ..))");
                    }
                    Event::PacketRx(Protocol::Store(key, _)) => {
                        debug!("PacketRx(Store({key}, ..))");
                    }
                    event => {
                        debug!("{event:?}");
                    }
                }
            }
        });

        Ok(())
    }
}
