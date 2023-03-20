use common::types::{MotorId, Speed};
use std::fmt::Debug;
use std::time::Duration;

const DEFAULT_MOTOR: Motor = Motor {
    channel: 255,
    max_speed: Speed::new(0.5), // Full speed on all motors would blow fuse
    // Taken from basic esc spec
    reverse: Duration::from_micros(1100),
    forward: Duration::from_micros(1900),
    center: Duration::from_micros(1500),
};

pub const MOTOR_FLB: Motor = Motor {
    channel: 0,
    ..DEFAULT_MOTOR
};
pub const MOTOR_FLT: Motor = Motor {
    channel: 1,
    ..DEFAULT_MOTOR
};
pub const MOTOR_FRB: Motor = Motor {
    channel: 2,
    ..DEFAULT_MOTOR
};
pub const MOTOR_FRT: Motor = Motor {
    channel: 3,
    ..DEFAULT_MOTOR
};
pub const MOTOR_BLB: Motor = Motor {
    channel: 4,
    ..DEFAULT_MOTOR
};
pub const MOTOR_BLT: Motor = Motor {
    channel: 5,
    ..DEFAULT_MOTOR
};
pub const MOTOR_BRB: Motor = Motor {
    channel: 6,
    ..DEFAULT_MOTOR
};
pub const MOTOR_BRT: Motor = Motor {
    channel: 7,
    ..DEFAULT_MOTOR
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Motor {
    /// PWM signal channel
    channel: u8,

    /// Speed settings, can be negative to reverse direction
    max_speed: Speed,

    /// PWM info
    reverse: Duration,
    forward: Duration,
    center: Duration,
}

impl Motor {
    pub fn speed_to_pwm(&self, speed: Speed) -> Duration {
        let speed = speed.get() * self.max_speed.get();

        let upper = if speed >= 0.0 {
            self.forward.as_micros()
        } else {
            self.reverse.as_micros()
        };
        let lower = self.center.as_micros();

        let scaled_speed = speed.abs() * 1000.0;
        let pulse = (upper as i64 * scaled_speed as i64
            + lower as i64 * (1000 - scaled_speed as i64))
            / 1000;

        Duration::from_micros(pulse as u64)
    }

    pub fn channel(&self) -> u8 {
        self.channel
    }
}

impl From<MotorId> for Motor {
    #[rustfmt::skip]
    fn from(value: MotorId) -> Self {
        match value {
            MotorId::FrontLeftBottom =>   MOTOR_FLB,
            MotorId::FrontLeftTop =>      MOTOR_FLT,
            MotorId::FrontRightBottom =>  MOTOR_FRB,
            MotorId::FrontRightTop =>     MOTOR_FRT,
            MotorId::BackLeftBottom =>    MOTOR_BLB,
            MotorId::BaclLeftTop =>       MOTOR_BLT,
            MotorId::BackRightBottom =>   MOTOR_BRB,
            MotorId::RearRightTop =>      MOTOR_BRT,
        }
    }
}
