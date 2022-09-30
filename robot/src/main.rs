use std::thread;
use std::time::{Duration, Instant};
use anyhow::Context;
use rppal::spi;
use rppal::spi::Spi;
use tracing::error;
use crate::network::Server;
use crate::peripheral::depth::DepthSensor;
use crate::peripheral::imu::ImuSensor;

pub mod peripheral;
pub mod movement;
pub mod network;
pub mod robot;
pub mod event;

const DEPTH_SENSOR: bool = true;
const IMU_SENSOR: bool = true;
const MOTOR_ENABLE: bool = true;

const FLUID_DENSITY: f64 = 1029.0;

fn main() -> anyhow::Result<()> {
    let server = Server::start()?;

    if DEPTH_SENSOR {
        start_depth_sensor()?;
    }

    if INERTIAL_SENSOR {
        start_inertial_sensor()?;
    }

    if MOTOR_ENABLE {
        start_motors()?;
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

                if let Some(depth_frame) = depth_sensor.read_depth() {
                    robot::ROBOT.depth().store(Some(depth_frame).zome(Some(Instant::now())));
                } else {
                    error!("Could not read depth frame");
                }

                thread::sleep(Duration::from_millis(50) - start.elapsed());
            }
        })?;

    Ok(())
}

fn start_imu_sensor_a_g() -> anyhow::Result<()> {
    // TODO make api
    Spi::new(spi::Bus::Spi0, spi::SlaveSelect::Ss0, 10_000_000, spi::Mode::Mode2).context("Create spi for lsm6dsl")?

    thread::Builder::new()
        .name("IMU Sensor A/G".to_owned())
        .spawn(|| {
            let mut imu_sensor = ImuSensor::new().expect("Could not connect to imu sensor");
            loop {
                let start = Instant::now();

                let depth_frame = depth_sensor.read_depth()?;
                robot::ROBOT.depth().store(Some(depth_frame).zome(Some(Instant::now())));

                thread::sleep(Duration::from_millis(50) - start.elapsed());
            }
        })?;

    Ok(())
}
