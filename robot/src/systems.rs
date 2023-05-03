pub mod cameras;
pub mod depth;
pub mod depth_control;
pub mod error;
pub mod hw_stat;
pub mod indicators;
pub mod inertial;
pub mod leak;
pub mod leveling;
pub mod logging;
pub mod motor;
pub mod networking;
pub mod orientation;
pub mod robot;
pub mod status;
pub mod stop;

use std::{
    any,
    collections::HashSet,
    thread::{self, Scope},
};
use tracing::info;

use crate::{events::EventHandle, SystemId};

/// Manages all the systems running on the robot
#[derive(Default)]
pub struct SystemManager(
    HashSet<SystemId>,
    Vec<(
        for<'a> fn(EventHandle, &'a Scope<'a, '_>) -> anyhow::Result<()>,
        SystemId,
    )>,
);

impl SystemManager {
    /// Registers a system
    #[tracing::instrument(skip(self))]
    pub fn add_system<S: System>(&mut self) -> anyhow::Result<()> {
        assert!(!self.0.insert(S::ID));

        self.1.push((S::start, S::ID));
        info!("Registered {} as {:?}", any::type_name::<S>(), S::ID);

        Ok(())
    }

    /// Starts all the systems
    #[tracing::instrument(skip(self))]
    pub fn start(self) {
        let Self(ids, systems) = self;

        info!("---------- Starting systems ----------");

        thread::scope(|spawner| {
            // Setup event system
            let mut event_handles = EventHandle::create(ids.into_iter());

            // Spawn each system on its own thread
            for (system, id) in systems {
                let handle = event_handles.remove(&id).unwrap();

                info!("Loading system {id:?}");

                spawner.spawn(move || {
                    (system)(handle, spawner).expect("Start system");
                });

                info!("Loaded system {id:?}");
            }

            assert!(event_handles.is_empty());

            info!("-------------------------------------");
        });

        info!("Shutting down!");
    }
}

/// Trait that repersents a system
pub trait System {
    const ID: SystemId;

    fn start<'scope>(events: EventHandle, spawner: &'scope Scope<'scope, '_>)
        -> anyhow::Result<()>;
}
