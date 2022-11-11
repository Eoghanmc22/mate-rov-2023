//! Robot Code for the MATE Sea Owls Team
#![feature(slice_as_chunks)]
#![warn(
    meta_variable_misuse,
    //missing_debug_implementations,
    //missing_docs,
    //unsafe_code,
    //unused_results,
    //unreachable_pub,
    //clippy::pedantic,
    //clippy::nursery,
    //clippy::unwrap_used,
    //clippy::expect_used
)]

pub mod peripheral;
mod systems;

use crate::systems::SystemManager;
use crate::systems::{hw_stat::HwStatSystem, networking::NetworkSystem};
use common::state::RobotState;
use common::types::MotorId;
use std::sync::{Arc, RwLock};
use tracing::{info, Level};

#[cfg(rpi)]
use crate::systems::motor::MotorSystem;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    info!("Starting robot");

    let robot = RobotState::new(&[
        MotorId::FrontL,
        MotorId::FrontR,
        MotorId::RearL,
        MotorId::RearR,
        MotorId::UpR,
        MotorId::UpL,
    ]);
    let robot = Arc::new(RwLock::new(robot));

    let mut systems = SystemManager::new(robot.clone());

    info!("---------- Starting systems ----------");
    systems.add_system::<NetworkSystem>()?;
    systems.add_system::<HwStatSystem>()?;
    #[cfg(rpi)]
    systems.add_system::<MotorSystem>()?;
    info!("--------------------------------------");

    systems.start();
    info!("Robot stopped");

    Ok(())
}
