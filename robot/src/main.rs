//! Robot Code for the MATE Sea Owls Team
#![feature(slice_as_chunks)]
#![warn(meta_variable_misuse, clippy::pedantic, clippy::nursery)]

pub mod event;
pub mod events;
pub mod peripheral;
mod systems;

use crate::systems::error::ErrorSystem;

use crate::systems::robot::StoreSystem;
use crate::systems::status::StatusSystem;
use crate::systems::stop::StopSystem;
use crate::systems::SystemManager;
use crate::systems::{hw_stat::HwStatSystem, networking::NetworkSystem};
use tracing::{info, Level};

#[cfg(rpi)]
use crate::systems::cameras::CameraSystem;
#[cfg(rpi)]
use crate::systems::depth::DepthSystem;
#[cfg(rpi)]
use crate::systems::indicators::IndicatorsSystem;
#[cfg(rpi)]
use crate::systems::inertial::InertialSystem;
#[cfg(rpi)]
use crate::systems::leak::LeakSystem;
#[cfg(rpi)]
use crate::systems::motor::MotorSystem;
#[cfg(rpi)]
use crate::systems::orientation::OrientationSystem;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    info!("Starting robot");

    let mut systems = SystemManager::new();

    info!("---------- Registering systems ----------");
    systems.add_system::<StopSystem>()?;
    // systems.add_system::<LogEventSystem>()?;
    systems.add_system::<ErrorSystem>()?;
    systems.add_system::<StoreSystem>()?;
    systems.add_system::<NetworkSystem>()?;
    systems.add_system::<HwStatSystem>()?;
    systems.add_system::<StatusSystem>()?;
    #[cfg(rpi)]
    systems.add_system::<MotorSystem>()?;
    #[cfg(rpi)]
    systems.add_system::<IndicatorsSystem>()?;
    #[cfg(rpi)]
    systems.add_system::<LeakSystem>()?;
    #[cfg(rpi)]
    systems.add_system::<InertialSystem>()?;
    #[cfg(rpi)]
    systems.add_system::<OrientationSystem>()?;
    #[cfg(rpi)]
    systems.add_system::<DepthSystem>()?;
    #[cfg(rpi)]
    systems.add_system::<CameraSystem>()?;
    info!("--------------------------------------");

    systems.start();

    info!("Robot stopped");

    Ok(())
}
