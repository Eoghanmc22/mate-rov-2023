use crate::events::EventHandle;
use crate::peripheral::motor::Motor;
use crate::systems::{stop, System};
use crate::SystemId;
use crate::{event::Event, peripheral::pca9685::Pca9685};
use anyhow::{anyhow, Context};
use common::store::UpdateCallback;
use common::{
    error::LogErrorExt,
    store::{tokens, KeyImpl, Store},
    types::{Armed, MotorFrame, MotorId, Movement},
};
use crossbeam::channel;
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use serde::Deserialize;
use std::sync::Arc;
use std::thread::{self, Scope};
use std::time::Duration;
use std::time::Instant;
use tracing::{span, Level};

pub const MAX_UPDATE_AGE: Duration = Duration::from_millis(250);

/// Handles Motor speed updated and controls the motors
pub struct MotorSystem;

enum Message {
    Event(Arc<Event>),
    Tick,
}

impl System for MotorSystem {
    const ID: SystemId = SystemId::Motor;

    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        let (tx, rx) = channel::bounded(32);

        let motor_data = read_motor_data().context("Load motor data")?;

        {
            let mut events = events.clone();
            spawner.spawn(move || {
                span!(Level::INFO, "Motor thread");

                let mut store = {
                    let mut events = events.clone();
                    Store::new(move |update| {
                        events.send(Event::Store(update));
                    })
                };

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

                const STOP_PWMS: [Duration; 16] = [Duration::from_micros(1500); 16];
                let rst = pwm_controller
                    .set_pwms(STOP_PWMS)
                    .context("Set initial pwms");
                if let Err(error) = rst {
                    events.send(Event::Error(
                        error.context("Couldnt set initial pwms".to_string()),
                    ));
                    return;
                }

                pwm_controller.output_enable();

                for message in rx {
                    if stop::world_stopped() {
                        // Pca9685 stops on drop
                        return;
                    }

                    match message {
                        Message::Tick => {
                            // Recalculate motor speeds
                            let calculated_speeds = if let Some(armed) =
                                store.get_alive(&tokens::ARMED, MAX_UPDATE_AGE)
                            {
                                if matches!(*armed, Armed::Armed) {
                                    if let Some(speed_overrides) =
                                        store.get(&tokens::MOVEMENT_OVERRIDE)
                                    {
                                        let mut new_speeds = HashMap::default();

                                        // TODO: Use an iterator?
                                        for (motor, speed) in speed_overrides.iter() {
                                            new_speeds.insert(*motor, MotorFrame::Percent(*speed));
                                        }

                                        new_speeds
                                    } else {
                                        let movement = sum_movements(&store);
                                        store.insert(&tokens::MOVEMENT_CALCULATED, movement);

                                        mix_movement(movement, &motor_data)
                                    }
                                } else {
                                    // Disarmed
                                    Default::default()
                                }
                            } else {
                                // events.send(Event::Error(anyhow!("No armed token")));
                                Default::default()
                            };
                            store.insert(&tokens::MOTOR_SPEED, calculated_speeds.clone());

                            // Speeds to PWMs
                            let mut pwms = STOP_PWMS;
                            for (motor_id, frame) in &calculated_speeds {
                                let motor = Motor::from(*motor_id);

                                let pwm = match frame {
                                    MotorFrame::Percent(pct) => motor.value_to_pwm(*pct),
                                    MotorFrame::Raw(raw) => *raw,
                                };

                                pwms[motor.channel as usize] = pwm;
                            }

                            // Write motor speeds
                            let rst = pwm_controller.set_pwms(pwms);
                            if let Err(error) = rst {
                                events.send(Event::Error(
                                    error.context("Couldn't set speeds".to_string()),
                                ));
                            }
                        }
                        Message::Event(event) => match &*event {
                            Event::SyncStore => {
                                store.refresh();
                            }
                            Event::ResetForignStore => {
                                store.reset_shared();
                            }
                            Event::Store(update) => {
                                store.handle_update_shared(update);
                            }
                            Event::Exit => {
                                return;
                            }
                            _ => {}
                        },
                    }
                }
            });
        }

