use common::types::{MotorId, Percent};
use std::fmt::Debug;
use std::time::Duration;

const DEFAULT_MOTOR_CW: Motor = Motor {
    channel: 255,
    max_value: Percent::new(0.4), // Full speed on all motors would blow fuse
    // Taken from basic esc spec
    reverse: Duration::from_micros(1100),
    forward: Duration::from_micros(1900),
    center: Duration::from_micros(1500),
};
const DEFAULT_MOTOR_CCW: Motor = Motor {
    channel: 255,
    max_value: Percent::new(-0.4), // Full speed on all motors would blow fuse
    // Taken from basic esc spec
    reverse: Duration::from_micros(1100),
    forward: Duration::from_micros(1900),
    center: Duration::from_micros(1500),
};

const DEFAULT_SERVO: Motor = Motor {
    channel: 255,
    max_value: Percent::new(1.0),
    // Taken from servo spec
    reverse: Duration::from_micros(1100),
    forward: Duration::from_micros(1900),
    center: Duration::from_micros(1500),
};

// ---------- Thrusters ----------
pub const MOTOR_FLB: Motor = Motor {
    channel: 0,
    ..DEFAULT_MOTOR_CCW
};
pub const MOTOR_FLT: Motor = Motor {
    channel: 4,
    ..DEFAULT_MOTOR_CW
};
pub const MOTOR_FRB: Motor = Motor {
    channel: 5,
    ..DEFAULT_MOTOR_CW
};
pub const MOTOR_FRT: Motor = Motor {
    channel: 3,
    ..DEFAULT_MOTOR_CCW
};
pub const MOTOR_BLB: Motor = Motor {
    channel: 7,
    ..DEFAULT_MOTOR_CW
};
pub const MOTOR_BLT: Motor = Motor {
    channel: 6,
    ..DEFAULT_MOTOR_CCW
};
pub const MOTOR_BRB: Motor = Motor {
    channel: 1,
    ..DEFAULT_MOTOR_CCW
};
pub const MOTOR_BRT: Motor = Motor {
    channel: 2,
    ..DEFAULT_MOTOR_CW
};

// ---------- Camera Servos ----------
pub const SERVO_CAM1: Motor = Motor {
    channel: 15,
    ..DEFAULT_SERVO
};
pub const SERVO_CAM2: Motor = Motor {
    channel: 14,
    ..DEFAULT_SERVO
};
pub const SERVO_CAM3: Motor = Motor {
    channel: 13,
    ..DEFAULT_SERVO
};
pub const SERVO_CAM4: Motor = Motor {
    channel: 12,
    ..DEFAULT_SERVO
};

// ---------- Auxiliary Servos ----------
pub const SERVO_AUX1: Motor = Motor {
    channel: 11,
    ..DEFAULT_SERVO
};
pub const SERVO_AUX2: Motor = Motor {
    channel: 10,
    ..DEFAULT_SERVO
};
pub const SERVO_AUX3: Motor = Motor {
    channel: 9,
    ..DEFAULT_SERVO
};
pub const SERVO_AUX4: Motor = Motor {
    channel: 8,
    ..DEFAULT_SERVO
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Motor {
    /// PWM signal channel
    channel: u8,

    /// Speed settings, can be negative to reverse direction
    max_value: Percent,

    /// PWM info
    reverse: Duration,
    forward: Duration,
    center: Duration,
}

impl Motor {
    #[must_use]
    pub fn value_to_pwm(&self, speed: Percent) -> Duration {
        let speed = speed.get() * self.max_value.get();

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

    #[must_use]
    pub const fn channel(&self) -> u8 {
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
            MotorId::BackLeftTop =>       MOTOR_BLT,
            MotorId::BackRightBottom =>   MOTOR_BRB,
            MotorId::BackRightTop =>      MOTOR_BRT,

            MotorId::Camera1 =>           SERVO_CAM1,
            MotorId::Camera2 =>           SERVO_CAM2,
            MotorId::Camera3 =>           SERVO_CAM3,
            MotorId::Camera4 =>           SERVO_CAM4,
            MotorId::Aux1 =>              SERVO_AUX1,
            MotorId::Aux2 =>              SERVO_AUX2,
            MotorId::Aux3 =>              SERVO_AUX3,
            MotorId::Aux4 =>              SERVO_AUX4,
        }
    }
}
