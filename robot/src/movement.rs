use std::collections::hash_map::Entry::Occupied;
use std::collections::HashMap;
use std::fmt::Debug;
use common::types::{MotorId, Movement, Speed};
use crate::peripheral::motor::{Motor, PwmDevice};

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
            motor.set_speed(movement.z + movement.x_rot)?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::UpB) {
            let motor = entry.get_mut();
            motor.set_speed(movement.z - movement.x_rot)?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::UpR) {
            let motor = entry.get_mut();
            motor.set_speed(movement.z + movement.y_rot)?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::UpL) {
            let motor = entry.get_mut();
            motor.set_speed(movement.z - movement.y_rot)?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::FrontL) {
            let motor = entry.get_mut();
            motor.set_speed(movement.y + movement.x + movement.z_rot)?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::FrontR) {
            let motor = entry.get_mut();
            motor.set_speed(movement.y - movement.x - movement.z_rot)?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::RearL) {
            let motor = entry.get_mut();
            motor.set_speed(-movement.y + movement.x - movement.z_rot)?;
        }

        if let Occupied(mut entry) = motors.entry(MotorId::RearR) {
            let motor = entry.get_mut();
            motor.set_speed(-movement.y - movement.x + movement.z_rot)?;
        }

        Ok(())
    }
}
