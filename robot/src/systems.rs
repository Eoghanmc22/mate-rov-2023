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
    sync::{Arc, Condvar, Mutex, RwLock},
};
use tracing::info;

use crate::events::EventHandle;

pub struct SystemManager(
    Arc<RwLock<RobotState>>,
    Vec<fn(Arc<RwLock<RobotState>>, EventHandle) -> anyhow::Result<()>>,
    (Mutex<bool>, Condvar),
);

impl SystemManager {
    pub fn new(robot: RobotState) -> Self {
        Self(
            Arc::new(RwLock::new(robot)),
            Vec::new(),
            (Mutex::new(true), Condvar::new()),
        )
    }

    #[tracing::instrument(skip(self))]
    pub fn add_system<S: System + Send + Sync + 'static>(&mut self) -> anyhow::Result<()> {
        self.1.push(S::start);
        info!("Registered {}", any::type_name::<S>());

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn start(&self) {
        info!("---------- Starting systems ----------");
        let system_count = self.1.len();
        let mut event_handles = EventHandle::create(system_count);
        for (idx, system) in self.1.iter().enumerate() {
            info!("Loading system {}/{}", idx + 1, system_count);
            (system)(self.0.clone(), event_handles.pop().unwrap()).expect("Start system");
            info!("Loaded system {}/{}", idx + 1, system_count);
        }
        assert!(event_handles.is_empty());
        info!("-------------------------------------");

        let (lock, cvar) = &self.2;
        let mut running = lock.lock().expect("Lock");

        while *running {
            running = cvar.wait(running).expect("Lock");
        }
    }

    pub fn shutdown(&self) {
        let (lock, cvar) = &self.2;
        let mut running = lock.lock().expect("Lock");

        *running = false;
        cvar.notify_all();
    }
}

pub trait System {
    fn start(robot: Arc<RwLock<RobotState>>, events: EventHandle) -> anyhow::Result<()>;
}
