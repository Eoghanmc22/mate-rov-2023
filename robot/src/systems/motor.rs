use crate::event::Event;
use crate::events::EventHandle;
use crate::peripheral::motor::Motor;
use crate::systems::System;
use anyhow::{anyhow, Context};
use common::{
    error::LogError,
    store::{tokens, KeyImpl, Store},
    types::{Armed, MotorFrame, MotorId, Movement, Speed},
};
use crossbeam::channel;
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use rppal::gpio::{Gpio, OutputPin};
use std::thread::{self, Scope};
use std::time::Duration;
use std::{collections::hash_map::Entry, time::Instant};
use tracing::{span, Level};

const MAX_UPDATE_AGE: Duration = Duration::from_millis(250);

pub struct MotorSystem;

enum Message {
    MotorSpeed(MotorId, MotorFrame, Instant),
    CheckDeadlines,
}

impl System for MotorSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        let (tx, rx) = channel::bounded(30);
        let gpio = Gpio::new().context("Create gpio")?;

        {
            let mut events = events.clone();
            spawner.spawn(move || {
                span!(Level::INFO, "Motor thread");
                let mut motors: HashMap<MotorId, Motor<OutputPin>> = HashMap::default();
                let mut deadlines: HashMap<MotorId, Instant> = HashMap::default();

                for message in rx.into_iter() {
                    match message {
                        Message::MotorSpeed(motor_id, frame, deadline) => {
                            deadlines.insert(motor_id, deadline);

                            let mut entry = motors.entry(motor_id);
                            let motor = match entry {
                                Entry::Occupied(ref mut occupied) => Some(occupied.get_mut()),
                                Entry::Vacant(vacant) => {
                                    let ret = Motor::new(&gpio, motor_id.into());
                                    match ret {
                                        Ok(motor) => Some(vacant.insert(motor)),
                                        Err(error) => {
                                            events.send(Event::Error(error.context(format!(
                                                "Could not create motor: {motor_id:?}"
                                            ))));
                                            None
                                        }
                                    }
                                }
                            };
                            if let Some(motor) = motor {
                                let ret = motor.set_speed(frame.0).context("Set speed");
                                if let Err(error) = ret {
                                    events.send(Event::Error(
                                        error.context(format!("Couldn't set speed: {motor_id:?}")),
                                    ));
                                }
                            }
                        }
                        Message::CheckDeadlines => {
                            for (motor_id, deadline) in &deadlines {
                                if Instant::now() - *deadline > MAX_UPDATE_AGE {
                                    if let Some(motor) = motors.get_mut(motor_id) {
                                        let ret = motor.set_speed(Speed::ZERO).context("Set speed");
                                        if let Err(error) = ret {
                                            events.send(Event::Error(error.context(format!(
                                                "Couldn't set speed: {motor_id:?}"
                                            ))));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        {
            let mut events = events.clone();
            let tx = tx.clone();
            spawner.spawn(move || {
                span!(Level::INFO, "Motor forward thread");

                let mut store = {
                    let mut events = events.clone();
                    Store::new(move |update| {
                        events.send(Event::Store(update));
                    })
                };

                let motor_ids = [
                    MotorId::FrontLeftBottom,
                    MotorId::FrontLeftTop,
                    MotorId::FrontRightBottom,
                    MotorId::FrontRightTop,
                    MotorId::BackLeftBottom,
                    MotorId::BaclLeftTop,
                    MotorId::BackRightBottom,
                    MotorId::RearRightTop,
                ];
                let motors = motor_ids
                    .into_iter()
                    .map(|it| (it, Default::default()))
                    .collect();

                store.insert(&tokens::MOTOR_SPEED, (motors, Instant::now()));

                let listening: HashSet<KeyImpl> = vec![
                    tokens::ARMED.0.clone(),
                    tokens::MOVEMENT_JOYSTICK.0.clone(),
                    tokens::MOVEMENT_OPENCV.0.clone(),
                    tokens::MOVEMENT_DEPTH.0.clone(),
                ]
                .into_iter()
                .collect();

                for event in listner.into_iter() {
                    match &*event {
                        Event::SyncStore => {
                            store.refresh();
                        }
                        Event::Store(update) => {
                            store.handle_update_shared(update);

                            // Need to recalculate motor speeds
                            if listening.contains(&update.0) {
                                let now = Instant::now();
                                let mut movement = Movement::default();

                                if let Some(data) = store.get(&tokens::ARMED) {
                                    let (armed, time_stamp) = *data;

                                    if matches!(armed, Armed::Armed)
                                        && now - time_stamp < MAX_UPDATE_AGE
                                    {
                                        if let Some(data) = store.get(&tokens::MOVEMENT_JOYSTICK) {
                                            let (joystick, time_stamp) = *data;
                                            if now - time_stamp < MAX_UPDATE_AGE {
                                                movement += joystick;
                                            }
                                        }
                                        if let Some(data) = store.get(&tokens::MOVEMENT_OPENCV) {
                                            let (opencv, time_stamp) = *data;
                                            if now - time_stamp < MAX_UPDATE_AGE {
                                                movement += opencv;
                                            }
                                        }
                                        if let Some(data) = store.get(&tokens::MOVEMENT_DEPTH) {
                                            let (depth, time_stamp) = *data;
                                            if now - time_stamp < MAX_UPDATE_AGE {
                                                movement += depth;
                                            }
                                        }
                                    } else {
                                        // Armed expired
                                    }
                                } else {
                                    // events.send(Event::Error(anyhow!("No armed token")));
                                }

                                store.insert(&tokens::MOVEMENT_CALCULATED, (movement, now));

                                if let Some(motors) = store.get(&tokens::MOTOR_SPEED) {
                                    let new_speeds = mix_movement(movement, motors.0.keys());
                                    let deadline = now + MAX_UPDATE_AGE;

                                    for (motor, speed) in &new_speeds {
                                        let ret =
                                            tx.send(Message::MotorSpeed(*motor, *speed, deadline));
                                        if let Err(_) = ret {
                                            events.send(Event::Error(anyhow!(
                                                "Couldn't update new speed"
                                            )));
                                        }
                                    }

                                    store.insert(&tokens::MOTOR_SPEED, (new_speeds, now));
                                } else {
                                    events.send(Event::Error(anyhow!("No motor speed token")));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            });
        }

        {
            let tx = tx.clone();
            spawner.spawn(move || {
                span!(Level::INFO, "Motor deadline check thread");

                loop {
                    tx.send(Message::CheckDeadlines)
                        .log_error("Could not send deadline check");

                    thread::sleep(Duration::from_millis(20));
                }
            });
        }

        Ok(())
    }
}

// TODO Fix motor math
pub fn mix_movement<'a>(
    mov: Movement,
    motors: impl IntoIterator<Item = &'a MotorId>,
) -> HashMap<MotorId, MotorFrame> {
    let mut speeds = HashMap::default();

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

        speeds.insert(*motor, MotorFrame(speed));
    }

    speeds
}
