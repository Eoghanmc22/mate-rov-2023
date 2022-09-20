use std::collections::HashMap;
use std::ops::{Add, Neg, Sub};
use bitflags::bitflags;
use glam::Quat;
use serde::{Serialize, Deserialize};

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct Filter: u8 {
        const ORIENTATION_UPDATES  = 0b00000001;
        const INERTIAL_FRAMES      = 0b00000010;
        const MAG_FRAMES           = 0b00000100;
        const DEPTH_FRAMES         = 0b00001000;
        const LOG_MESSAGES         = 0b00010000;
        const CAMERA_EVENTS        = 0b00100000;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Orientation(pub Quat);

/// +X: Right, +Y: Forwards, +Z: Up
/// +XR: Pitch Up, +YR: Roll Counterclockwise, +ZR: Yaw Clockwise (top view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movement {
    pub x: Speed,      // Right
    pub y: Speed,      // Forwards
    pub z: Speed,      // Up

    pub x_rot: Speed,  // Pitch Up
    pub y_rot: Speed,  // Roll Counterclockwise
    pub z_rot: Speed,  // Yaw Clockwise (top view)
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MotorId {
    UpF,
    UpB,
    UpL,
    UpR,
    FrontL,
    FrontR,
    RearL,
    RearR,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Role {
    Controller,
    Monitor,
}


// Raw Data Frames

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthFrame {
    pub depth: Meters,
    pub temperature: Celsius,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InertialFrame {
    pub gyro_x: Degrees,
    pub gyro_y: Degrees,
    pub gyro_z: Degrees,

    pub accel_x: GForce,
    pub accel_y: GForce,
    pub accel_z: GForce,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagFrame {
    pub mag_x: Gauss,
    pub mag_y: Gauss,
    pub mag_z: Gauss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorFrame(HashMap<MotorId, Speed>);


// Basic Units

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Meters(pub f64);

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Celsius(pub f64);

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct GForce(pub f64);

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Degrees(pub f64);

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Gauss(pub f64);

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct Speed(f64);

impl Speed {
    pub const MAX_VAL: Speed = Speed(1.0);
    pub const MIN_VAL: Speed = Speed(-1.0);
    pub const ZERO: Speed = Speed(0.0);

    /// Creates a new `Speed`. Input should be between -1.0 and 1.0
    pub fn new(speed: f64) -> Self {
        assert!(speed.is_normal());
        Self(speed).clamp(Self::MIN_VAL, Self::MAX_VAL)
    }

    /// Clamps a speed to be between `min` and `max`
    pub fn clamp(self, min: Speed, max: Speed) -> Speed {
        if self > max {
            max
        } else if self < min {
            min
        } else {
            self
        }
    }

    /// Get the speed as a float between -1.0 and 1.0
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Add<Speed> for Speed {
    type Output = Speed;

    fn add(self, rhs: Speed) -> Self::Output {
        Speed::new(self.0 + rhs.0)
    }
}

impl Sub<Speed> for Speed {
    type Output = Speed;

    fn sub(self, rhs: Speed) -> Self::Output {
        Speed::new(self.0 - rhs.0)
    }
}

impl Neg for Speed {
    type Output = Speed;

    fn neg(self) -> Self::Output {
        Speed(-self.0)
    }
}
