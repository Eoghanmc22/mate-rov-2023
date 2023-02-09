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

use common::state::RobotState;
use std::{
    any,
    sync::RwLock,
    thread::{self, Scope},
};
use tracing::info;

use crate::events::EventHandle;

pub struct SystemManager(
    RobotState,
    Vec<for<'a> fn(&'a RwLock<RobotState>, EventHandle, &'a Scope<'a, '_>) -> anyhow::Result<()>>,
);

impl SystemManager {
    pub fn new(robot: RobotState) -> Self {
        Self(robot, Vec::new())
    }

    #[tracing::instrument(skip(self))]
    pub fn add_system<S: System>(&mut self) -> anyhow::Result<()> {
        self.1.push(S::start);
        info!("Registered {}", any::type_name::<S>());

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn start(self) {
        let SystemManager(robot, systems) = self;
        let robot = RwLock::new(robot);

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
                    (system)(&robot, handle, spawner).expect("Start system");
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
    fn start<'scope>(
        robot: &'scope RwLock<RobotState>,
        events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()>;
}
