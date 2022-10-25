use std::fmt::Debug;
use std::time::Duration;
use anyhow::Context;
use rppal::gpio::{Gpio, OutputPin};
use tracing::trace;
use common::types::{MotorId, Speed};

// TODO Verify correctness
// TODO Simplify impl
// TODO Extract constants

const DEFAULT_MOTOR: MotorConfig = MotorConfig {
    signal_pin: 255,
    max_speed: Speed::new(0.5), // Full speed on all motors would blow fuse
    // Taken from basic esc spec
    reverse: Duration::from_micros(1100),
    forward: Duration::from_micros(1900),
    center: Duration::from_micros(1500),
    period: Duration::from_nanos(1_000_000_000 / 400), // 400Hz
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
pub struct Motor<PinType: Debug> {
    config: MotorConfig,
    pin: PinType,
    speed: Speed
}

impl Motor<OutputPin> {
    #[tracing::instrument]
    pub fn new(gpio: &Gpio, config: MotorConfig) -> anyhow::Result<Self> {
        trace!("Motor::new()");

        let mut pin = gpio.get(config.signal_pin).context("Get pin")?.into_output();
        pin.set_pwm(config.period, config.center).context("Set pwm")?;

        Ok(Motor {
            config,
            pin,
            speed: Speed::ZERO,
        })
    }
}

impl<P: PwmDevice> Motor<P> {
    #[tracing::instrument]
    pub fn set_speed(&mut self, speed: Speed) -> anyhow::Result<()> {
        trace!("Motor::set_speed()");
        self.speed = speed;

        let speed = speed.get() * self.config.max_speed.get();

        let upper = if speed >= 0.0 {
            self.config.forward.as_micros()
        } else {
            self.config.reverse.as_micros()
        };
        let lower = self.config.center.as_micros();

        let speed = speed.abs() * 100.0;
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

#[derive(Copy, Clone, PartialEq, Debug,)]
pub struct MotorConfig {
    /// PWM signal pin
    signal_pin: u8,
    
    /// Speed settings, can be negative to reverse direction
    max_speed: Speed,
    
    /// PWM info
    reverse: Duration,
    forward: Duration,
    center: Duration,
    period: Duration,
}

impl From<MotorId> for MotorConfig {
    fn from(value: MotorId) -> Self {
        match value {
            MotorId::UpF => MOTOR_F,
            MotorId::UpB => MOTOR_B,
            MotorId::UpL => MOTOR_L,
            MotorId::UpR => MOTOR_R,
            MotorId::FrontL => MOTOR_FL,
            MotorId::FrontR => MOTOR_FR,
            MotorId::RearL => MOTOR_BL,
            MotorId::RearR => MOTOR_BR,
        }
    }
}

pub trait PwmDevice: Debug {
    /// Send pulses of width `pulse_width` every `period` to this device
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
            pin: DummyPwm::default(),
            speed: Default::default()
        };

        motor.set_speed(Speed::MAX_VAL).unwrap();

        let Motor { pin: DummyPwm(period, pulse_width), .. } = motor;
        assert_eq!(period, Duration::from_nanos(1_000_000_000 / 400));
        assert_eq!(pulse_width, Duration::from_micros(1700));

        let mut motor = Motor {
            config: DEFAULT_MOTOR,
            pin: DummyPwm::default(),
            speed: Default::default()
        };

        motor.set_speed(Speed::MIN_VAL).unwrap();

        let Motor { pin: DummyPwm(period, pulse_width), .. } = motor;
        assert_eq!(period, Duration::from_nanos(1_000_000_000 / 400));
        assert_eq!(pulse_width, Duration::from_micros(1300));

        let mut motor = Motor {
            config: DEFAULT_MOTOR,
            pin: DummyPwm::default(),
            speed: Default::default()
        };

        motor.stop().unwrap();

        let Motor { pin: DummyPwm(period, pulse_width), .. } = motor;
        assert_eq!(period, Duration::from_nanos(1_000_000_000 / 400));
        assert_eq!(pulse_width, Duration::from_micros(1500));
    }
}
