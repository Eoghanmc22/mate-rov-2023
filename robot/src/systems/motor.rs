use crate::peripheral::motor::Motor;
use crate::systems::RobotSystem;
use anyhow::Context;
use common::state::{RobotState, RobotStateUpdate};
use common::types::{MotorFrame, MotorId, Movement};
use crossbeam::channel;
use crossbeam::channel::Sender;
use rppal::gpio::{Gpio, OutputPin};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use tracing::{error, info, span, Level};

pub struct MotorSystem(Sender<Message>);

enum Message {
    MotorSpeed(MotorId, MotorFrame),
}

impl RobotSystem for MotorSystem {
    #[tracing::instrument]
    fn start(robot: Arc<RwLock<RobotState>>) -> anyhow::Result<Self> {
        info!("Starting motor system");
        let (tx, rx) = channel::bounded(30);
        let gpio = Gpio::new().context("Create gpio")?;

        thread::spawn(move || {
            span!(Level::INFO, "Motor thread");
            let mut motors: HashMap<MotorId, Motor<OutputPin>> = HashMap::new();

            for message in rx.into_iter() {
                match message {
                    Message::MotorSpeed(motor_id, frame) => {
                        let mut entry = motors.entry(motor_id);
                        let motor = match entry {
                            Entry::Occupied(ref mut occupied) => Some(occupied.get_mut()),
                            Entry::Vacant(vacant) => {
                                let ret = Motor::new(&gpio, motor_id.into());
                                match ret {
                                    Ok(motor) => Some(vacant.insert(motor)),
                                    Err(error) => {
                                        error!("Could not create motor: {motor_id:?} {error:?}");
                                        None
                                    }
                                }
                            }
                        };
                        if let Some(motor) = motor {
                            let ret = motor.set_speed(frame.0).context("Set speed");
                            if let Err(error) = ret {
                                error!("Couldn't set speed: {motor_id:?} {error:?}");
                            }
                        }
                    }
                }
            }
        });

        Ok(MotorSystem(tx))
    }

    fn on_update(&self, update: &RobotStateUpdate, robot: &RobotState) -> Vec<RobotStateUpdate> {
        match update {
            RobotStateUpdate::Armed(armed) => {
                todo!();
            }
            RobotStateUpdate::Motor(id, frame) => {
                self.0
                    .send(Message::MotorSpeed(*id, *frame))
                    .expect("Send message");
            }
            RobotStateUpdate::Movement(movement) => {
                return mix_movement(*movement, robot.motors().keys());
            }
            _ => {}
        }

        Vec::new()
    }
}

pub fn mix_movement<'a>(
    movement: Movement,
    motors: impl IntoIterator<Item = &'a MotorId>,
) -> Vec<RobotStateUpdate> {
    let mut messages = Vec::new();

    for motor in motors {
        let speed = match motor {
            MotorId::UpF => movement.z + movement.x_rot,
            MotorId::UpB => movement.z - movement.x_rot,
            MotorId::UpL => movement.z - movement.y_rot,
            MotorId::UpR => movement.z + movement.y_rot,
            MotorId::FrontL => movement.y + movement.x + movement.z_rot,
            MotorId::FrontR => movement.y - movement.x - movement.z_rot,
            MotorId::RearL => -movement.y + movement.x - movement.z_rot,
            MotorId::RearR => -movement.y - movement.x + movement.z_rot,
        };

        messages.push(RobotStateUpdate::Motor(*motor, MotorFrame(speed)));
    }

    messages
}
