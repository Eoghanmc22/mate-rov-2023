use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::Scope,
};

use anyhow::Context;

use crate::{event::Event, events::EventHandle};

use super::System;

static STOP_THE_WORLD: AtomicBool = AtomicBool::new(false);

pub struct StopSystem;

impl System for StopSystem {
    fn start<'scope>(
        mut events: EventHandle,
        _spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let _ = events.take_listner();

        ctrlc::set_handler(move || {
            STOP_THE_WORLD.store(true, Ordering::Relaxed);
            events.send(Event::Exit);
        })
        .context("Set ctrl-c")?;

        Ok(())
    }
}

pub fn world_stopped() -> bool {
    STOP_THE_WORLD.load(Ordering::Relaxed)
}
