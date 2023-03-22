use std::{thread, time::Duration};

use anyhow::Context;
use common::store::{self, tokens};
use rppal::gpio::{Gpio, Level, Trigger};

use crate::event::Event;

use super::System;

pub struct LeakSystem;

impl System for LeakSystem {
    fn start<'scope>(
        mut events: crate::events::EventHandle,
        _spawner: &'scope std::thread::Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let _ = events.take_listner().unwrap();

        let gpio = Gpio::new().context("Open gpio")?;
        let mut leak_pin = gpio.get(27).context("Open leak pin")?.into_input_pulldown();

        let update_initial = store::create_update(&tokens::LEAK, false);
        events.send(Event::Store(update_initial));

        leak_pin
            .set_async_interrupt(Trigger::Both, move |level| match level {
                Level::High => {
                    let update = store::create_update(&tokens::LEAK, true);
                    events.send(Event::Store(update));
                }
                Level::Low => {
                    let update = store::create_update(&tokens::LEAK, false);
                    events.send(Event::Store(update));
                }
            })
            .context("Set async leak interrupt")?;

        // Dont drop leak pin
        loop {
            thread::sleep(Duration::MAX);
        }

        // Ok(())
    }
}
