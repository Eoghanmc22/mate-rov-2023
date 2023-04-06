use std::{
    thread::{self, Scope},
    time::{Duration, Instant},
};

use common::store::{self, tokens};
use tracing::{span, Level};

use crate::{event::Event, events::EventHandle, peripheral::ms5937::Ms5837, systems::stop};

use super::System;

pub struct DepthSystem;

impl System for DepthSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let _ = events.take_listner();

        spawner.spawn(move || {
            span!(Level::INFO, "Depth sensor monitor thread");

            let depth = Ms5837::new(Ms5837::I2C_BUS, Ms5837::I2C_ADDRESS);
            let mut depth = match depth {
                Ok(depth) => depth,
                Err(err) => {
                    events.send(Event::Error(err.context("MS5837")));
                    return;
                }
            };

            let interval = Duration::from_secs_f64(1.0 / 100.0);

            let mut deadline = Instant::now();
            while !stop::world_stopped() {
                deadline += interval;

                let rst = depth.read_frame();

                match rst {
                    Ok(frame) => {
                        let update = store::create_update(&tokens::RAW_DEPTH, frame);
                        events.send(Event::Store(update));
                    }
                    Err(err) => {
                        events.send(Event::Error(err.context("Could not read depth")));
                    }
                }
                let remaining = deadline - Instant::now();
                thread::sleep(remaining);
            }
        });

        Ok(())
    }
}
