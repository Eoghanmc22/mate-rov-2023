use std::{
    sync::Arc,
    thread::{self, Scope},
    time::{Duration, Instant},
};

use common::{
    error::LogErrorExt,
    store::{self, tokens, Store, Update},
    types::Orientation,
};
use crossbeam::channel::bounded;
use tracing::{span, warn, Level};

use crate::{event::Event, events::EventHandle, systems::stop};

use super::System;

pub struct LevelingSystem;

impl System for LevelingSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        let (tx, rx) = bounded(10);

        {
            let tx = tx.clone();
            spawner.spawn(move || {
                span!(Level::INFO, "Leveling watcher thread");

                for event in listner {
                    match &*event {
                        Event::Store(_) | Event::SyncStore | Event::ResetForignStore => {
                            tx.try_send(LevelingEvent::Event(event))
                                .log_error("Send Exit");
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
                            todo!()
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

#[derive(Default, Clone, Copy)]
struct PidController {
    last_error: Option<f32>,
    integral: f32,
}

#[derive(Clone, Copy)]
struct PidConfig {
    k_p: f32,
    k_i: f32,
    k_d: f32,

    max_integral: f32,

    clamp_p: f32,
    clamp_i: f32,
    clamp_d: f32,

    clamp_total: f32,
}

impl PidController {
    pub fn update(&mut self, error: f32, config: PidConfig) -> f32 {
        let p = error;

        self.integral = clamp(self.integral + error, config.max_integral);
        let i = self.integral;

        let d = if let Some(last_error) = self.last_error {
            error - last_error
        } else {
            0.0
        };
        self.last_error = Some(error);

        let p = clamp(p * config.k_p, config.clamp_p);
        let i = clamp(i * config.k_i, config.clamp_i);
        let d = clamp(d * config.k_d, config.clamp_d);

        clamp(p + i + d, config.clamp_total)
    }
}

impl Default for PidConfig {
    fn default() -> Self {
        Self {
            k_p: 0.05,
            k_i: 0.0,
            k_d: 0.0,
            max_integral: 2.0,
            clamp_p: 0.3,
            clamp_i: 0.3,
            clamp_d: 0.3,
            clamp_total: 0.2,
        }
    }
}

fn clamp(val: f32, range: f32) -> f32 {
    val.clamp(-range, range)
}
