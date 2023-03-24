// TODO mag
// TODO depth
// TODO fusion
// TODO ai
pub mod cameras;
pub mod depth;
pub mod error;
pub mod hw_stat;
pub mod indicators;
pub mod inertial;
pub mod leak;
pub mod logging;
pub mod motor;
pub mod networking;
pub mod robot;
pub mod status;
pub mod stop;

use std::{
    any,
    thread::{self, Scope},
};
use tracing::info;

use crate::events::EventHandle;

/// Manages all the systems running on the robot
pub struct SystemManager(Vec<for<'a> fn(EventHandle, &'a Scope<'a, '_>) -> anyhow::Result<()>>);

impl SystemManager {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Registers a system
    #[tracing::instrument(skip(self))]
    pub fn add_system<S: System>(&mut self) -> anyhow::Result<()> {
        self.0.push(S::start);
        info!("Registered {}", any::type_name::<S>());

        Ok(())
    }

    /// Starts all the systems
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

/// Trait that repersents a system
pub trait System {
    fn start<'scope>(events: EventHandle, spawner: &'scope Scope<'scope, '_>)
        -> anyhow::Result<()>;
}
