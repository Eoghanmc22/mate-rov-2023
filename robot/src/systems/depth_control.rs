use std::{
    sync::Arc,
    thread::{self, Scope},
    time::{Duration, Instant},
};

use common::{
    error::LogErrorExt,
    store::{tokens, Store},
    types::{DepthControlMode, DepthCorrection, Movement, Percent, PidConfig, PidController},
};
use crossbeam::channel::bounded;
use glam::{Quat, Vec3};
use tracing::{span, warn, Level};

use crate::{event::Event, events::EventHandle, systems::stop, SystemId};

use super::System;

const PID_CONFIG: PidConfig = PidConfig {
    kp: 0.7,
    ki: 0.0,
    kd: 0.0,
    max_integral: 2.0,
};
const PERIOD: Duration = Duration::from_millis(20);

pub struct DepthControlSystem;

impl System for DepthControlSystem {
    const ID: SystemId = SystemId::DepthControl;

    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        let (tx, rx) = bounded(30);

        {
            let tx = tx.clone();
            spawner.spawn(move || {
                span!(Level::INFO, "Depth control watcher thread");

                for event in listner {
                    match &*event {
                        Event::Store(_) | Event::SyncStore | Event::ResetForignStore => {
                            tx.try_send(DepthControlEvent::Event(event))
                                .log_error("Send Event");
                        }
                        Event::Exit => {
                            tx.try_send(DepthControlEvent::Exit).log_error("Send Exit");
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
                span!(Level::INFO, "Depth control tick thread");

                let mut deadline = Instant::now() + PERIOD;

                while !stop::world_stopped() {
                    tx.try_send(DepthControlEvent::Tick).log_error("Send tick");

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
                span!(Level::INFO, "Depth control fusion thread");

                let mut store = {
                    let mut events = events.clone();
                    Store::new(move |update| {
                        events.send(Event::Store(update));
                    })
                };

                let mut depth_controller = PidController::new(PERIOD);

                for event in rx {
                    match event {
                        DepthControlEvent::Event(event) => match &*event {
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
                        DepthControlEvent::Tick => {
                            if let (Some(mode), Some(depth_observed), Some(orientation)) = (
                                store.get(&tokens::DEPTH_CONTROL_MODE),
                                store.get(&tokens::RAW_DEPTH),
                                store.get(&tokens::ORIENTATION),
                            ) {
                                if let DepthControlMode::Enabled(depth_target) = *mode {
                                    let depth_error = depth_target.0 - depth_observed.depth.0;

                                    let config = store
                                        .get(&tokens::DEPTH_CONTROL_PID_OVERRIDE)
                                        .map(|it| *it)
                                        .unwrap_or(PID_CONFIG);
                                    let depth_pid_result =
                                        depth_controller.update(depth_error, config);

                                    let max_correction = 1.0;
                                    let depth_corection = depth_pid_result
                                        .correction()
                                        .clamp(-max_correction, max_correction);

                                    store.insert(&tokens::DEPTH_CONTROL_RESULT, depth_pid_result);
                                    store.insert(
                                        &tokens::DEPTH_CONTROL_CORRECTION,
                                        DepthCorrection {
                                            depth: depth_pid_result.correction(),
                                        },
                                    );

                                    let orientation: Quat = orientation.0.into();
                                    let correction_vec = orientation.inverse()
                                        * Vec3::new(0.0, 0.0, -depth_corection as f32);

                                    store.insert(
                                        &tokens::MOVEMENT_DEPTH,
                                        Movement {
                                            x: Percent::new(high_pass(
                                                correction_vec.x as f64,
                                                0.05,
                                            )),
                                            y: Percent::new(high_pass(
                                                correction_vec.y as f64,
                                                0.05,
                                            )),
                                            z: Percent::new(high_pass(
                                                correction_vec.z as f64,
                                                0.05,
                                            )),
                                            ..Movement::default()
                                        },
                                    );
                                } else {
                                    depth_controller = PidController::new(PERIOD);
                                    store.remove(&tokens::MOVEMENT_DEPTH);
                                }
                            } else {
                                depth_controller = PidController::new(PERIOD);
                                store.remove(&tokens::MOVEMENT_DEPTH);
                            }
                        }
                        DepthControlEvent::Exit => {
                            return;
                        }
                    }
                }
            });
        }

        Ok(())
    }
}

enum DepthControlEvent {
    Event(Arc<Event>),
    Tick,
    Exit,
}

fn high_pass(value: f64, threshold: f64) -> f64 {
    if value.abs() > threshold {
        value
    } else {
        0.0
    }
}
