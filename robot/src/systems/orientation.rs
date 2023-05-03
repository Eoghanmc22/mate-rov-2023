use std::thread::Scope;

use ahrs::{Ahrs, Madgwick};
use common::{
    error::LogErrorExt,
    store::{self, tokens},
    types::Orientation,
};
use nalgebra::Vector3;
use tracing::{span, Level};

use crate::{event::Event, events::EventHandle, systems::System, SystemId};

/// Handles error events
pub struct OrientationSystem;

impl System for OrientationSystem {
    const ID: SystemId = SystemId::Orientation;

    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listner = events.take_listner().unwrap();

        {
            spawner.spawn(move || {
                span!(Level::INFO, "Sensor fusion thread");

                let mut madgwick_filter = Madgwick::new(0.001, 0.041);

                for event in listner {
                    match &*event {
                        Event::SensorFrame(frame) => {
                            // We currently ignore mag updates as the compass is not calibrated
                            for inertial in frame.inertial {
                                let gyro = Vector3::new(
                                    inertial.gyro_x.0,
                                    inertial.gyro_y.0,
                                    inertial.gyro_z.0,
                                ) * (std::f64::consts::PI / 180.0);
                                let accel = Vector3::new(
                                    inertial.accel_x.0,
                                    inertial.accel_y.0,
                                    inertial.accel_z.0,
                                );

                                let rst = madgwick_filter.update_imu(&gyro, &accel);
                                if let Err(_) = rst {
                                    rst.log_error("Update orientation");
                                }
                            }

                            let orientation = Orientation(madgwick_filter.quat.cast().into());

                            {
                                let orientation_update =
                                    store::create_update(&tokens::ORIENTATION, orientation);
                                events.send(Event::Store(orientation_update));

                                let imu_update =
                                    store::create_update(&tokens::RAW_INERTIAL, frame.inertial[19]);
                                events.send(Event::Store(imu_update));

                                let mag_update =
                                    store::create_update(&tokens::RAW_MAGNETIC, frame.mag[1]);
                                events.send(Event::Store(mag_update));
                            }
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
