use std::thread::Scope;

use anyhow::anyhow;
use common::{protocol::Protocol, store::tokens};
use tracing::{span, Level};

use crate::{event::Event, events::EventHandle, SystemId};

use super::System;

/// Handles inbound and outbound updates to the global store
pub struct StoreSystem;

impl System for StoreSystem {
    const ID: SystemId = SystemId::Store;

    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        spawner.spawn(move || {
            span!(Level::INFO, "Robot update thread");

            let adapters = tokens::generate_adaptors();

            for event in listner {
                match &*event {
                    // Handle inbound stores
                    Event::PacketRx(Protocol::Store(key, data)) => {
                        let key = key.clone().into();
                        let adapter = adapters.get(&key);

                        if let Some(adapter) = adapter {
                            match data {
                                Some(data) => {
                                    let data = adapter.deserialize(data);

                                    if let Some(data) = data {
                                        events.send(Event::Store((key, Some(data.into()))));
                                    } else {
                                        events.send(Event::Error(anyhow!(
                                            "Could not deserialize for {key:?}"
                                        )));
                                    }
                                }
                                None => events.send(Event::Store((key, None))),
                            }
                        } else {
                            events.send(Event::Error(anyhow!("No adapter found for {key:?}")));
                        }
                    }
                    // Handle outbound stores
                    Event::Store((key, data)) => {
                        let adapter = adapters.get(key);

                        if let Some(adapter) = adapter {
                            match data {
                                Some(data) => {
                                    let data = adapter.serialize(&**data);

                                    if let Some(data) = data {
                                        events.send(Event::PacketTx(Protocol::Store(
                                            key.to_string(),
                                            Some(data),
                                        )));
                                    } else {
                                        events.send(Event::Error(anyhow!(
                                            "Could not serialize for {key:?}"
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
                        } else {
                            events.send(Event::Error(anyhow!("No adapter found for {key:?}")));
                        }
                    }
                    // Handle forign invalidation
                    Event::PeerDisconnected(_) => {
                        events.send(Event::ResetForignStore);
                        events.send(Event::SyncStore);
                    }
                    // Handle sync requests
                    Event::PacketRx(Protocol::RequestSync) => {
                        events.send(Event::SyncStore);
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
