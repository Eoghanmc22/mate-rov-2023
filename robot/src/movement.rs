use std::collections::hash_map::Entry::Occupied;
use std::collections::HashMap;
use std::fmt::Debug;
use serde::Serialize;
use serde::Deserialize;
use crate::peripheral::motor::{Motor, PwmDevice, Speed};

pub struct MovementController<PinType: Debug> {
    motors: HashMap<MotorId, Motor<PinType>>
}

impl<PinType: Debug> MovementController<PinType> {
    pub fn new(motors: impl Into<HashMap<MotorId, Motor<PinType>>>) -> Self {
        MovementController {
            motors: motors.into()
        }
    }
}

impl<PinType: PwmDevice> MovementController<PinType> {
    pub fn send_movement(&mut self, movement: Movement) -> anyhow::Result<()> {
        let motors = &mut self.motors;

        if let Occupied(mut entry) = motors.entry(MotorId::UpF) {
            let motor = entry.get_mut();
            motor.set_speed(Speed::new(movement.z + movement.x_rot))?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::UpB) {
            let motor = entry.get_mut();
            motor.set_speed(Speed::new(movement.z - movement.x_rot))?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::UpR) {
            let motor = entry.get_mut();
            motor.set_speed(Speed::new(movement.z + movement.y_rot))?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::UpL) {
            let motor = entry.get_mut();
            motor.set_speed(Speed::new(movement.z - movement.y_rot))?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::FrontL) {
            let motor = entry.get_mut();
            motor.set_speed(Speed::new(movement.y + movement.x + movement.z_rot))?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::FrontR) {
            let motor = entry.get_mut();
            motor.set_speed(Speed::new(movement.y - movement.x - movement.z_rot))?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::RearL) {
            let motor = entry.get_mut();
            motor.set_speed(Speed::new(-movement.y + movement.x - movement.z_rot))?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::RearR) {
            let motor = entry.get_mut();
            motor.set_speed(Speed::new(-movement.y - movement.x + movement.z_rot))?;
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MotorId {
    UpF,
    UpB,
    UpL,
    UpR,
    FrontL,
    FrontR,
    RearL,
    RearR,
}

/// +X: Right, +Y: Forwards, +Z: Up
/// +XR: Pitch Up, +YR: Roll Counterclockwise, +ZR: Yaw Clockwise (top view)
pub struct Movement {
    pub x: f64,
    pub y: f64,
    pub z: f64,

    pub x_rot: f64,
    pub y_rot: f64,
    pub z_rot: f64,
}
