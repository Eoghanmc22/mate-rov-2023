use std::thread::Scope;

use common::{protocol::Protocol, store::tokens};
use tracing::{span, Level};

use crate::{event::Event, events::EventHandle};

use super::System;

pub struct StoreSystem;

impl System for StoreSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        spawner.spawn(move || {
            span!(Level::INFO, "Robot update thread");

            let adapters = tokens::generate_adaptors();

            for event in listner.into_iter() {
                match &*event {
                    Event::PacketRx(Protocol::Store(key, data)) => {
                        let adapter = adapters.get(key.as_str());

                        if let Some(adapter) = adapter {
                            match data {
                                Some(data) => {
                                    let data = adapter.deserialize(data);

                                    if let Some(data) = data {
                                        events.send(Event::Store((key, Some(data.into()))));
                                    }
                                }
                                None => events.send(Event::Store((key, None))),
                            }
                        }
                    }
                    Event::Store((key, data)) => {
                        let adapter = adapters.get(key);

                        if let Some(adapter) = adapter {
                            match data {
                                Some(data) => {
                                    let data = adapter.serialize(data);

                                    if let Some(data) = data {
                                        events.send(Event::PacketTx(Protocol::Store(
                                            key.to_string(),
                                            Some(data),
                                        )));
                                    }
                                }
                                None => {
                                    events.send(Event::PacketTx(Protocol::Store(
                                        key.to_string(),
                                        None,
                                    )));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }
}
