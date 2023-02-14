pub mod error;
pub mod hw_stat;
pub mod motor;
pub mod networking;
pub mod robot;
// TODO indicators
// TODO cameras
// TODO inertial
// TODO mag
// TODO depth
// TODO logging
// TODO perhaps just a single sensor system?

use std::{
    any,
    sync::RwLock,
    thread::{self, Scope},
};
use tracing::info;

use crate::events::EventHandle;

pub struct SystemManager(Vec<for<'a> fn(EventHandle, &'a Scope<'a, '_>) -> anyhow::Result<()>>);

impl SystemManager {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[tracing::instrument(skip(self))]
    pub fn add_system<S: System>(&mut self) -> anyhow::Result<()> {
        self.0.push(S::start);
        info!("Registered {}", any::type_name::<S>());

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn start(self) {
        let SystemManager(systems) = self;

        info!("---------- Starting systems ----------");

        thread::scope(|spawner| {
            // Setup event system
            let system_count = systems.len();
            let mut event_handles = EventHandle::create(system_count);

            // Spawn each system on its own thread
            for (idx, system) in systems.iter().enumerate() {
                info!("Loading system {}/{}", idx + 1, system_count);

                let handle = event_handles.pop().unwrap();

                spawner.spawn(|| {
                    (system)(handle, spawner).expect("Start system");
                });

                info!("Loaded system {}/{}", idx + 1, system_count);
            }

            assert!(event_handles.is_empty());

            info!("-------------------------------------");
        });

        info!("Shutting down!");
    }
}

pub trait System {
    fn start<'scope>(events: EventHandle, spawner: &'scope Scope<'scope, '_>)
        -> anyhow::Result<()>;
}
