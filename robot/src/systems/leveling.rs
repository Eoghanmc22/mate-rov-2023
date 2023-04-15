use std::{
    thread::{self, Scope},
    time::{Duration, Instant},
};

use common::{
    error::LogErrorExt,
    store::{self, tokens},
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
                        Event::Store(update) => {
                            if let Some(orientation) =
                                store::handle_update(&tokens::ORIENTATION, update)
                            {
                                tx.try_send(LevelingEvent::Orientation(*orientation))
                                    .log_error("Repeat orientation update");
                            }
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

                let mut orientation = Orientation::default();

                for event in rx {
                    match event {
                        LevelingEvent::Orientation(frame) => orientation = frame,
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
    Orientation(Orientation),
    Tick,
    Exit,
}
