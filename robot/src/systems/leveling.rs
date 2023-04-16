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
use tracing::{info, span, warn, Level};

use crate::{event::Event, events::EventHandle, systems::stop};

use super::System;

const PID_CONFIG: PidConfig = PidConfig {
    k_p: 0.05,
    k_i: 0.0,
    k_d: 0.0,
    max_integral: 2.0,
    clamp_p: 0.3,
    clamp_i: 0.3,
    clamp_d: 0.3,
    clamp_total: 0.2,
};

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

                let interval = Duration::from_millis(20);

                let mut deadline = Instant::now() + interval;

                while !stop::world_stopped() {
                    tx.try_send(LevelingEvent::Tick).log_error("Send tick");

                    let remaining = deadline - Instant::now();
                    if !remaining.is_zero() {
                        thread::sleep(remaining);
                    } else {
                        warn!("Behind schedual");
                    }
                    deadline += interval;
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

                let mut pitch_controller = PidController::default();
                let mut roll_controller = PidController::default();

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
                                    let (pitch_target, roll_target) =
                                        (pitch_target.to_radians(), roll_target.to_radians());
                                    let (_, pitch_observed, roll_observed) =
                                        orientation.to_euler(EulerRot::ZXY);

                                    let (pitch_error, roll_error) = (
                                        pitch_target - pitch_observed,
                                        roll_target - roll_observed,
                                    );

                                    let config = store
                                        .get(&tokens::LEVELING_PID)
                                        .map(|it| *it)
                                        .unwrap_or(PID_CONFIG);
                                    let pitch_correction =
                                        pitch_controller.update(pitch_error, config);
                                    let roll_correction =
                                        roll_controller.update(roll_error, config);

                                    store.insert(
                                        &tokens::LEVELING_CORRECTION,
                                        LevelingCorrection {
                                            pitch: pitch_correction,
                                            roll: roll_correction,
                                        },
                                    );
                                    store.insert(
                                        &tokens::MOVEMENT_LEVELING,
                                        Movement {
                                            x_rot: Percent::new(pitch_correction as f64),
                                            y_rot: Percent::new(roll_correction as f64),
                                            ..Movement::default()
                                        },
                                    );
                                } else {
                                    pitch_controller = Default::default();
                                    roll_controller = Default::default();
                                    store.remove(&tokens::MOVEMENT_LEVELING);
                                }
                            } else {
                                pitch_controller = Default::default();
                                roll_controller = Default::default();
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