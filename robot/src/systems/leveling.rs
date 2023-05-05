use std::{
    f32::consts::{PI, TAU},
    sync::Arc,
    thread::{self, Scope},
    time::{Duration, Instant},
};

use common::{
    error::LogErrorExt,
    store::{tokens, Store},
    types::{LevelingCorrection, LevelingMode, Movement, Percent, PidConfig, PidController},
};
use crossbeam::channel::bounded;
use glam::{Quat, Vec3};
use tracing::{span, warn, Level};

use crate::{event::Event, events::EventHandle, systems::stop, SystemId};

use super::System;

const PID_CONFIG: PidConfig = PidConfig {
    kp: 0.4,
    ki: 0.1,
    kd: 0.2,
    max_integral: 2.0,
};
const PID_PITCH_MULTIPLIER: f64 = 2.0;
const PID_ROLL_MULTIPLIER: f64 = 1.0;
const PERIOD: Duration = Duration::from_millis(20);

pub struct LevelingSystem;

impl System for LevelingSystem {
    const ID: SystemId = SystemId::Leveling;

    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        let (tx, rx) = bounded(30);

        {
            let tx = tx.clone();
            spawner.spawn(move || {
                span!(Level::INFO, "Leveling watcher thread");

                for event in listner {
                    match &*event {
                        Event::Store(_) | Event::SyncStore | Event::ResetForignStore => {
                            tx.try_send(LevelingEvent::Event(event))
                                .log_error("Send Event");
                        }
                        Event::Exit => {
                            tx.try_send(LevelingEvent::Exit).log_error("Send Exit");
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
                span!(Level::INFO, "Leveling tick thread");

                let mut deadline = Instant::now() + PERIOD;

                while !stop::world_stopped() {
                    tx.try_send(LevelingEvent::Tick).log_error("Send tick");

                    let remaining = deadline - Instant::now();
                    if !remaining.is_zero() {
                        thread::sleep(remaining);
                    } else {
                        warn!("Behind schedual");
                    }
                    deadline += PERIOD;
                }
            });
        }

        {
            let rx = rx;
            spawner.spawn(move || {
                span!(Level::INFO, "Leveling fusion thread");

                let mut store = {
                    let mut events = events.clone();
                    Store::new(move |update| {
                        events.send(Event::Store(update));
                    })
                };

                let mut pitch_controller = PidController::new(PERIOD);
                let mut roll_controller = PidController::new(PERIOD);

                for event in rx {
                    match event {
                        LevelingEvent::Event(event) => match &*event {
                            Event::SyncStore => {
                                store.refresh();
                            }
                            Event::ResetForignStore => {
                                store.reset_shared();
                            }
                            Event::Store(update) => {
                                store.handle_update_shared(update);
                            }
                            _ => unreachable!(),
                        },
                        LevelingEvent::Tick => {
                            if let Some((mode, orientation)) = Option::zip(
                                store.get(&tokens::LEVELING_MODE),
                                store.get(&tokens::ORIENTATION),
                            ) {
                                let orientation = Quat::from(orientation.0);

                                if let LevelingMode::Enabled(target_up) = *mode {
                                    let target_up: Vec3 = target_up.into();
                                    let observed_up = orientation * Vec3::Z;

                                    let error = Quat::from_rotation_arc(observed_up, target_up);
                                    let pitch_error =
                                        instant_twist(error, orientation * Vec3::X).to_degrees();
                                    let roll_error =
                                        instant_twist(error, orientation * Vec3::Y).to_degrees();

                                    let config = store
                                        .get(&tokens::LEVELING_PID_OVERRIDE)
                                        .map(|it| *it)
                                        .unwrap_or(PID_CONFIG);
                                    let pitch_pid_result =
                                        pitch_controller.update(pitch_error, config);
                                    let roll_pid_result =
                                        roll_controller.update(roll_error, config);

                                    let max_correction = 0.30;
                                    let pitch_corection = pitch_pid_result
                                        .corection()
                                        .clamp(-max_correction, max_correction);
                                    let roll_corection = roll_pid_result
                                        .corection()
                                        .clamp(-max_correction, max_correction);

                                    store.insert(&tokens::LEVELING_PITCH_RESULT, pitch_pid_result);
                                    store.insert(&tokens::LEVELING_ROLL_RESULT, roll_pid_result);
                                    store.insert(
                                        &tokens::LEVELING_CORRECTION,
                                        LevelingCorrection {
                                            pitch: pitch_pid_result.corection(),
                                            roll: roll_pid_result.corection(),
                                        },
                                    );
                                    store.insert(
                                        &tokens::MOVEMENT_LEVELING,
                                        Movement {
                                            x_rot: Percent::new(
                                                pitch_corection as f64 * PID_PITCH_MULTIPLIER,
                                            ),
                                            y_rot: Percent::new(
                                                roll_corection as f64 * PID_ROLL_MULTIPLIER,
                                            ),
                                            ..Movement::default()
                                        },
                                    );
                                } else {
                                    pitch_controller = PidController::new(PERIOD);
                                    roll_controller = PidController::new(PERIOD);
                                    store.remove(&tokens::MOVEMENT_LEVELING);
                                }
                            } else {
                                pitch_controller = PidController::new(PERIOD);
                                roll_controller = PidController::new(PERIOD);
                                store.remove(&tokens::MOVEMENT_LEVELING);
                            }
                        }
                        LevelingEvent::Exit => {
                            return;
                        }
                    }
                }
            });
        }

        Ok(())
    }
}

enum LevelingEvent {
    Event(Arc<Event>),
    Tick,
    Exit,
}

fn instant_twist(q: Quat, twist_axis: Vec3) -> f32 {
    let rotation_axis = Vec3::new(q.x, q.y, q.z);

    let sign = rotation_axis.dot(twist_axis).signum();
    let projected = rotation_axis.project_onto(twist_axis);
    let twist = Quat::from_xyzw(projected.x, projected.y, projected.z, q.w).normalize() * sign;

    let angle = twist.w.acos() * 2.0;
    normalize_angle(angle)
}

fn normalize_angle(angle: f32) -> f32 {
    let wrapped_angle = modf(angle, TAU);
    if wrapped_angle > PI {
        wrapped_angle - TAU
    } else {
        wrapped_angle
    }
}

fn modf(a: f32, b: f32) -> f32 {
    (a % b + b) % b
}