        {
            let mut events = events.clone();
            let tx = tx.clone();
            spawner.spawn(move || {
                span!(Level::INFO, "Motor forward thread");

                let listening: HashSet<KeyImpl> = vec![
                    tokens::ARMED.0,
                    tokens::MOVEMENT_JOYSTICK.0,
                    tokens::MOVEMENT_OPENCV.0,
                    tokens::MOVEMENT_DEPTH.0,
                    tokens::MOVEMENT_LEVELING.0,
                    tokens::MOVEMENT_OVERRIDE.0,
                ]
                .into_iter()
                .collect();

                for event in listner {
                    match &*event {
                        Event::Store(store) => {
                            if listening.contains(&store.0) {
                                tx.try_send(Message::Event(event))
                                    .log_error("Forward event to motor thread");
                            }
                        }
                        Event::SyncStore | Event::ResetForignStore => {
                            tx.try_send(Message::Event(event))
                                .log_error("Forward event to motor thread");
                        }
                        Event::Exit => {
                            tx.try_send(Message::Event(event))
                                .log_error("Forward event to motor thread");
                            return;
                        }
                        _ => {}
                    }
                }
            });
        }

        {
            let tx = tx;
            spawner.spawn(move || {
                span!(Level::INFO, "Motor deadline check thread");

                let interval = Duration::from_secs_f64(1.0 / 100.0);
                let mut deadline = Instant::now();

                while !stop::world_stopped() {
                    deadline += interval;

                    tx.send(Message::Tick)
                        .log_error("Could not send deadline check");

                    let remaining = deadline - Instant::now();
                    thread::sleep(remaining);
                }
            });
        }

        Ok(())
    }
}

pub fn sum_movements<C: UpdateCallback>(store: &Store<C>) -> Movement {
    let mut movement = Movement::default();

    if let Some(joystick) = store.get_alive(&tokens::MOVEMENT_JOYSTICK, MAX_UPDATE_AGE) {
        movement += *joystick;
    }
    if let Some(opencv) = store.get_alive(&tokens::MOVEMENT_OPENCV, MAX_UPDATE_AGE) {
        movement += *opencv;
    }
    if let Some(leveling) = store.get_alive(&tokens::MOVEMENT_LEVELING, MAX_UPDATE_AGE) {
        movement += *leveling;
    }
    if let Some(depth) = store.get_alive(&tokens::MOVEMENT_DEPTH, MAX_UPDATE_AGE) {
        movement += *depth;
    }

    movement
}

