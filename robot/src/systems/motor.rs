use crate::events::EventHandle;
use crate::peripheral_new::motor::Motor;
use crate::systems::System;
use crate::{event::Event, peripheral_new::pca9685::Pca9685};
use anyhow::anyhow;
use common::{
    error::LogErrorExt,
    store::{tokens, KeyImpl, Store},
    types::{Armed, MotorFrame, MotorId, Movement, Speed},
};
use crossbeam::channel;
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::thread::{self, Scope};
use std::time::Duration;
use std::time::Instant;
use tracing::{span, Level};

pub const MAX_UPDATE_AGE: Duration = Duration::from_millis(250);

/// Handles Motor speed updated and controls the motors
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

        {
            let mut events = events.clone();
            spawner.spawn(move || {
                span!(Level::INFO, "Motor thread");

                let pwm_controller = Pca9685::new(
                    Pca9685::I2C_BUS,
                    Pca9685::I2C_ADDRESS,
                    Duration::from_secs_f64(1.0 / 400.0),
                );
                let mut pwm_controller = match pwm_controller {
                    Ok(pwm_controller) => pwm_controller,
                    Err(err) => {
                        events.send(Event::Error(err.context("PCA9685")));
                        return;
                    }
                };
                pwm_controller.output_enable();

                let mut deadlines: HashMap<MotorId, Instant> = HashMap::default();

                for message in rx.into_iter() {
                    match message {
                        Message::MotorSpeed(motor_id, frame, deadline) => {
                            deadlines.insert(motor_id, deadline);

                            let motor = Motor::from(motor_id);
                            let pwm = motor.speed_to_pwm(frame.0);

                            let rst = pwm_controller.set_pwm(motor.channel(), pwm);
                            if let Err(error) = rst {
                                events.send(Event::Error(
                                    error.context(format!("Couldn't set speed: {motor_id:?}")),
                                ));
                            }
                        }
                        Message::CheckDeadlines => {
                            for (motor_id, deadline) in &deadlines {
                                if Instant::now() - *deadline > MAX_UPDATE_AGE {
                                    let motor = Motor::from(*motor_id);
                                    let pwm = motor.speed_to_pwm(Speed::ZERO);

                                    let rst = pwm_controller.set_pwm(motor.channel(), pwm);
                                    if let Err(error) = rst {
                                        events.send(Event::Error(
                                            error.context(format!(
                                                "Couldn't set speed: {motor_id:?}"
                                            )),
                                        ));
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
                    tokens::ARMED.0,
                    tokens::MOVEMENT_JOYSTICK.0,
                    tokens::MOVEMENT_OPENCV.0,
                    tokens::MOVEMENT_DEPTH.0,
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
                                        if ret.is_err() {
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
