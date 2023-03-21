use std::{
    thread::{self, Scope},
    time::{Duration, Instant},
};

use common::store::{self, tokens};
use tracing::{error, span, Level};

use crate::{event::Event, events::EventHandle, peripheral_new::icm20602::Icm20602};

use super::System;

pub struct InertialSystem;

impl System for InertialSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let _ = events.take_listner();

        spawner.spawn(move || {
            span!(Level::INFO, "Inertial sensor monitor thread");

            let imu = Icm20602::new(Icm20602::SPI_BUS, Icm20602::SPI_SELECT, Icm20602::SPI_CLOCK);
            let mut imu = match imu {
                Ok(imu) => imu,
                Err(err) => {
                    events.send(Event::Error(err.context("ICM20602")));
                    return;
                }
            };

            let duration = Duration::from_secs_f64(1.0 / 1000.0);
            let imu_divisor = 1;

            let mut deadline = Instant::now();
            let mut counter = 0;
            loop {
                deadline += duration;

                if counter % imu_divisor == 0 {
                    let rst = imu.read_frame();

                    match rst {
                        Ok(frame) => {
                            let update = store::create_update(
                                &tokens::RAW_INERTIAL,
                                (frame, Instant::now()),
                            );
                            events.send(Event::Store(update));
                        }
                        Err(err) => {
                            events.send(Event::Error(err.context("Could not read imu")));
                        }
                    }
                }

                let sleep = deadline - Instant::now();
                if !sleep.is_zero() {
                    thread::sleep(sleep);
                } else {
                    error!("Did not meet deadline");
                }

                counter += 1;
            }
        });

        todo!()
    }
}
