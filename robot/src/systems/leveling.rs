use std::{
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
use glam::{EulerRot, Quat};
use tracing::{span, warn, Level};

use crate::{event::Event, events::EventHandle, systems::stop};

use super::System;

const PID_CONFIG: PidConfig = PidConfig {
    k_p: 0.05,
    k_i: 0.0,
    k_d: 0.0,
    max_integral: 2.0,
};
const PERIOD: Duration = Duration::from_millis(20);

pub struct LevelingSystem;

impl System for LevelingSystem {
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

                                if let LevelingMode::Enabled(pitch_target, roll_target) = *mode {
                                    let (pitch_target, roll_target) = (
                                        pitch_target.0.to_radians() as f32,
                                        roll_target.0.to_radians() as f32,
                                    );
                                    let (_, pitch_observed, roll_observed) =
                                        orientation.to_euler(EulerRot::ZXY);

                                    let (pitch_error, roll_error) = (
                                        pitch_target - pitch_observed,
                                        roll_target - roll_observed,
                                    );

                                    let config = store
                                        .get(&tokens::LEVELING_PID_OVERRIDE)
                                        .map(|it| *it)
                                        .unwrap_or(PID_CONFIG);
                                    let pitch_pid_result =
                                        pitch_controller.update(pitch_error, config);
                                    let roll_pid_result =
                                        roll_controller.update(roll_error, config);

                                    let max_correction = 0.15;
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
                                            x_rot: Percent::new(pitch_corection as f64),
                                            y_rot: Percent::new(roll_corection as f64),
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
