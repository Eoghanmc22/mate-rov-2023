pub mod networking;
pub mod motor;
// TODO indicators
// TODO cameras
// TODO inertial
// TODO mag
// TODO depth
// TODO logging
// TODO perhaps just a single sensor system?

use std::sync::{Arc, Condvar, Mutex, RwLock};
use anyhow::Context;
use common::state::{RobotState, RobotStateUpdate};

pub struct SystemManager(Arc<RwLock<RobotState>>, Vec<Box<dyn RobotSystem + Send + Sync + 'static>>, (Mutex<bool>, Condvar));

impl SystemManager {
    pub fn new(robot: Arc<RwLock<RobotState>>) -> Self {
        Self(robot, Vec::new(), (Mutex::new(false), Condvar::new()))
    }

    pub fn add_system<S: RobotSystem + Send + Sync + 'static>(&mut self) -> anyhow::Result<()> {
        let system = S::start(self.0.clone()).context("Start system")?;

        self.1.push(Box::new(system));

        Ok(())
    }

    pub fn start(self) {
        let mut robot = self.0.write().expect("Lock");
        robot.set_callback(move |update, robot| {
            for system in &self.1 {
                system.on_update(update, robot);
            }
        });

        // TODO Fire events for updates made during setup?

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

pub trait RobotSystem {
    fn start(robot: Arc<RwLock<RobotState>>) -> anyhow::Result<Self> where Self: Sized;
    fn on_update(&self, update: &RobotStateUpdate, robot: &mut RobotState);
}