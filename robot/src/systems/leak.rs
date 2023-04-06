use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Context;
use common::store::{self, tokens};
use rppal::gpio::{Gpio, Level, Trigger};

use crate::event::Event;

use super::System;

pub struct LeakSystem;

impl System for LeakSystem {
    fn start<'scope>(
        mut events: crate::events::EventHandle,
        spawner: &'scope std::thread::Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listener = events.take_listner().unwrap();

        let gpio = Gpio::new().context("Open gpio")?;
        let mut leak_pin = gpio.get(27).context("Open leak pin")?.into_input_pulldown();

        let update_initial = store::create_update(&tokens::LEAK, false);
        events.send(Event::Store(update_initial));

        let leak_detected = Arc::new(AtomicBool::new(leak_pin.is_high()));

        // Listen to pin interrupts
        {
            let mut events = events.clone();
            let leak_detected = leak_detected.clone();

            leak_pin
                .set_async_interrupt(Trigger::Both, move |level| {
                    let level = match level {
                        Level::High => true,
                        Level::Low => false,
                    };

                    let update = store::create_update(&tokens::LEAK, level);
                    events.send(Event::Store(update));

                    leak_detected.store(level, Ordering::Relaxed);
                })
                .context("Set async leak interrupt")?;
        }

        // Rebrodcast state when sync is requested
        // Dont drop leak pin until program exit
        {
            let mut events = events.clone();
            let leak_detected = leak_detected;

            spawner.spawn(move || {
                let _leak_pin = leak_pin;

                for event in listener {
                    match &*event {
                        Event::SyncStore => {
                            let update = store::create_update(
                                &tokens::LEAK,
                                leak_detected.load(Ordering::Relaxed),
                            );
                            events.send(Event::Store(update));
                        }
                        Event::Exit => {
                            return;
                        }
                        _ => {}
                    }
                }
            });
        }

        Ok(())
    }
}
