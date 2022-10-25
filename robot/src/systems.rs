pub mod networking;
pub mod motor;
// TODO indicators
// TODO inertial
// TODO mag
// TODO depth
// TODO logging

use std::sync::{Arc, Condvar, Mutex, RwLock};
use anyhow::{bail, Context};
use lazy_static::lazy_static;
use rppal::gpio::Gpio;
use common::state::{RobotState, RobotStateUpdate};

lazy_static! {
    static ref SYSTEMS: Arc<SystemManager> = Arc::new(SystemManager(Mutex::new(Vec::new()), (Mutex::new(false), Condvar::new())));
}

pub struct SystemManager(Mutex<Vec<Box<dyn RobotSystem + Send + Sync + 'static>>>, (Mutex<bool>, Condvar));

impl SystemManager {
    pub fn add_system<S: RobotSystem + Send + Sync + 'static>(robot: Arc<RwLock<RobotState>>) -> anyhow::Result<()> {
        let gpio = Gpio::new().context("Create gpio")?;
        let system = S::start(robot, gpio).context("Start system")?;

        match SYSTEMS.0.lock() {
            Ok(mut systems) => {
                systems.push(Box::new(system));
            }
            Err(error) => {
                bail!("Couldn't add system: {error:?}");
            }
        }

        Ok(())
    }

    pub fn handle_update(update: RobotStateUpdate) {
        let mut systems = SYSTEMS.0.lock().expect("Lock");
        for system in &mut *systems {
            system.on_update(update);
        }
    }

    pub fn shutdown() {
        let (lock, cvar) = &SYSTEMS.1;
        let mut running = lock.lock().expect("Lock");

        *running = false;
        cvar.notify_all();
    }

    pub fn block() {
       let (lock, cvar) = &SYSTEMS.1;
        let mut running = lock.lock().expect("Lock");

        while *running {
            running = cvar.wait(running).expect("Lock");
        }
    }
}

pub trait RobotSystem {
    fn start(robot: Arc<RwLock<RobotState>>, gpio: Gpio) -> anyhow::Result<Self> where Self: Sized;
    fn on_update(&mut self, update: RobotStateUpdate);
}