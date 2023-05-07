//! Robot Code for the MATE Sea Owls Team
#![feature(split_array)]
#![warn(meta_variable_misuse)]

pub mod event;
pub mod events;
pub mod peripheral;
mod systems;

use crate::systems::error::ErrorSystem;

use crate::systems::SystemManager;
#[cfg(rpi)]
use crate::systems::{
    cameras::CameraSystem, depth::DepthSystem, depth_control::DepthControlSystem,
    indicators::IndicatorsSystem, inertial::InertialSystem, leak::LeakSystem,
    leveling::LevelingSystem, motor::MotorSystem, orientation::OrientationSystem,
};
use crate::systems::{
    hw_stat::HwStatSystem, networking::NetworkSystem, robot::StoreSystem, status::StatusSystem,
    stop::StopSystem,
};
use tracing::{info, Level};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    info!("Starting robot");

    let mut systems = SystemManager::default();

    info!("---------- Registering systems ----------");
    {
        systems.add_system::<StopSystem>()?;
        // systems.add_system::<LogEventSystem>()?;
        systems.add_system::<ErrorSystem>()?;
        systems.add_system::<StoreSystem>()?;
        systems.add_system::<NetworkSystem>()?;
        systems.add_system::<HwStatSystem>()?;
        systems.add_system::<StatusSystem>()?;
    }
    #[cfg(rpi)]
    {
        systems.add_system::<MotorSystem>()?;
        systems.add_system::<IndicatorsSystem>()?;
        systems.add_system::<LeakSystem>()?;
        systems.add_system::<InertialSystem>()?;
        systems.add_system::<OrientationSystem>()?;
        systems.add_system::<DepthControlSystem>()?;
        systems.add_system::<LevelingSystem>()?;
        systems.add_system::<DepthSystem>()?;
        systems.add_system::<CameraSystem>()?;
    }
    info!("--------------------------------------");

    systems.start();

    info!("Robot stopped");

    Ok(())
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum SystemId {
    Stop,
    LogEvents,
    Error,
    Store,
    Network,
    HwStatus,
    RobotStatus,
    Motor,
    Indicators,
    Leak,
    Inertial,
    Orientation,
    DepthControl,
    Leveling,
    Depth,
    Camera,
}
