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

pub mod event;
pub mod events;
pub mod peripheral;
mod systems;

use crate::systems::error::ErrorSystem;
use crate::systems::robot::StoreSystem;
use crate::systems::SystemManager;
use crate::systems::{hw_stat::HwStatSystem, networking::NetworkSystem};
use tracing::{info, Level};

#[cfg(rpi)]
use crate::systems::motor::MotorSystem;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    info!("Starting robot");

    let mut systems = SystemManager::new();

    info!("---------- Registering systems ----------");
    systems.add_system::<ErrorSystem>()?;
    systems.add_system::<StoreSystem>()?;
    systems.add_system::<NetworkSystem>()?;
    systems.add_system::<HwStatSystem>()?;
    #[cfg(rpi)]
    systems.add_system::<MotorSystem>()?;
    info!("--------------------------------------");

    systems.start();
    info!("Robot stopped");

    Ok(())
}
