use std::{
    thread::{self, Scope},
    time::{Duration, Instant},
};

use common::store::{self, tokens};
use tracing::{span, Level};

use crate::{
    event::Event,
    events::EventHandle,
    peripheral::{icm20602::Icm20602, mmc5983::Mcc5983},
    systems::stop,
};

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

            let mag = Mcc5983::new(Mcc5983::SPI_BUS, Mcc5983::SPI_SELECT, Mcc5983::SPI_CLOCK);
            let mut mag = match mag {
                Ok(mag) => mag,
                Err(err) => {
                    events.send(Event::Error(err.context("MCC5983")));
                    return;
                }
            };

            let interval = Duration::from_secs_f64(1.0 / 1000.0);
            let imu_divisor = 1;
            let mag_divisor = 10;

            let mut deadline = Instant::now();
            let mut counter = 0;
            while !stop::world_stopped() {
                deadline += interval;

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

                if counter % mag_divisor == 0 {
                    let rst = mag.read_frame();

                    match rst {
                        Ok(frame) => {
                            let update = store::create_update(
                                &tokens::RAW_MAGNETIC,
                                (frame, Instant::now()),
                            );
                            events.send(Event::Store(update));
                        }
                        Err(err) => {
                            events.send(Event::Error(err.context("Could not read mag")));
                        }
                    }
                }

                let remaining = deadline - Instant::now();
                thread::sleep(remaining);

                counter += 1;
            }
        });

        Ok(())
    }
}