// TODO Fix motor math
pub fn mix_movement<'a>(mov: Movement, motor_data: &MotorData) -> HashMap<MotorId, MotorFrame> {
    const MAX_AMPERAGE: f64 = 20.0;

    let drive_ids = [
        MotorId::FrontLeftBottom,
        MotorId::FrontLeftTop,
        MotorId::FrontRightBottom,
        MotorId::FrontRightTop,
        MotorId::BackLeftBottom,
        MotorId::BackLeftTop,
        MotorId::BackRightBottom,
        MotorId::BackRightTop,
    ];
    let servo_ids = [
        MotorId::Camera1,
        MotorId::Camera2,
        MotorId::Camera3,
        MotorId::Camera4,
        MotorId::Aux1,
        MotorId::Aux2,
        MotorId::Aux3,
        MotorId::Aux4,
    ];

    let Movement {
        x,
        y,
        z,
        x_rot,
        y_rot,
        z_rot,
        cam_1,
        cam_2,
        cam_3,
        cam_4,
        aux_1,
        aux_2,
        aux_3,
        aux_4,
    } = mov;

    let (x, y, z) = (x.get(), y.get(), z.get());
    let (x_rot, y_rot, z_rot) = (x_rot.get(), y_rot.get(), z_rot.get());

    let mut raw_mix = HashMap::default();

    for motor_id in drive_ids {
        let motor = Motor::from(motor_id);

        #[rustfmt::skip]
        let speed = match motor_id {
            MotorId::FrontLeftBottom =>   -x - y + z + x_rot + y_rot - z_rot,
            MotorId::FrontLeftTop =>      -x - y - z - x_rot - y_rot - z_rot,
            MotorId::FrontRightBottom =>   x - y + z + x_rot - y_rot + z_rot,
            MotorId::FrontRightTop =>      x - y - z - x_rot + y_rot + z_rot,
            MotorId::BackLeftBottom =>    -x + y + z - x_rot + y_rot + z_rot,
            MotorId::BackLeftTop =>       -x + y - z + x_rot - y_rot + z_rot,
            MotorId::BackRightBottom =>    x + y + z - x_rot - y_rot - z_rot,
            MotorId::BackRightTop =>       x + y - z + x_rot + y_rot - z_rot,

            _ => unreachable!()
        };

        let skew = if speed >= 0.0 { 1.0 } else { 1.25 };
        let direction = motor.max_value.get().signum();

        raw_mix.insert(motor_id, speed * skew * direction);
    }

    let max_raw = raw_mix.len() as f64;
    let total_raw: f64 = raw_mix.values().map(|it| it.abs()).sum();
    let scale_raw = if total_raw > max_raw {
        max_raw / total_raw
    } else {
        // Handle cases where we dont want to go max speed
        1.0
    };

    let motor_amperage = MAX_AMPERAGE / max_raw;
    let mut speeds: HashMap<MotorId, MotorFrame> = raw_mix
        .into_iter()
        .map(|(motor, value)| (motor, value * scale_raw * motor_amperage))
        .map(|(motor, current)| (motor, motor_data.pwm_for_current(current)))
        .map(|(motor, pwm)| (motor, MotorFrame::Raw(pwm)))
        .collect();

    for motor in servo_ids {
        #[rustfmt::skip]
        let speed = match motor {
            MotorId::Camera1 => cam_1,
            MotorId::Camera2 => cam_2,
            MotorId::Camera3 => cam_3,
            MotorId::Camera4 => cam_4,

            MotorId::Aux1 => aux_1,
            MotorId::Aux2 => aux_2,
            MotorId::Aux3 => aux_3,
            MotorId::Aux4 => aux_4,

            _ => unreachable!()
        };

        speeds.insert(motor, MotorFrame::Percent(speed));
    }

    speeds
}

pub struct MotorData {
    forward: Vec<MotorRecord>,
    backward: Vec<MotorRecord>,
}

impl MotorData {
    pub fn sort(&mut self) {
        self.forward
            .sort_by(|a, b| f64::total_cmp(&a.current, &b.current));
        self.backward
            .sort_by(|a, b| f64::total_cmp(&a.current, &b.current));
    }

    pub fn pwm_for_current(&self, signed_current: f64) -> Duration {
        let current = signed_current.abs();

        let data_set = if signed_current >= 0.0 {
            &self.forward
        } else {
            &self.backward
        };
        assert!(!data_set.is_empty());

        let idx = data_set.partition_point(|x| x.current < current);
        let pwm = if idx > 0 && idx < data_set.len() {
            let a = &data_set[idx - 1];
            let b = &data_set[idx];

            let alpha = (current - a.current) / (b.current - a.current);

            a.pwm * (1.0 - alpha) + (b.pwm * alpha)
        } else {
            data_set[0].pwm
        };

        Duration::from_micros(pwm as u64)
    }
}

#[derive(Deserialize, Debug)]
pub struct MotorRecord {
    pwm: f64,
    rpm: f64,
    current: f64,
    voltage: f64,
    power: f64,
    force: f64,
    efficiency: f64,
}

pub fn read_motor_data() -> anyhow::Result<MotorData> {
    let forward = csv::Reader::from_path("forward_motor_data.csv").context("Read forward data")?;
    let reverse = csv::Reader::from_path("reverse_motor_data.csv").context("Read reverse data")?;

    let mut forward_data = Vec::default();
    for result in forward.into_deserialize() {
        let record: MotorRecord = result.context("Parse motor record")?;
        forward_data.push(record);
    }

    let mut reverse_data = Vec::default();
    for result in reverse.into_deserialize() {
        let record: MotorRecord = result.context("Parse motor record")?;
        reverse_data.push(record);
    }

    let mut motor_data = MotorData {
        forward: forward_data,
        backward: reverse_data,
    };
    motor_data.sort();

    Ok(motor_data)
}
