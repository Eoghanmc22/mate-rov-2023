use crate::event::Event;
use crate::events::EventHandle;
use crate::peripheral::motor::Motor;
use crate::systems::System;
use anyhow::Context;
use common::state::{RobotState, RobotStateUpdate};
use common::types::{MotorFrame, MotorId, Movement};
use crossbeam::channel;
use crossbeam::channel::Sender;
use rppal::gpio::{Gpio, OutputPin};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::RwLock;
use std::thread::Scope;
use tracing::{error, info, span, Level};

pub struct MotorSystem(Sender<Message>);

enum Message {
    MotorSpeed(MotorId, MotorFrame),
}

impl System for MotorSystem {
    #[tracing::instrument]
    fn start<'scope>(
        robot: &'scope RwLock<RobotState>,
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        info!("Starting motor system");
        let (tx, rx) = channel::bounded(30);
        let gpio = Gpio::new().context("Create gpio")?;
        let listner = events.take_listner().unwrap();

        spawner.spawn(move || {
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

        spawner.spawn(move || {
            span!(Level::INFO, "Motor forward thread");
            for event in listner.into_iter() {
                if let Event::StateUpdate(updates) = &*event {
                    for update in updates {
                        match update {
                            RobotStateUpdate::Armed(armed) => {
                                todo!();
                            }
                            RobotStateUpdate::Motor(id, frame) => {
                                tx.send(Message::MotorSpeed(*id, *frame))
                                    .expect("Send message");
                            }
                            RobotStateUpdate::Movement(movement) => {
                                let robot = robot.read().expect("Accquire read");
                                events.send(Event::StateUpdate(mix_movement(
                                    *movement,
                                    robot.motors().keys(),
                                )));
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

pub fn mix_movement<'a>(
    mov: Movement,
    motors: impl IntoIterator<Item = &'a MotorId>,
) -> Vec<RobotStateUpdate> {
    let mut messages = Vec::new();

    for motor in motors {
        #[rustfmt::skip]
        let speed = match motor {
            MotorId::FrontLeftBottom =>    mov.x + mov.y - mov.z - mov.x_rot - mov.y_rot + mov.z_rot,
            MotorId::FrontLeftTop =>       mov.x + mov.y + mov.z + mov.x_rot + mov.y_rot + mov.z_rot,
            MotorId::FrontRightBottom =>  -mov.x + mov.y - mov.z - mov.x_rot + mov.y_rot - mov.z_rot,
            MotorId::FrontRightTop =>     -mov.x + mov.y + mov.z + mov.x_rot - mov.y_rot - mov.z_rot,
            MotorId::BackLeftBottom =>     mov.x - mov.y - mov.z + mov.x_rot - mov.y_rot - mov.z_rot,
            MotorId::BaclLeftTop =>        mov.x - mov.y + mov.z - mov.x_rot + mov.y_rot - mov.z_rot,
            MotorId::BackRightBottom =>   -mov.x - mov.y - mov.z + mov.x_rot + mov.y_rot + mov.z_rot,
            MotorId::RearRightTop =>      -mov.x - mov.y + mov.z - mov.x_rot - mov.y_rot + mov.z_rot,
        };

        messages.push(RobotStateUpdate::Motor(*motor, MotorFrame(speed)));
    }

    messages
}
