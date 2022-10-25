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

use std::sync::{Arc, RwLock};
use common::state::RobotState;
use common::types::MotorId;
use crate::systems::motor::MotorSystem;
use crate::systems::networking::NetworkSystem;
use crate::systems::SystemManager;

pub mod movement;
pub mod peripheral;
mod systems;

fn main() -> anyhow::Result<()> {
    let robot = RobotState::new(
        &[
            MotorId::FrontL,
            MotorId::FrontR,
            MotorId::RearL,
            MotorId::RearR,
            MotorId::UpR,
            MotorId::UpL
        ],
        SystemManager::handle_update
    );
    let robot = Arc::new(RwLock::new(robot));

    SystemManager::add_system::<NetworkSystem>(robot.clone())?;
    SystemManager::add_system::<MotorSystem>(robot.clone())?;

    SystemManager::block();

    Ok(())
}
