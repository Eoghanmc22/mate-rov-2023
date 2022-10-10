//! Robot Code for the MATE Sea Owls Team
#![feature(slice_as_chunks)]
#![warn(
    meta_variable_misuse,
    missing_debug_implementations,
    //missing_docs,
    //unsafe_code,
    //unused_results,
    //unreachable_pub,
    //clippy::pedantic,
    //clippy::nursery,
    //clippy::unwrap_used,
    //clippy::expect_used
)]

use std::thread;
use std::time::{Duration, Instant};
use anyhow::Context;
use rppal::spi;
use rppal::spi::Spi;
use tracing::error;
use crate::network::Server;
use crate::peripheral::depth::DepthSensor;
use crate::peripheral::imu::{Inertial, Magnetometer};

pub mod peripheral;
pub mod movement;
pub mod network;
pub mod robot;
pub mod event;

const DEPTH_SENSOR: bool = true;
const INERTIAL_SENSOR: bool = true;
const MAGNETIC_SENSOR: bool = true;
const MOTOR_ENABLE: bool = true;

const FLUID_DENSITY: f64 = 1029.0;

fn main() {

}

/*fn main() -> anyhow::Result<()> {
    let server = Server::start()?;

    if DEPTH_SENSOR {
        start_depth_sensor()?;
    }

    if INERTIAL_SENSOR {
        start_inertial_sensor()?;
    }

    if MAGNETIC_SENSOR {
        start_magnetic_sensor()?;
    }

    if MOTOR_ENABLE {
        //start_motors()?;
    }

    Ok(())
}

fn start_depth_sensor() -> anyhow::Result<()> {
    let mut depth_sensor = DepthSensor::new(FLUID_DENSITY).expect("Could not connect to depth sensor");

    thread::Builder::new()
        .name("Depth Sensor".to_owned())
        .spawn(|| {
            // todo tracing span
            loop {
                let start = Instant::now();

                if let Ok(depth_frame) = depth_sensor.read_depth() {
                    robot::ROBOT.depth().store(Some(depth_frame).zip(Some(Instant::now())));
                } else {
                    error!("Could not read depth frame");
                }

                thread::sleep(Duration::from_millis(50) - start.elapsed());
            }
        })?;

    Ok(())
}

fn start_inertial_sensor() -> anyhow::Result<()> {
    // TODO make api
    let spi = Spi::new(spi::Bus::Spi0, spi::SlaveSelect::Ss0, 10_000_000, spi::Mode::Mode2).context("Create spi for lsm6dsl")?;

    thread::Builder::new()
        .name("IMU Sensor A/G".to_owned())
        .spawn(|| {
            let mut inertial_sensor = Inertial::new(spi).expect("Could not connect to imu sensor");
            loop {
                let start = Instant::now();

                let inertial_frame = inertial_sensor.read_sensor().unwrap()/*?*/;
                robot::ROBOT.inertial().store(Some(inertial_frame).zip(Some(Instant::now())));

                thread::sleep(Duration::from_millis(50) - start.elapsed());
            }
        })?;

    Ok(())
}

fn start_magnetic_sensor() -> anyhow::Result<()> {
    // TODO make api
    let spi = Spi::new(spi::Bus::Spi0, spi::SlaveSelect::Ss0, 10_000_000, spi::Mode::Mode2).context("Create spi for lsm6dsl")?;

    thread::Builder::new()
        .name("IMU Sensor M".to_owned())
        .spawn(|| {
            let mut magnetic_sensor = Magnetometer::new(spi).expect("Could not connect to imu sensor");
            loop {
                let start = Instant::now();

                let magnetic_frame = magnetic_sensor.read_sensor().unwrap()/*?*/;
                robot::ROBOT.mag().store(Some(magnetic_frame).zip(Some(Instant::now())));

                thread::sleep(Duration::from_millis(50) - start.elapsed());
            }
        })?;

    Ok(())
}*/
