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
use std::sync::{Arc, Condvar, Mutex, RwLock};

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

    pub fn add_system<S: System + Send + Sync + 'static>(&mut self) -> anyhow::Result<()> {
        self.1.push(S::start);

        Ok(())
    }

    pub fn start(&self) {
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
