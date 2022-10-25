use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use anyhow::Context;
use crossbeam::channel;
use crossbeam::channel::Sender;
use rppal::gpio::{Gpio, OutputPin};
use tracing::{error, Level, span};
use common::state::{RobotState, RobotStateUpdate};
use common::types::{MotorFrame, MotorId};
use crate::peripheral::motor::Motor;
use crate::systems::RobotSystem;

pub struct MotorSystem(Sender<Message>);

enum Message {
    MotorSpeed(MotorId, MotorFrame)
}

impl RobotSystem for MotorSystem {
    // TODO handle movement updates
    fn start(_robot: Arc<RwLock<RobotState>>, gpio: Gpio) -> anyhow::Result<Self> {
        let (tx, rx) = channel::bounded(10);
        
        thread::spawn(move || {
            span!(Level::INFO, "Motor thread");
            let mut motors: HashMap<MotorId, Motor<OutputPin>> = HashMap::new();

            for message in rx.into_iter() {
                match message {
                    Message::MotorSpeed(motor_id, frame) => {
                        let mut entry = motors.entry(motor_id);
                        let motor = match entry {
                            Entry::Occupied(ref mut occupied) => {
                                Some(occupied.get_mut())
                            }
                            Entry::Vacant(vacant) => {
                                let ret = Motor::new(&gpio, motor_id.into());
                                match ret {
                                    Ok(motor) => {
                                        Some(vacant.insert(motor))
                                    }
                                    Err(error) => {
                                        error!("Could not create motor: {:?} {:?}", motor_id, error);
                                        None
                                    }
                                }
                            }
                        };
                        if let Some(motor) = motor {
                            let ret = motor.set_speed(frame.0).context("Set speed");
                            if let Err(error) = ret {
                                error!("Couldn't set speed: {:?} {:?}", motor_id, error);
                            }
                        } else {
                            error!("Couldn't find motor: {:?}", motor_id);
                        }
                    }
                }
            }
        });

        Ok(MotorSystem(tx))
    }

    fn on_update(&mut self, update: RobotStateUpdate) {
        match update {
            RobotStateUpdate::Motor(id, frame) => {
                self.0.send(Message::MotorSpeed(id, frame)).expect("Send message");
            }
            _ => {}
        }
    }
}