use std::{
    thread::{self, Scope},
    time::{Duration, Instant},
};

use ahrs::{Ahrs, Madgwick};
use common::{
    error::LogErrorExt,
    store::{self, tokens},
    types::Orientation,
};
use crossbeam::channel::bounded;
use nalgebra::Vector3;
use tracing::{span, warn, Level};

use crate::{
    event::{Event, SensorFrame},
    events::EventHandle,
    systems::{stop, System},
};

/// Handles error events
pub struct OrientationSystem;

impl System for OrientationSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        let (tx, rx) = bounded(10);

        {
            let tx = tx.clone();
            spawner.spawn(move || {
                span!(Level::INFO, "Sensor watcher thread");

                for event in listner {
                    match &*event {
                        Event::SensorFrame(frame) => {
                            tx.send(OrientationEvent::SensorFrame(*frame))
                                .log_error("Send SensorFrame");
                        }
                        Event::Exit => {
                            tx.try_send(OrientationEvent::Exit).log_error("Send Exit");
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
                span!(Level::INFO, "Sensor tick thread");

                let interval = Duration::from_micros(1000);

                let mut deadline = Instant::now() + interval;

                while !stop::world_stopped() {
                    tx.try_send(OrientationEvent::Tick).log_error("Send tick");

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
                span!(Level::INFO, "Sensor fusion thread");

                let brodcast_divisor = 20;

                let mut tick_counter = 0;

                let mut imu_frame = None;
                let mut mag_frame = None;
                let mut orientation = Orientation::default();

                let mut madgwick_filter = Madgwick::new(0.001, 0.041);

                for event in rx {
                    match event {
                        OrientationEvent::SensorFrame(frame) => match frame {
                            SensorFrame::Imu(imu) => imu_frame = Some(imu),
                            SensorFrame::Mag(mag) => mag_frame = Some(mag),
                        },
                        OrientationEvent::Tick => {
                            if let Some((imu, mag)) = Option::zip(imu_frame, mag_frame) {
                                let gyro = Vector3::new(imu.gyro_x.0, imu.gyro_y.0, imu.gyro_z.0)
                                    * (std::f64::consts::PI / 180.0);
                                let accel =
                                    Vector3::new(imu.accel_x.0, imu.accel_y.0, imu.accel_z.0);
                                let mag = Vector3::new(mag.mag_x.0, mag.mag_y.0, mag.mag_y.0);

                                let rst = madgwick_filter.update_imu(&gyro, &accel);
                                // let rst = madgwick_filter.update(&gyro, &accel, &mag);

                                match rst {
                                    Ok(quat) => orientation = Orientation(quat.cast().into()),
                                    err => err.log_error("Update orientation"),
                                }
                            }

                            if tick_counter % brodcast_divisor == 0 {
                                let orientation_update =
                                    store::create_update(&tokens::ORIENTATION, orientation);
                                events.send(Event::Store(orientation_update));

                                if let Some(imu) = imu_frame {
                                    let imu_update =
                                        store::create_update(&tokens::RAW_INERTIAL, imu);
                                    events.send(Event::Store(imu_update));
                                }

                                if let Some(mag) = mag_frame {
                                    let mag_update =
                                        store::create_update(&tokens::RAW_MAGNETIC, mag);
                                    events.send(Event::Store(mag_update));
                                }
                            }

                            tick_counter += 1;
                        }
                        OrientationEvent::Exit => {
                            return;
                        }
                    }
                }
            });
        }

        Ok(())
    }
}

enum OrientationEvent {
    SensorFrame(SensorFrame),
    Tick,
    Exit,
}
