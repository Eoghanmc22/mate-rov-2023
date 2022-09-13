use std::fmt::Debug;
use std::time::Duration;
use anyhow::Context;
use rppal::gpio::{Gpio, OutputPin};
use serde::Serialize;
use serde::Deserialize;
use tracing::trace;

const DEFAULT_MOTOR: MotorConfig = MotorConfig {
    signal_pin: 255,
    min_speed: Speed::new(-50),
    max_speed: Speed::new(50),
    reverse: Duration::from_micros(1100),
    forward: Duration::from_micros(1900),
    center: Duration::from_micros(1500),
    period: Duration::from_nanos(1_000_000_000 / 400),
};

//TODO get the actual pins
pub const MOTOR_FL: MotorConfig = MotorConfig { signal_pin: 255, ..DEFAULT_MOTOR };
pub const MOTOR_FR: MotorConfig = MotorConfig { signal_pin: 255, ..DEFAULT_MOTOR };
pub const MOTOR_BL: MotorConfig = MotorConfig { signal_pin: 255, ..DEFAULT_MOTOR };
pub const MOTOR_BR: MotorConfig = MotorConfig { signal_pin: 255, ..DEFAULT_MOTOR };

pub const MOTOR_F: MotorConfig = MotorConfig { signal_pin: 255, ..DEFAULT_MOTOR };
pub const MOTOR_B: MotorConfig = MotorConfig { signal_pin: 255, ..DEFAULT_MOTOR };
pub const MOTOR_R: MotorConfig = MotorConfig { signal_pin: 255, ..DEFAULT_MOTOR };
pub const MOTOR_L: MotorConfig = MotorConfig { signal_pin: 255, ..DEFAULT_MOTOR };

#[derive(Debug)]
pub struct Motor<Pin: Debug> {
    config: MotorConfig,
    pin: Pin,
}

impl<P: PwmDevice> Motor<P> {
    #[tracing::instrument]
    pub fn new(gpio: &Gpio, config: MotorConfig) -> anyhow::Result<Motor<OutputPin>> {
        trace!("Motor::new()");

        let mut pin = gpio.get(config.signal_pin).context("Get pin")?.into_output();
        pin.set_pwm(config.period, config.center).context("Set pwm")?;

        Ok(Motor {
            config,
            pin
        })
    }

    #[tracing::instrument]
    pub fn set_speed(&mut self, speed: Speed) -> anyhow::Result<()> {
        trace!("Motor::set_speed()");

        let speed = speed.clamp(self.config.min_speed, self.config.max_speed);
        let speed = speed.get();

        let upper = if speed >= 0 {
            self.config.forward.as_micros()
        } else {
            self.config.reverse.as_micros()
        };
        let lower = self.config.center.as_micros();

        let speed = speed.abs();
        let pulse = (upper as i64 * speed as i64 + lower as i64 * (100 - speed as i64)) / 100;
        let pulse = Duration::from_micros(pulse as u64);

        self.pin.set_pwm(self.config.period, pulse).context("Set pwm")?;

        Ok(())
    }

    #[tracing::instrument]
    pub fn stop(&mut self) -> anyhow::Result<()> {
        trace!("Motor::stop()");
        self.set_speed(Speed::ZERO)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug,)]
pub struct MotorConfig {
    /// PWM signal pin
    signal_pin: u8,
    
    /// Speed settings
    min_speed: Speed,
    max_speed: Speed,
    
    /// PWM info
    reverse: Duration,
    forward: Duration,
    center: Duration,
    period: Duration,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default, Serialize, Deserialize)]
pub struct Speed(i8);

impl Speed {
    pub const MAX_VAL: Speed = Speed(100);
    pub const MIN_VAL: Speed = Speed(-100);
    pub const ZERO: Speed = Speed(0);

    pub const fn new(speed: i8) -> Self {
        Self(speed).clamp(Speed::MIN_VAL, Speed::MAX_VAL)
    }

    // This can be improved once PartialOrd becomes constant
    pub const fn clamp(self, min: Speed, max: Speed) -> Speed {
        if self.0 > max.0 {
            max
        } else if self.0 < min.0 {
            min
        } else {
            self
        }
    }

    pub const fn get(self) -> i8 {
        self.0
    }
}

pub trait PwmDevice: Debug {
    fn set_pwm(&mut self, period: Duration, pulse_width: Duration) -> anyhow::Result<()>;
}

impl PwmDevice for OutputPin {
    fn set_pwm(&mut self, period: Duration, pulse_width: Duration) -> anyhow::Result<()> {
        self.set_pwm(period, pulse_width)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motor_control() {
        #[derive(Default, Debug)]
        struct DummyPwm(Duration, Duration);
        impl PwmDevice for DummyPwm {
            fn set_pwm(&mut self, period: Duration, pulse_width: Duration) -> anyhow::Result<()> {
                self.0 = period;
                self.1 = pulse_width;

                Ok(())
            }
        }

        let mut motor = Motor {
            config: DEFAULT_MOTOR,
            pin: DummyPwm::default()
        };

        motor.set_speed(Speed::MAX_VAL).unwrap();

        let Motor { pin: DummyPwm(period, pulse_width), .. } = motor;
        assert_eq!(period, Duration::from_nanos(1_000_000_000 / 400));
        assert_eq!(pulse_width, Duration::from_micros(1700));

        let mut motor = Motor {
            config: DEFAULT_MOTOR,
            pin: DummyPwm::default()
        };

        motor.set_speed(Speed::MIN_VAL).unwrap();

        let Motor { pin: DummyPwm(period, pulse_width), .. } = motor;
        assert_eq!(period, Duration::from_nanos(1_000_000_000 / 400));
        assert_eq!(pulse_width, Duration::from_micros(1300));

        let mut motor = Motor {
            config: DEFAULT_MOTOR,
            pin: DummyPwm::default()
        };

        motor.stop().unwrap();

        let Motor { pin: DummyPwm(period, pulse_width), .. } = motor;
        assert_eq!(period, Duration::from_nanos(1_000_000_000 / 400));
        assert_eq!(pulse_width, Duration::from_micros(1500));
    }
}
